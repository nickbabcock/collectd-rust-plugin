use api::{ConfigItem, LogLevel, ValueList};
use chrono::Duration;
use errors::NotImplemented;
use failure::Error;

bitflags! {
    /// Bitflags of capabilities that a plugin advertises to collectd.
    #[derive(Default)]
    pub struct PluginCapabilities: u32 {
        const READ =   0b0000_0001;
        const LOG =    0b0000_0010;
        const WRITE =  0b0000_0100;
        const FLUSH =  0b0000_1000;
    }
}

bitflags! {
    /// Bitflags of capabilities that a plugin manager advertises to collectd
    #[derive(Default)]
    pub struct PluginManagerCapabilities: u32 {
        const INIT = 0b0000_0001;
    }
}

/// How many instances of the plugin will be registered
pub enum PluginRegistration {
    /// Our module will only register a single plugin
    Single(Box<Plugin>),

    /// Our module registers several modules. The String in the tuple must be unique identifier
    Multiple(Vec<(String, Box<Plugin>)>),
}

impl PluginCapabilities {
    pub fn has_read(self) -> bool {
        self.intersects(PluginCapabilities::READ)
    }

    pub fn has_log(self) -> bool {
        self.intersects(PluginCapabilities::LOG)
    }

    pub fn has_write(self) -> bool {
        self.intersects(PluginCapabilities::WRITE)
    }

    pub fn has_flush(self) -> bool {
        self.intersects(PluginCapabilities::FLUSH)
    }
}

/// Defines the entry point for a collectd plugin. Based on collectd's configuration, a
/// `PluginManager` will register any number of plugins (or return an error)
pub trait PluginManager {
    /// Name of the plugin.
    fn name() -> &'static str;

    /// Defines the capabilities of the plugin manager.
    fn capabilities() -> PluginManagerCapabilities {
        PluginManagerCapabilities::default()
    }

    /// Returns one or many instances of a plugin that is configured from collectd's configuration
    /// file. If parameter is `None`, a configuration section for the plugin was not found, so
    /// default values should be used.
    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error>;

