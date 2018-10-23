#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate itertools;
#[macro_use]
extern crate log;

use collectd_plugin::{
    collectd_log, CollectdLoggerBuilder, ConfigItem, LogLevel, Plugin, PluginCapabilities,
    PluginManager, PluginRegistration, ValueList,
};
use failure::Error;
use itertools::Itertools;
use log::LevelFilter;

#[derive(Default)]
struct TestWritePlugin;

impl PluginManager for TestWritePlugin {
    fn name() -> &'static str {
        "testwriteplugin"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        // Register a logging hook so that any usage of the `log` crate will be forwarded to
        // collectd's logging facilities
        CollectdLoggerBuilder::new()
            .prefix_plugin::<Self>()
            .filter_level(LevelFilter::Info)
            .try_init()
            .expect("really the only thing that should create a logger");

        let line = format!("collectd logging configuration: {:?}", config);
        collectd_log(LogLevel::Info, &line);
        info!("rust logging configuration: {:?}", config);
        Ok(PluginRegistration::Single(Box::new(TestWritePlugin)))
    }
}

impl Plugin for TestWritePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values<'a>(&self, list: ValueList<'a>) -> Result<(), Error> {
        let values = list
            .values
            .iter()
            .map(|v| format!("{} - {}", v.name, v.value))
            .join(", ");

        info!(
            "plugin_instance: {}, plugin: {}, type: {}, type_instance: {}, host: {}, time: {}, interval: {} seconds, values: {}",
            list.plugin_instance.unwrap_or("<none>"),
            list.plugin,
            list.type_,
            list.type_instance.unwrap_or("<none>"),
            list.host,
            list.time,
            list.interval.num_seconds(),
            values,
        );

        collectd_log_raw!(LogLevel::Info, b"I'm a raw log with arguments: %d\0", 10);
        Ok(())
    }
}

collectd_plugin!(TestWritePlugin);
