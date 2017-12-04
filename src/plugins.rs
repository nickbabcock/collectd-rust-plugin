use failure::Error;
use errors::NotImplemented;
use api::{LogLevel, RecvValueList};

bitflags! {
    /// Bitflags of capabilities that a plugin advertises to collectd.
    #[derive(Default)]
    pub struct PluginCapabilities: u32 {
        const CONFIG = 0b0000_0001;
        const READ =   0b0000_0010;
        const INIT =   0b0000_0100;
        const LOG =    0b0000_1000;
        const WRITE =  0b0001_0000;
    }
}

impl PluginCapabilities {
    pub fn has_read(&self) -> bool {
        self.intersects(PluginCapabilities::READ)
    }

    pub fn has_config(&self) -> bool {
        self.intersects(PluginCapabilities::CONFIG)
    }

    pub fn has_init(&self) -> bool {
        self.intersects(PluginCapabilities::INIT)
    }

    pub fn has_log(&self) -> bool {
        self.intersects(PluginCapabilities::LOG)
    }

    pub fn has_write(&self) -> bool {
        self.intersects(PluginCapabilities::WRITE)
    }
}

pub trait Plugin {
    /// Name of the plugin.
    fn name(&self) -> &str;

    /// A plugin's capabilities. By default a plugin does nothing, but can advertise that it can
    /// configure itself and / or report values.
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// Configuration keys that the plugin configures itself with. Will only be consulted if the
    /// plugin has at least a capability of `CONFIG`.
    fn config_keys(&self) -> Vec<String> {
        vec![]
    }

    /// A key value pair related to the plugin that collectd parsed from the collectd configuration
    /// files. Will only be called if a plugin has a capability of at least `CONFIG`
    fn config_callback(&mut self, _key: String, _value: String) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Initialize any socket, files, or expensive resources that may have been parsed from the
    /// configuration. If an error is reported, all hooks registered will be unregistered.
    fn initialize(&mut self) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Customizes how a message of a given level is logged
    fn log(&mut self, _lvl: LogLevel, _msg: String) -> Result<(), Error> {
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

    fn write_values<'a>(&mut self, _list: RecvValueList<'a>) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

#[macro_export]
macro_rules! collectd_plugin {
    ($type: ty, $plugin: path) => {

        // This global variable is only for the functions of config / init where we know that the
        // function will be called only once from one thread. std::ptr::null_mut is not a const
        // function on stable, so we inline the function body
        static mut COLLECTD_PLUGIN_FOR_INIT: *mut $type = 0 as *mut $type;

        #[no_mangle]
        pub extern "C" fn module_register() {
            use std::os::raw::{c_char, c_void};
            use std::ffi::CString;
            use std::ptr;
            use $crate::bindings::{plugin_register_config, plugin_register_init, plugin_register_write, plugin_register_complex_read, plugin_register_log};

            let pl: Box<$type> = Box::new($plugin());

            // Grab all the properties we need until `into_raw` away
            let should_read = pl.capabilities().has_read();
            let should_config = pl.capabilities().has_config();
            let should_init = pl.capabilities().has_init();
            let should_log = pl.capabilities().has_log();
            let should_write = pl.capabilities().has_write();
            let ck: Vec<CString> = pl.config_keys()
                .into_iter()
                .map(|x| CString::new(x).expect("Config key to not contain nulls"))
                .collect();

            // Use expects for assertions -- no one really should be passing us strings that
            // contain nulls
            let s = CString::new(pl.name()).expect("Plugin name to not contain nulls");

            unsafe {
                COLLECTD_PLUGIN_FOR_INIT = Box::into_raw(pl);
                let dtptr: *mut c_void = std::mem::transmute(COLLECTD_PLUGIN_FOR_INIT);

                // The user data that is passed to read, writes, logs, etc. It is not passed to
                // config or init. Since user_data_t implements copy, we don't need to forget about
                // it. See clippy suggestion (forget_copy)
                let mut data = $crate::bindings::user_data_t {
                    data: dtptr,
                    free_func: Some(collectd_plugin_free_user_data),
                };

                if should_read {
                    plugin_register_complex_read(
                        ptr::null(),
                        s.as_ptr(),
                        Some(collectd_plugin_read),
                        $crate::get_default_interval(),
                        &mut data
                    );
                }

                if should_write {
                    plugin_register_write(
                        s.as_ptr(),
                        Some(collectd_plugin_write),
                        &mut data
                    );
                }

                if should_init {
                    plugin_register_init(s.as_ptr(), Some(collectd_plugin_init));
                }

                if should_config {
                    // Now grab all the pointers to the c strings for ffi
                    let mut pointers: Vec<*const c_char> = ck.iter()
                        .map(|arg| arg.as_ptr())
                        .collect();

                    plugin_register_config(
                        s.as_ptr(),
                        Some(collectd_plugin_config),
                        pointers.as_mut_ptr(),
                        pointers.len() as i32,
                    );

                    // We must forget the vector as collectd hangs on to the info and if we were to
                    // drop it, collectd would segfault trying to read the newly freed up data
                    // structure
                    std::mem::forget(ck);
                    std::mem::forget(pointers);
                }

                if should_log {
                    plugin_register_log(s.as_ptr(), Some(collectd_plugin_log), &mut data);
                }
            }
        }

        unsafe extern "C" fn collectd_plugin_read(dt: *mut $crate::bindings::user_data_t) -> std::os::raw::c_int {
            let ptr: *mut $type = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);
            let result = if let Err(ref e) = plugin.read_values() {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!("read error: {}", e)
                );
                -1
            } else {
                0
            };

            std::mem::forget(plugin);
            result
        }

