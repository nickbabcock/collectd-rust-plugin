//! Module used exclusively to setup the `collectd_plugin!` macro. No public functions from here
//! should be used.
use crate::api::{
    empty_to_none, get_default_interval, log_err, CdTime, ConfigItem, LogLevel, ValueList,
};
use crate::bindings::{
    cdtime_t, data_set_t, oconfig_item_t, plugin_register_complex_read, plugin_register_flush,
    plugin_register_log, plugin_register_write, user_data_t, value_list_t,
};
use crate::errors::FfiError;
use crate::plugins::{Plugin, PluginManager, PluginManagerCapabilities, PluginRegistration};
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{self, catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

extern "C" fn plugin_read(dt: *mut user_data_t) -> c_int {
    let res = unsafe { with_plugin(dt, |plugin| catch_unwind(|| plugin.read_values())) };
    let res = res
        .ok_or(FfiError::Dropped)
        .and_then(|x| x.map_err(|_| FfiError::Panic))
        .and_then(|x| x.map_err(FfiError::Plugin));

    if let Err(ref e) = res {
        log_err("read", e);
    }

    res.map(|_| 0).unwrap_or(-1)
}

extern "C" fn plugin_log(severity: c_int, message: *const c_char, dt: *mut user_data_t) {
    // Guard against potential null messages even if they are not supposed to happen.
    if message.is_null() {
        return;
    }

    // Here we allow the potential allocation of a string that contains replacement
    // characters as it wouldn't be right if collectd-plugin stopped the logging of an
    // important message when a small portion of the message may be illegible.
    let msg = unsafe { CStr::from_ptr(message).to_string_lossy() };
    let res = LogLevel::try_from(severity as u32)
        .ok_or(FfiError::UnknownSeverity(severity))
        .and_then(|lvl| {
            let res = unsafe {
                with_plugin(dt, |plugin| {
                    catch_unwind(|| plugin.log(lvl, Deref::deref(&msg)))
                })
            };

            match res {
                Some(res) => res
                    .map_err(|_| FfiError::Panic)
                    .and_then(|x| x.map_err(FfiError::Plugin)),

                // The plugin was dropped at shutdown, but collectd keeps calling log
                // callbacks until it exits
                None => Ok(()),
            }
        });

    if let Err(ref e) = res {
        log_err("logging", e);
    }
}

extern "C" fn plugin_write(
    ds: *const data_set_t,
    vl: *const value_list_t,
    dt: *mut user_data_t,
) -> c_int {
    let res = unsafe { ValueList::from(&*ds, &*vl) }
        .map_err(|e| FfiError::Collectd(Box::new(e)))
        .and_then(|list| {
            unsafe { with_plugin(dt, |plugin| catch_unwind(|| plugin.write_values(list))) }
                .ok_or(FfiError::Dropped)
                .and_then(|x| x.map_err(|_| FfiError::Panic))
                .and_then(|x| x.map_err(FfiError::Plugin))
        });

    if let Err(ref e) = res {
        log_err("writing", e);
    }

    res.map(|_| 0).unwrap_or(-1)
}

extern "C" fn plugin_flush(
    timeout: cdtime_t,
    identifier: *const c_char,
    dt: *mut user_data_t,
) -> c_int {
    let dur = if timeout == 0 {
        None
    } else {
        Some(CdTime::from(timeout).into())
    };

    let ident = if identifier.is_null() {
        Ok(None)
    } else {
        unsafe { CStr::from_ptr(identifier) }
            .to_str()
            .map(empty_to_none)
            .map_err(|e| FfiError::Utf8("flush identifier", e))
    };

    let res = ident.and_then(|id| {
        unsafe { with_plugin(dt, |plugin| catch_unwind(|| plugin.flush(dur, id))) }
            .ok_or(FfiError::Dropped)
            .and_then(|x| x.map_err(|_| FfiError::Panic))
            .and_then(|x| x.map_err(FfiError::Plugin))
    });

    if let Err(ref e) = res {
        log_err("flush", e);
    }

    res.map(|_| 0).unwrap_or(-1)
}

/// Holds the plugins of a plugin manager.
///
/// Collectd callbacks only hold weak references to a plugin, and the plugins are dropped in the
/// plugin manager's shutdown callback. Collectd runs shutdown callbacks after its last read,
/// write, and flush calls, but before it removes any callbacks, so a plugin's `Drop` can still
/// log. If a callback from another thread is running at that moment, the plugin is instead dropped
/// on that thread once the callback returns. Log callbacks stop reaching the plugin after it is
/// dropped.
pub type PluginStore = Mutex<Vec<Arc<dyn Plugin>>>;

/// Calls `f` with the plugin of a callback, or returns `None` if the plugin has been dropped.
///
/// # Safety
///
/// `dt` must be the user data of a callback registered by `plugin_registration`.
unsafe fn with_plugin<R>(dt: *mut user_data_t, f: impl FnOnce(&dyn Plugin) -> R) -> Option<R> {
    let plugin = (*((*dt).data as *const Weak<dyn Plugin>)).upgrade()?;
    let res = f(&*plugin);

    // If the plugin manager shut down while `f` ran, this drops the plugin. Catch any panic
    // from its `Drop`, as a panic can't unwind into collectd.
    if catch_unwind(AssertUnwindSafe(|| drop(plugin))).is_err() {
        log_err("drop", &FfiError::Panic);
    }

    Some(res)
}

unsafe extern "C" fn plugin_free_user_data(raw: *mut c_void) {
    drop(Box::from_raw(raw as *mut Weak<dyn Plugin>));
}

/// Creates user data for a callback. Collectd owns it once it is passed to a registration
/// function, and frees it with `plugin_free_user_data`.
fn new_user_data(plugin: &Arc<dyn Plugin>) -> user_data_t {
    user_data_t {
        data: Box::into_raw(Box::new(Arc::downgrade(plugin))) as *mut c_void,
        free_func: Some(plugin_free_user_data),
    }
}

fn lock_plugins(plugins: &PluginStore) -> MutexGuard<'_, Vec<Arc<dyn Plugin>>> {
    plugins.lock().unwrap_or_else(|e| e.into_inner())
}

