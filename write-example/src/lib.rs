#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate chrono;

use collectd_plugin::{collectd_log, LogLevel, Plugin, PluginCapabilities, DataSet, RecvValueList};
use chrono::prelude::*;
use failure::Error;
use chrono::Duration;

#[derive(Default)]
struct TestWritePlugin;

impl Plugin for TestWritePlugin {
    fn name(&self) -> &str {
        "testwriteplugin"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values<'a>(&mut self, set: DataSet<'a>, list: RecvValueList<'a>) -> Result<(), Error> {
        let line = format!(
            "dataset: {}, plugin_instance: {}, plugin: {}, type: {}, type_instance: {}, host: {}, time: {}, interval: {}",
            set.metric,
            list.plugin_instance.unwrap_or(""),
            list.plugin,
            list.type_,
            list.type_instance.unwrap_or(""),
            list.host.unwrap_or(""),
            list.time.unwrap_or(Utc::now()),
            list.interval.unwrap_or(Duration::seconds(10)),
        );
        collectd_log(
            LogLevel::Warning,
            &line
        );
        Ok(())
    }
}

collectd_plugin!(TestWritePlugin, Default::default);
