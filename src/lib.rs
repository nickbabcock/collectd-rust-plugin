#[macro_use]
extern crate bitflags;
extern crate chrono;
#[macro_use]
extern crate failure;

pub mod bindings;
mod api;
mod errors;
#[macro_use]
mod plugins;

pub use api::{collectd_log, empty_to_none, from_array, get_default_interval, CdTime, ConfigItem,
              ConfigValue, LogLevel, RecvValueList, Value, ValueListBuilder};
pub use errors::{ArrayError, SubmitError};
pub use plugins::{Plugin, PluginCapabilities, PluginManager, PluginRegistration, PluginManagerCapabilities, IdPlugin};
use failure::Error;

#[cfg(test)]
#[allow(private_no_mangle_fns)]
#[allow(dead_code)]
mod tests {
    use super::*;

    struct MyPlugin;

    impl PluginManager for MyPlugin {
        fn name() -> &'static str {
            "myplugin"
        }

        fn plugins(_config: Option<&ConfigItem>) -> Result<PluginRegistration, Error> {
            Ok(PluginRegistration::Multiple(vec![]))
        }
    }

    collectd_plugin!(MyPlugin);

    #[test]
    fn can_generate_blank_plugin() {
        assert!(true);
    }
}