/// Registers the plugin's callbacks with collectd, logging any that fail
fn plugin_registration(plugins: &PluginStore, name: &str, plugin: Box<dyn Plugin>) {
    let plugin: Arc<dyn Plugin> = Arc::from(plugin);
    lock_plugins(plugins).push(Arc::clone(&plugin));

    let capabilities = match catch_unwind(|| plugin.capabilities()) {
        Ok(capabilities) => capabilities,
        Err(_) => {
            log_err("collectd config", &FfiError::Panic);
            return;
        }
    };

    let s = match CString::new(name) {
        Ok(s) => s,
        Err(_) => {
            log_err("collectd config", &FfiError::PluginName(name.to_string()));
            return;
        }
    };

    let check = |callback: &'static str, code: c_int| {
        if code != 0 {
            log_err(
                "collectd config",
                &FfiError::Register {
                    callback,
                    plugin: name.to_string(),
                    code,
                },
            );
        }
    };

    unsafe {
        if capabilities.has_read() {
            let data = new_user_data(&plugin);
            let code = plugin_register_complex_read(
                ptr::null(),
                s.as_ptr(),
                Some(plugin_read),
                get_default_interval(),
                &data,
            );
            check("read", code);
        }

        if capabilities.has_write() {
            let data = new_user_data(&plugin);
            let code = plugin_register_write(s.as_ptr(), Some(plugin_write), &data);
            check("write", code);
        }

        if capabilities.has_log() {
            let data = new_user_data(&plugin);
            let code = plugin_register_log(s.as_ptr(), Some(plugin_log), &data);
            check("log", code);
        }

        if capabilities.has_flush() {
            let data = new_user_data(&plugin);
            let code = plugin_register_flush(s.as_ptr(), Some(plugin_flush), &data);
            check("flush", code);
        }
    }
}

fn register_all_plugins<T: PluginManager>(
    plugins: &PluginStore,
    config: Option<&[ConfigItem<'_>]>,
) -> c_int {
    let res = catch_unwind(|| T::plugins(config))
        .map_err(|_| FfiError::Panic)
        .and_then(|reged| reged.map_err(FfiError::Plugin))
        .map(|registration| match registration {
            PluginRegistration::Single(pl) => plugin_registration(plugins, T::name(), pl),
            PluginRegistration::Multiple(v) => {
                for (id, pl) in v {
                    let name = format!("{}/{}", T::name(), id);

                    plugin_registration(plugins, name.as_str(), pl)
                }
            }
        });

    if let Err(ref e) = res {
        log_err("collectd config", e);
    }
    res.map(|_| 0).unwrap_or(-1)
}

pub fn plugin_init<T: PluginManager>(config_seen: &AtomicBool, plugins: &PluginStore) -> c_int {
    let mut result = if !config_seen.swap(true, Ordering::SeqCst) {
        register_all_plugins::<T>(plugins, None)
    } else {
        0
    };

    let capabilities = T::capabilities();
    if capabilities.intersects(PluginManagerCapabilities::INIT) {
        let res = catch_unwind(T::initialize)
            .map_err(|_e| FfiError::Panic)
            .and_then(|init| init.map_err(FfiError::Plugin));

        if let Err(ref e) = res {
            result = -1;
            log_err("init", e);
        }
    }

    result
}

pub fn plugin_shutdown<T: PluginManager>(plugins: &PluginStore) -> c_int {
    let mut result = 0;

    // Drop the plugins before the plugin manager shuts down, as they may use resources that the
    // plugin manager cleans up. Take them out first, so the lock isn't held while `Drop` runs.
    let plugins = std::mem::take(&mut *lock_plugins(plugins));
    for plugin in plugins {
        if catch_unwind(AssertUnwindSafe(|| drop(plugin))).is_err() {
            result = -1;
            log_err("shutdown", &FfiError::Panic);
        }
    }

    let capabilities = T::capabilities();
    if capabilities.intersects(PluginManagerCapabilities::INIT) {
        let res = catch_unwind(T::shutdown)
            .map_err(|_e| FfiError::Panic)
            .and_then(|r| r.map_err(FfiError::Plugin));

        if let Err(ref e) = res {
            result = -1;
            log_err("shutdown", e);
        }
    }

    result
}

/// # Safety
///
/// It is assumed that the config is not a null pointer
pub unsafe fn plugin_complex_config<T: PluginManager>(
    config_seen: &AtomicBool,
    plugins: &PluginStore,
    config: *mut oconfig_item_t,
) -> c_int {
    // If we've already seen the config, let's error out as one shouldn't use multiple
    // sections of configuration (group them under nodes like write_graphite)
    if config_seen.swap(true, Ordering::SeqCst) {
        log_err("config", &FfiError::MultipleConfig);
        return -1;
    }

    match ConfigItem::from(&*config) {
        Ok(config) => register_all_plugins::<T>(plugins, Some(&config.children)),
        Err(e) => {
            log_err(
                "collectd config conversion",
                &FfiError::Collectd(Box::new(e)),
            );
            -1
        }
    }
}

pub fn register_panic_handler() {
    panic::set_hook(Box::new(|info| {
        log_err("panic hook", &FfiError::PanicHook(info));
    }));
}
