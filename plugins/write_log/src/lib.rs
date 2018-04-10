#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate itertools;

use collectd_plugin::{collectd_log, ConfigItem, LogLevel, Plugin, PluginCapabilities,
                      PluginManager, PluginRegistration, ValueList};
use failure::Error;
use itertools::Itertools;

#[derive(Default)]
struct TestWritePlugin;

impl PluginManager for TestWritePlugin {
    fn name() -> &'static str {
        "testwriteplugin"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        let line = format!("Received configuration of {:?}", config);
        collectd_log(LogLevel::Info, &line);
        Ok(PluginRegistration::Single(Box::new(TestWritePlugin)))
    }
}

impl Plugin for TestWritePlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values<'a>(&mut self, list: ValueList<'a>) -> Result<(), Error> {
        let values = list.values
            .iter()
            .map(|v| format!("{} - {}", v.name, v.value))
            .join(", ");

        let line = format!(
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

        collectd_log(LogLevel::Info, &line);
        collectd_log_raw!(LogLevel::Info, b"I'm a raw log with arguments: %d\0", 10);
        Ok(())
    }
}

collectd_plugin!(TestWritePlugin);