        unsafe extern "C" fn collectd_plugin_free_user_data(raw: *mut ::std::os::raw::c_void) {
            let ptr: *mut $type = std::mem::transmute(raw);
            Box::from_raw(ptr);
        }

        unsafe extern "C" fn collectd_plugin_log(
            severity: ::std::os::raw::c_int,
            message: *const std::os::raw::c_char,
            dt: *mut $crate::bindings::user_data_t
        ) {
            use std::ffi::{CStr, CString};
            let ptr: *mut $type = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);
            if let Ok(msg) = CStr::from_ptr(message).to_owned().into_string() {
                let lvl: $crate::LogLevel = std::mem::transmute(severity as u32);

                // If there is an error with logging, in order to avoid recursive errors,
                // unregister our logger, but send the error message to the other loggers
                if let Err(ref e) = plugin.log(lvl, msg) {
                    let s = CString::new(plugin.name()).unwrap();
                    $crate::bindings::plugin_unregister_log(s.as_ptr());
                    $crate::collectd_log(
                        $crate::LogLevel::Error,
                        &format!("logging error: {}", e)
                    );
                }
            }
            std::mem::forget(plugin);
        }

        unsafe extern "C" fn collectd_plugin_write(
           ds: *const $crate::bindings::data_set_t,
           vl: *const $crate::bindings::value_list_t,
           dt: *mut $crate::bindings::user_data_t
        ) -> std::os::raw::c_int {
            let ptr: *mut $type = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);
            let list = $crate::RecvValueList::from(&*ds, &*vl);
            let result = 
                if let Err(ref e) = plugin.write_values(list) {
                    $crate::collectd_log(
                        $crate::LogLevel::Error,
                        &format!("writing error: {}", e)
                    );
                    -1
                } else {
                    0
                };
            std::mem::forget(plugin);
            result
        }

        unsafe extern "C" fn collectd_plugin_init() -> std::os::raw::c_int {
            let mut plugin = Box::from_raw(COLLECTD_PLUGIN_FOR_INIT);
            let result =
                if let Ok(()) = plugin.initialize() {
                    0
                } else {
                    -1
                };

            std::mem::forget(plugin);
            result
        }

        unsafe extern "C" fn collectd_plugin_config(
            key: *const std::os::raw::c_char,
            value: *const std::os::raw::c_char
        ) -> std::os::raw::c_int {
            use std::ffi::CStr;
            let mut plugin = Box::from_raw(COLLECTD_PLUGIN_FOR_INIT);
            let mut result = -1;

            if let Ok(key) = CStr::from_ptr(key).to_owned().into_string() {
                if let Ok(value) = CStr::from_ptr(value).to_owned().into_string() {
                    if let Err(ref e) = plugin.config_callback(key, value) {
                        $crate::collectd_log(
                            $crate::LogLevel::Error,
                            &format!("config error: {}", e)
                        );
                    } else {
                        result = 0;
                    }
                }
            }

            std::mem::forget(plugin);
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_capabilities() {
        let capabilities = PluginCapabilities::READ | PluginCapabilities::CONFIG;
        assert_eq!(capabilities.has_read(), true);
        assert_eq!(capabilities.has_config(), true);

        let capabilities = PluginCapabilities::READ;
        assert_eq!(capabilities.has_read(), true);
        assert_eq!(capabilities.has_config(), false);
    }
}
