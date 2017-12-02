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

pub use api::{collectd_log, LogLevel, Value, ValueListBuilder};
pub use errors::{ArrayError, SubmitError};
pub use plugins::{Plugin, PluginCapabilities, PluginConfig};

#[cfg(test)]
#[allow(private_no_mangle_fns)]
#[allow(dead_code)]
mod tests {
    use super::*;

    struct MyPlugin;

    impl MyPlugin {
        fn new() -> Self {
            MyPlugin
        }
    }

    impl Plugin for MyPlugin {
        type Config = ();
        fn name(&self) -> &str {
            "myplugin"
        }
    }

    collectd_plugin!(MyPlugin, MyPlugin::new);

    #[test]
    fn can_generate_blank_plugin() {
        assert!(true);
    }
}
