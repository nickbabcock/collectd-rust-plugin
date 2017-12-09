#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use collectd_plugin::{ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration,
                      Value, ValueListBuilder};
use failure::Error;

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct MyConfig {
    short: Option<f64>,
    mid: Option<f64>,
    long: Option<f64>,
}

#[derive(Debug, PartialEq)]
struct MyLoadPlugin {
    short: f64,
    mid: f64,
    long: f64,
}

impl PluginManager for MyLoadPlugin {
    fn name() -> &'static str {
        "myplugin"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        let config: MyConfig =
            collectd_plugin::de::from_collectd(config.unwrap_or_else(Default::default))?;
        let plugin = MyLoadPlugin {
            short: config.short.unwrap_or(15.0),
            mid: config.mid.unwrap_or(10.0),
            long: config.long.unwrap_or(12.0),
        };
        Ok(PluginRegistration::Single(Box::new(plugin)))
    }
}

impl Plugin for MyLoadPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first followed by mid-term and long-term. The number of
        // values that you submit at a time depends on types.db in collectd configurations
        let values: Vec<Value> = vec![
            Value::Gauge(self.short),
            Value::Gauge(self.mid),
            Value::Gauge(self.long),
        ];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(Self::name(), "load")
            .values(values)
            .submit()
    }
}

collectd_plugin!(MyLoadPlugin);
