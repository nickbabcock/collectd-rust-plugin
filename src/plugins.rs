use failure::Error;
use errors::NotImplemented;
use api::{ConfigItem, LogLevel, RecvValueList};
use chrono::Duration;

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
    #[derive(Default)]
    pub struct PluginManagerCapabilities: u32 {
        const INIT = 0b0000_0001;
    }
}

pub enum PluginRegistration {
    Single(Box<Plugin>),
    Multiple(Vec<(String, Box<Plugin>)>),
}

impl PluginCapabilities {
    pub fn has_read(&self) -> bool {
        self.intersects(PluginCapabilities::READ)
    }

    pub fn has_log(&self) -> bool {
        self.intersects(PluginCapabilities::LOG)
    }

    pub fn has_write(&self) -> bool {
        self.intersects(PluginCapabilities::WRITE)
    }

    pub fn has_flush(&self) -> bool {
        self.intersects(PluginCapabilities::FLUSH)
    }
}

pub trait PluginManager {
    /// Name of the plugin.
    fn name() -> &'static str;

    fn capabilities() -> PluginManagerCapabilities {
        PluginManagerCapabilities::default()
    }

    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error>;

    fn initialize() -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

pub trait Plugin {
    /// A plugin's capabilities. By default a plugin does nothing, but can advertise that it can
    /// configure itself and / or report values.
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// Initialize any socket, files, or expensive resources that may have been parsed from the
    /// configuration. If an error is reported, all hooks registered will be unregistered.

    /// Customizes how a message of a given level is logged
    fn log(&mut self, _lvl: LogLevel, _msg: String) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// This function is called when collectd expects the plugin to report values, which will occur
    /// at the `Interval` defined in the global config (but can be overridden). Implementations
    /// that expect to report values need to have at least have a capability of `READ`. An error in
    /// reporting values will cause collectd to backoff exponentially until a delay of a day is
    /// reached.
    ///
    /// ## Warning
    ///
    /// It is up to you to make sure that this function is thread safe, so make sure anything that
    /// is being worked with implements `Sync`
    fn read_values(&mut self) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Collectd is giving you reported values, do with them as you please. If writing values is
    /// expensive, prefer to buffer them in some way and register a `flush` callback to write.
    ///
    /// ## Warning
    ///
    /// It is up to you to make sure that this function is thread safe, so make sure anything that
    /// is being worked with implements `Sync`
    fn write_values<'a>(&mut self, _list: RecvValueList<'a>) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }

    /// Flush values to be written that are older than given duration. If an identifier is given,
    /// then only those buffered values should be flushed.
    ///
    /// ## Warning
    ///
    /// It is up to you to make sure that this function is thread safe, so make sure anything that
    /// is being worked with implements `Sync`
    fn flush(
        &mut self,
        _timeout: Option<Duration>,
        _identifier: Option<&str>,
    ) -> Result<(), Error> {
        Err(Error::from(NotImplemented))
    }
}

