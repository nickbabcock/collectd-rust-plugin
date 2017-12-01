use failure::Error;
use errors::NotImplemented;

bitflags! {
    /// Bitflags of capabilities that a plugin advertises to collectd.
    #[derive(Default)]
    pub struct PluginCapabilities: u32 {
        const CONFIG = 0b0000_0001;
        const READ =   0b0000_0010;
    }
}

impl PluginCapabilities {
    pub fn has_read(&self) -> bool {
        self.intersects(PluginCapabilities::READ)
    }

    pub fn has_config(&self) -> bool {
        self.intersects(PluginCapabilities::CONFIG)
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

    /// This function is called when collectd expects the plugin to report values, which will occur
    /// at the `Interval` defined in the global config (but can be overridden). Implementations
    /// that expect to report values need to have at least have a capability of `READ`. An error in
    /// reporting values will cause collectd to backoff exponentially until a delay of a day is
    /// reached.
    fn read_values(&mut self) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

#[macro_export]
macro_rules! collectd_plugin {
    ($plugin:ident) => {
        #[no_mangle]
        pub extern "C" fn module_register() {
            use std::os::raw::c_char;
            use std::ffi::CString;
            use $crate::bindings::{plugin_register_config, plugin_register_read};

            let pl = $plugin.lock().unwrap();

            // Use expects for assertions -- no one really should be passing us strings that
            // contain nulls
            let s = CString::new(pl.name()).expect("Plugin name to not contain nulls");

            unsafe {
                if pl.capabilities().has_read()  {
                    plugin_register_read(s.as_ptr(), Some(collectd_plugin_read));
                }

                if pl.capabilities().has_config() {
                    let ck: Vec<CString> = pl.config_keys()
                        .into_iter()
                        .map(|x| CString::new(x).expect("Config key to not contain nulls"))
                        .collect();

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
            }
        }

        unsafe extern fn collectd_plugin_read() -> std::os::raw::c_int {
            if let Err(ref e) = $plugin.lock().unwrap().read_values() {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!("read error: {}", e)
                );
                return -1;
            }
            0
        }

        unsafe extern fn collectd_plugin_config(
            key: *const std::os::raw::c_char,
            value: *const std::os::raw::c_char
        ) -> std::os::raw::c_int {
            use std::ffi::CStr;

            if let Ok(key) = CStr::from_ptr(key).to_owned().into_string() {
                if let Ok(value) = CStr::from_ptr(value).to_owned().into_string() {
                    if let Err(ref e) = $plugin.lock().unwrap().config_callback(key, value) {
                        $crate::collectd_log(
                            $crate::LogLevel::Error,
                            &format!("config error: {}", e)
                        );
                        return -1;
                    } else {
                        return 0;
                    }
                }
            }
            -1
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
