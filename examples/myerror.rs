#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate log;

use collectd_plugin::{
    CollectdLoggerBuilder, ConfigItem, Plugin, PluginCapabilities, PluginManager,
    PluginRegistration,
};
use log::LevelFilter;
use std::error;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct MyErrorManager;

#[derive(Default)]
struct MyErrorPlugin {
    state: AtomicBool,
}

impl PluginManager for MyErrorPlugin {
    fn name() -> &'static str {
        "myerror"
    }

    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Box<error::Error>> {
        CollectdLoggerBuilder::new()
            .prefix_plugin::<Self>()
            .filter_level(LevelFilter::Info)
            .try_init()
            .expect("really the only thing that should create a logger");

        Ok(PluginRegistration::Single(Box::new(
            MyErrorPlugin::default(),
        )))
    }
}

impl Plugin for MyErrorPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&self) -> Result<(), Box<error::Error>> {
        if self.state.fetch_xor(true, Ordering::Relaxed) {
            panic!("Oh dear what is wrong!?")
        } else {
            Err(failure::err_msg("bailing"))?
        }
    }
}

collectd_plugin!(MyErrorPlugin);
