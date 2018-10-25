#![cfg(feature = "serde")]

#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use collectd_plugin::{
    collectd_log, CollectdLoggerBuilder, ConfigItem, LogLevel, Plugin, PluginCapabilities,
    PluginManager, PluginRegistration, ValueList,
};
use failure::Error;
use itertools::Itertools;
use log::LevelFilter;

fn true_default() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct TestWritePlugin {
    #[serde(default = "true_default", rename = "StoreRates")]
    store_rates: bool,
}

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
        let plugin: TestWritePlugin =
            collectd_plugin::de::from_collectd(config.unwrap_or_else(Default::default))?;
        Ok(PluginRegistration::Single(Box::new(plugin)))
    }
}

impl Plugin for TestWritePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values(&self, list: ValueList) -> Result<(), Error> {
        let values = if self.store_rates {
            list.rates()
        } else {
            Ok(::std::borrow::Cow::Borrowed(&list.values))
        }?;

        let values = values
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