#[macro_export]
macro_rules! collectd_plugin {
    ($type: ty) => {

        // Let's us know if we've seen our config section before
        static mut CONFIG_SEEN: bool = false;

        #[no_mangle]
        pub extern "C" fn module_register() {
            use std::ffi::CString;
            use $crate::bindings::{plugin_register_init, plugin_register_complex_config};

            let s = CString::new(<$type as PluginManager>::name())
                .expect("Plugin name to not contain nulls");

            unsafe {
                plugin_register_complex_config(
                    s.as_ptr(),
                    Some(collectd_plugin_complex_config)
                );

                plugin_register_init(s.as_ptr(), Some(collectd_plugin_init));
            }

        }

        unsafe extern "C" fn collectd_plugin_read(dt: *mut $crate::bindings::user_data_t) -> std::os::raw::c_int {
            let ptr: *mut Box<$crate::Plugin>  = std::mem::transmute((*dt).data);
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
            let ptr: *mut Box<$crate::Plugin> = std::mem::transmute(raw);
            Box::from_raw(ptr);
        }

        unsafe extern "C" fn collectd_plugin_log(
            severity: ::std::os::raw::c_int,
            message: *const std::os::raw::c_char,
            dt: *mut $crate::bindings::user_data_t
        ) {
            use std::ffi::CStr;
            let ptr: *mut Box<$crate::Plugin> = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);
            if let Ok(msg) = CStr::from_ptr(message).to_owned().into_string() {
                let lvl: $crate::LogLevel = std::mem::transmute(severity as u32);
                if let Err(ref e) = plugin.log(lvl, msg) {
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
            let ptr: *mut Box<$crate::Plugin> = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);
            let list = $crate::RecvValueList::from(&*ds, &*vl);
            if let Err(ref e) = list {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!("Unable to decode collectd data: {}", e)
                );
                std::mem::forget(plugin);
                return -1;
            }

            let result =
                if let Err(ref e) = plugin.write_values(list.unwrap()) {
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
            let mut result = if !CONFIG_SEEN {
                collectd_register_all_plugins(None)
            } else {
                0
            };

            let capabilities = <$type as PluginManager>::capabilities();
            if capabilities.intersects($crate::PluginManagerCapabilities::INIT) {
                if let Err(ref e) = <$type as PluginManager>::initialize() {
                    result = -1;
                    $crate::collectd_log(
                        $crate::LogLevel::Error,
                        &format!("init error: {}", e)
                    );
                }
            }

            result
        }

        unsafe extern "C" fn collectd_plugin_flush(
            timeout: $crate::bindings::cdtime_t,
            identifier: *const std::os::raw::c_char,
            dt: *mut $crate::bindings::user_data_t
        ) -> std::os::raw::c_int {
            use std::ffi::CStr;

            let ptr: *mut Box<$crate::Plugin> = std::mem::transmute((*dt).data);
            let mut plugin = Box::from_raw(ptr);

            let dur = if timeout == 0 { None } else { Some($crate::CdTime::from(timeout).into()) };
            let result =
                if let Ok(ident) = CStr::from_ptr(identifier).to_str() {
                    if let Err(ref e) = plugin.flush(dur, $crate::empty_to_none(ident)) {
                        $crate::collectd_log(
                            $crate::LogLevel::Error,
                            &format!("flush error: {}", e)
                        );
                        -1
                    } else {
                        0
                    }
                } else {
                    -1
                };

            std::mem::forget(plugin);
            result
        }

        unsafe extern "C" fn collectd_plugin_complex_config(
            config: *mut $crate::bindings::oconfig_item_t
        ) -> std::os::raw::c_int {
            // If we've already seen the config, let's error out as one shouldn't use multiple
            // sections of configuration (group them under nodes like write_graphite)
            if CONFIG_SEEN {
                $crate::collectd_log(
                    $crate::LogLevel::Error,
                    &format!("Already seen a config section for {}", <$type as PluginManager>::name())
                );
                return -1;
            }

            CONFIG_SEEN = true;
            let result =
                if let Ok(config) = $crate::ConfigItem::from(&*config) {
                    collectd_register_all_plugins(Some(&config.children))
                } else {
                    -1
                };

            result
        }

        fn collectd_register_all_plugins(
            config: Option<&[$crate::ConfigItem]>
        ) -> std::os::raw::c_int {
            match <$type as PluginManager>::plugins(config) {
                Ok(registration) => {
                    match registration {
                        $crate::PluginRegistration::Single(pl) => {
                            collectd_plugin_registration(
                                <$type as $crate::PluginManager>::name(),
                                pl
                            );
                        }
                        $crate::PluginRegistration::Multiple(v) => {
                            for (id, pl) in v {
                                let name = format!("{}/{}",
                                    <$type as $crate::PluginManager>::name(),
                                    id
                                );

                                collectd_plugin_registration(name.as_str(), pl);
                            }
                        }
                    }
                    0
                },
                Err(ref e) => {
                    $crate::collectd_log(
                        $crate::LogLevel::Error,
                        &format!("config error: {}", e)
                    );
                    -1
                }
            }
        }

        fn collectd_plugin_registration(name: &str, plugin: Box<$crate::Plugin>) {
            use std::os::raw::c_void;
            use std::ptr;
            use std::ffi::CString;
            use $crate::bindings::{plugin_register_write, plugin_register_complex_read, plugin_register_log, plugin_register_flush};

            let pl: Box<Box<$crate::Plugin>> = Box::new(plugin);

            // Grab all the properties we need until `into_raw` away
            let should_read = pl.capabilities().has_read();
            let should_log = pl.capabilities().has_log();
            let should_write = pl.capabilities().has_write();
            let should_flush = pl.capabilities().has_flush();

            let s = CString::new(name).expect("Plugin name to not contain nulls");
            unsafe {
                let plugin_ptr: *mut c_void = std::mem::transmute(Box::into_raw(pl));

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
