use collectd_plugin::{
    collectd_plugin, CollectdLoggerBuilder, ConfigItem, Plugin, PluginCapabilities, PluginManager,
    PluginRegistration,
};

use log::LevelFilter;
use std::error;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct MyErrorPlugin {
    state: AtomicBool,
}

impl PluginManager for MyErrorPlugin {
    fn name() -> &'static str {
        "myerror"
    }

    fn plugins(
        _config: Option<&[ConfigItem<'_>]>,
    ) -> Result<PluginRegistration, Box<dyn error::Error>> {
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

    fn read_values(&self) -> Result<(), Box<dyn error::Error>> {
        if self.state.fetch_xor(true, Ordering::SeqCst) {
            panic!("Oh dear what is wrong!?")
        } else {
            Err(anyhow::anyhow!("bailing").into())
        }
    }
}

collectd_plugin!(MyErrorPlugin);
