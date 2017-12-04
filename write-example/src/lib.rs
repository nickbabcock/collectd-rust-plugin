#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate chrono;

use collectd_plugin::{collectd_log, LogLevel, Plugin, PluginCapabilities, RecvValueList};
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

    fn write_values<'a>(&mut self, list: RecvValueList<'a>) -> Result<(), Error> {
        let mut line2 = String::new();
        for v in list.values {
            line2 += &format!("{} - {:?}", v.name, v.value);
        }

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