    /// Initialize any socket, files, or expensive resources that may have been parsed from the
    /// configuration. If an error is reported, all hooks registered will be unregistered. This is
    /// really only useful for `PluginRegistration::Single` modules who want global data.
    fn initialize() -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

/// An individual plugin that is capable of reporting values to collectd, receiving values from
/// other plugins, or logging messages. A plugin must implement `Sync` as collectd could be sending
/// values to be written or logged concurrently. The Rust compiler will now ensure that everything
/// not thread safe is wrapped in a Mutex (or another compatible datastructure)
pub trait Plugin: Sync {
    /// A plugin's capabilities. By default a plugin does nothing, but can advertise that it can
    /// configure itself and / or report values.
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// Customizes how a message of a given level is logged. If the message isn't valid UTF-8, an
    /// allocation is done to replace all invalid characters with the UTF-8 replacement character
    fn log(&mut self, _lvl: LogLevel, _msg: &str) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// This function is called when collectd expects the plugin to report values, which will occur
    /// at the `Interval` defined in the global config (but can be overridden). Implementations
    /// that expect to report values need to have at least have a capability of `READ`. An error in
    /// reporting values will cause collectd to backoff exponentially until a delay of a day is
    /// reached.
    fn read_values(&mut self) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Collectd is giving you reported values, do with them as you please. If writing values is
    /// expensive, prefer to buffer them in some way and register a `flush` callback to write.
    fn write_values<'a>(&mut self, _list: ValueList<'a>) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Flush values to be written that are older than given duration. If an identifier is given,
    /// then only those buffered values should be flushed.
    fn flush(
        &mut self,
        _timeout: Option<Duration>,
        _identifier: Option<&str>,
    ) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

/// Sets up all the ffi entry points that collectd expects when given a `PluginManager`.
#[macro_export]
macro_rules! collectd_plugin {
    ($type:ty) => {
        // Let's us know if we've seen our config section before
        static CONFIG_SEEN: ::std::sync::atomic::AtomicBool =
            ::std::sync::atomic::AtomicBool::new(false);

        // This is the main entry point that collectd looks for. Our plugin manager will register
        // callbacks for configuration related to our name. It also registers a callback for
        // initialization for when configuratio is absent or a single plugin wants to hold global
        // data
        #[no_mangle]
        pub extern "C" fn module_register() {
            use std::ffi::CString;
            use $crate::bindings::{plugin_register_complex_config, plugin_register_init};

            let s = CString::new(<$type as $crate::PluginManager>::name())
                .expect("Plugin name to not contain nulls");

            unsafe {
                plugin_register_complex_config(s.as_ptr(), Some(collectd_plugin_complex_config));

                plugin_register_init(s.as_ptr(), Some(collectd_plugin_init));
            }
        }

        // Logs an error with a description and all the causes
        fn collectd_log_err(desc: &str, err: &::failure::Error) {
            // We join all the causes into a single string. Some thoughts
            //  - This is not the most efficient way (that would belong to itertool crate), but
            //    collecting into a vector then joining is not terribly more expensive.
            //  - When an error occurs, one should expect there is some performance price to pay
            //    for additional, and much needed, context
            //  - Adding a new dependency to this library for a single function to save one line
            //    seems to be a heavy handed solution
            //  - While nearly all languages will display each cause on a separate line for a
            //    stacktrace, I'm not aware of any collectd plugin doing the same. So to keep
            //    convention, all causes are logged on the same line, semicolon delimited.
            let joined = err
                .iter_chain()
                .map(|x| format!("{}", x))
                .collect::<Vec<String>>()
                .join("; ");

            $crate::collectd_log(
                $crate::LogLevel::Error,
                &format!("{} error: {}", desc, joined),
            );
        }

        unsafe fn collectd_user_data(
            dt: *mut $crate::bindings::user_data_t,
        ) -> Box<Box<$crate::Plugin>> {
            Box::from_raw((*dt).data as *mut Box<$crate::Plugin>)
        }

        extern "C" fn collectd_plugin_read(
            dt: *mut $crate::bindings::user_data_t,
        ) -> ::std::os::raw::c_int {
            let mut plugin = unsafe { collectd_user_data(dt) };
            let result = if let Err(ref e) = plugin.read_values() {
                collectd_log_err("read", e);
                -1
            } else {
                0
            };

            ::std::mem::forget(plugin);
            result
        }

        unsafe extern "C" fn collectd_plugin_free_user_data(raw: *mut ::std::os::raw::c_void) {
            let ptr = raw as *mut Box<$crate::Plugin>;
            Box::from_raw(ptr);
        }

        extern "C" fn collectd_plugin_log(
            severity: ::std::os::raw::c_int,
            message: *const ::std::os::raw::c_char,
            dt: *mut $crate::bindings::user_data_t,
        ) {
            use std::ffi::CStr;
            let mut plugin = unsafe { collectd_user_data(dt) };
            let msg = unsafe { CStr::from_ptr(message).to_string_lossy() };
            let log_level = $crate::LogLevel::try_from(severity as u32);
            if let Some(lvl) = log_level {
                if let Err(ref e) = plugin.log(lvl, ::std::ops::Deref::deref(&msg)) {
                    collectd_log_err("logging", e);
                }
            } else {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!(
                        "Unrecognized severity log level: {} for {}",
                        severity,
                        <$type as $crate::PluginManager>::name()
                    ),
                );
            }

            ::std::mem::forget(plugin);
        }

        extern "C" fn collectd_plugin_write(
            ds: *const $crate::bindings::data_set_t,
            vl: *const $crate::bindings::value_list_t,
            dt: *mut $crate::bindings::user_data_t,
        ) -> ::std::os::raw::c_int {
            let mut plugin = unsafe { collectd_user_data(dt) };
            let list = unsafe { $crate::ValueList::from(&*ds, &*vl) };
            if let Err(ref e) = list {
                collectd_log_err("unable to decode collectd data", e);
                ::std::mem::forget(plugin);
                return -1;
            }

            let result = if let Err(ref e) = plugin.write_values(list.unwrap()) {
                collectd_log_err("writing", e);
                -1
            } else {
                0
            };
            ::std::mem::forget(plugin);
            result
        }

        extern "C" fn collectd_plugin_init() -> ::std::os::raw::c_int {
            let mut result = if !CONFIG_SEEN.swap(true, ::std::sync::atomic::Ordering::Relaxed) {
                collectd_register_all_plugins(None)
            } else {
                0
            };

            let capabilities = <$type as $crate::PluginManager>::capabilities();
            if capabilities.intersects($crate::PluginManagerCapabilities::INIT) {
                if let Err(ref e) = <$type as $crate::PluginManager>::initialize() {
                    result = -1;
                    collectd_log_err("init", e);
                }
            }

            result
        }

        extern "C" fn collectd_plugin_flush(
            timeout: $crate::bindings::cdtime_t,
            identifier: *const ::std::os::raw::c_char,
            dt: *mut $crate::bindings::user_data_t,
        ) -> ::std::os::raw::c_int {
            use std::ffi::CStr;
            let mut plugin = unsafe { collectd_user_data(dt) };

            let dur = if timeout == 0 {
                None
            } else {
                Some($crate::CdTime::from(timeout).into())
            };

            let result = if let Ok(ident) = unsafe { CStr::from_ptr(identifier) }.to_str() {
                if let Err(ref e) = plugin.flush(dur, $crate::empty_to_none(ident)) {
                    collectd_log_err("flush", e);
                    -1
                } else {
                    0
                }
            } else {
                -1
            };

            ::std::mem::forget(plugin);
            result
        }

        extern "C" fn collectd_plugin_complex_config(
            config: *mut $crate::bindings::oconfig_item_t,
        ) -> ::std::os::raw::c_int {
            // If we've already seen the config, let's error out as one shouldn't use multiple
            // sections of configuration (group them under nodes like write_graphite)
            if CONFIG_SEEN.swap(true, ::std::sync::atomic::Ordering::Relaxed) {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!(
                        "already seen a config section for {}",
                        <$type as $crate::PluginManager>::name()
                    ),
                );
                return -1;
            }

            match unsafe { $crate::ConfigItem::from(&*config) } {
                Ok(config) => collectd_register_all_plugins(Some(&config.children)),
                Err(ref e) => {
                    collectd_log_err("collectd config conversion", e);
                    -1
                }
            }
        }

