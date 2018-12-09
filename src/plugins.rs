use api::{ConfigItem, LogLevel, ValueList};
use chrono::Duration;
use errors::NotImplemented;
use std::error;
use std::panic::{RefUnwindSafe, UnwindSafe};

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
    /// Name of the plugin. Must not contain null characters or panic.
    fn name() -> &'static str;

    /// Defines the capabilities of the plugin manager. Must not panic.
    fn capabilities() -> PluginManagerCapabilities {
        PluginManagerCapabilities::default()
    }

    /// Returns one or many instances of a plugin that is configured from collectd's configuration
    /// file. If parameter is `None`, a configuration section for the plugin was not found, so
    /// default values should be used.
    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Box<error::Error>>;

    /// Initialize any socket, files, or expensive resources that may have been parsed from the
    /// configuration. If an error is reported, all hooks registered will be unregistered. This is
    /// really only useful for `PluginRegistration::Single` modules who want global data.
    fn initialize() -> Result<(), Box<error::Error>> {
        Err(NotImplemented)?
    }
}

/// An individual plugin that is capable of reporting values to collectd, receiving values from
/// other plugins, or logging messages. A plugin must implement `Sync + Send` as collectd could be sending
/// values to be written or logged concurrently. The Rust compiler will ensure that everything
/// not thread safe is wrapped in a Mutex (or another compatible datastructure)
pub trait Plugin: Send + Sync + UnwindSafe + RefUnwindSafe {
    /// A plugin's capabilities. By default a plugin does nothing, but can advertise that it can
    /// configure itself and / or report values.
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::default()
    }

    /// Customizes how a message of a given level is logged. If the message isn't valid UTF-8, an
    /// allocation is done to replace all invalid characters with the UTF-8 replacement character
    fn log(&self, _lvl: LogLevel, _msg: &str) -> Result<(), Box<error::Error>> {
        Err(NotImplemented)?
    }

    /// This function is called when collectd expects the plugin to report values, which will occur
    /// at the `Interval` defined in the global config (but can be overridden). Implementations
    /// that expect to report values need to have at least have a capability of `READ`. An error in
    /// reporting values will cause collectd to backoff exponentially until a delay of a day is
    /// reached.
    fn read_values(&self) -> Result<(), Box<error::Error>> {
        Err(NotImplemented)?
    }

    /// Collectd is giving you reported values, do with them as you please. If writing values is
    /// expensive, prefer to buffer them in some way and register a `flush` callback to write.
    fn write_values(&self, _list: ValueList) -> Result<(), Box<error::Error>> {
        Err(NotImplemented)?
    }

    /// Flush values to be written that are older than given duration. If an identifier is given,
    /// then only those buffered values should be flushed.
    fn flush(
        &self,
        _timeout: Option<Duration>,
        _identifier: Option<&str>,
    ) -> Result<(), Box<error::Error>> {
        Err(NotImplemented)?
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

        extern "C" fn collectd_plugin_init() -> ::std::os::raw::c_int {
            $crate::internal::plugin_init::<$type>(&CONFIG_SEEN)
        }

        unsafe extern "C" fn collectd_plugin_complex_config(
            config: *mut $crate::bindings::oconfig_item_t,
        ) -> ::std::os::raw::c_int {
            $crate::internal::plugin_complex_config::<$type>(&CONFIG_SEEN, config)
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
