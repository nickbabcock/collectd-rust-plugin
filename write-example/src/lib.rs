#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate chrono;
extern crate itertools;

use collectd_plugin::{collectd_log, LogLevel, Plugin, PluginCapabilities, RecvValueList};
use chrono::prelude::*;
use failure::Error;
use chrono::Duration;
use itertools::Itertools;

#[derive(Default)]
struct TestWritePlugin;

impl Plugin for TestWritePlugin {
    fn name(&self) -> &str {
        "testwriteplugin"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values<'a>(&mut self, list: RecvValueList<'a>) -> Result<(), Error> {
        let values = list.values.iter().map(|v| format!("{} - {}", v.name, v.value)).join(", ");

        let line = format!(
            "plugin_instance: {}, plugin: {}, type: {}, type_instance: {}, host: {}, time: {}, interval: {}, values: {}",
            list.plugin_instance.unwrap_or("<none>"),
            list.plugin,
            list.type_,
            list.type_instance.unwrap_or("<none>"),
            list.host.unwrap_or("<none>"),
            list.time.unwrap_or(Utc::now()),
            list.interval.unwrap_or_else(|| Duration::seconds(10)),
            line2,
        );
        collectd_log(
            LogLevel::Warning,
            &line
        );
        Ok(())
    }
}

collectd_plugin!(TestWritePlugin, Default::default);