        fn collectd_register_all_plugins(
            config: Option<&[$crate::ConfigItem]>,
        ) -> ::std::os::raw::c_int {
            match <$type as $crate::PluginManager>::plugins(config) {
                Ok(registration) => {
                    match registration {
                        $crate::PluginRegistration::Single(pl) => {
                            collectd_plugin_registration(
                                <$type as $crate::PluginManager>::name(),
                                pl,
                            );
                        }
                        $crate::PluginRegistration::Multiple(v) => {
                            for (id, pl) in v {
                                let name =
                                    format!("{}/{}", <$type as $crate::PluginManager>::name(), id);

                                collectd_plugin_registration(name.as_str(), pl);
                            }
                        }
                    }
                    0
                }
                Err(ref e) => {
                    collectd_log_err("collectd config", e);
                    -1
                }
            }
        }

        fn collectd_plugin_registration(name: &str, plugin: Box<$crate::Plugin>) {
            use std::ffi::CString;
            use std::os::raw::c_void;
            use std::ptr;
            use $crate::bindings::{
                plugin_register_complex_read, plugin_register_flush, plugin_register_log,
                plugin_register_write,
            };

            let pl: Box<Box<$crate::Plugin>> = Box::new(plugin);

            // Grab all the properties we need until `into_raw` away
            let should_read = pl.capabilities().has_read();
            let should_log = pl.capabilities().has_log();
            let should_write = pl.capabilities().has_write();
            let should_flush = pl.capabilities().has_flush();

            let s = CString::new(name).expect("Plugin name to not contain nulls");

            // Plugin registration differs only a tiny bit between collectd-57 and older
            // versions. The one difference is that user_data_t went from mutable to not
            // mutable. The code duplication is annoying, but it's better to have it
            // encapsulated in a single crate instead of many others.
            #[cfg_attr(feature = "cargo-clippy", allow(unnecessary_mut_passed))]
            unsafe {
                let plugin_ptr = Box::into_raw(pl) as *mut c_void;

                // The user data that is passed to read, writes, logs, etc. It is not passed to
                // config or init. Since user_data_t implements copy, we don't need to forget about
                // it. See clippy suggestion (forget_copy)
                let mut data = $crate::bindings::user_data_t {
                    data: plugin_ptr,
                    free_func: Some(collectd_plugin_free_user_data),
                };

                if should_read {
                    plugin_register_complex_read(
                        ptr::null(),
                        s.as_ptr(),
                        Some(collectd_plugin_read),
                        $crate::get_default_interval(),
                        &mut data,
                    );
                }

                if should_write {
                    plugin_register_write(s.as_ptr(), Some(collectd_plugin_write), &mut data);
                }

                if should_log {
                    plugin_register_log(s.as_ptr(), Some(collectd_plugin_log), &mut data);
                }

                if should_flush {
                    plugin_register_flush(s.as_ptr(), Some(collectd_plugin_flush), &mut data);
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_capabilities() {
        let capabilities = PluginCapabilities::READ | PluginCapabilities::WRITE;
        assert_eq!(capabilities.has_read(), true);
        assert_eq!(capabilities.has_write(), true);

        let capabilities = PluginCapabilities::READ;
        assert_eq!(capabilities.has_read(), true);
        assert_eq!(capabilities.has_write(), false);
    }
}
