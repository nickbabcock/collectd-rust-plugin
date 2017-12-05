#[macro_use]
extern crate collectd_plugin;
#[macro_use]
extern crate failure;

use std::num::ParseFloatError;
use collectd_plugin::{Plugin, PluginCapabilities, Value, ValueListBuilder};
use failure::Error;

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "value {} is not a number", value)]
    InvalidValue {
        value: String,
        #[cause] err: ParseFloatError,
    },

    #[fail(display = "config key {} not recognized", _0)] UnrecognizedKey(String),
}

#[derive(Debug, PartialEq)]
struct MyLoadPlugin {
    short: f64,
    mid: f64,
    long: f64,
}

impl MyLoadPlugin {
    fn new() -> Self {
        // By default we'll use contrived values for the load plugin unless they are overridden at
        // the config level
        MyLoadPlugin {
            short: 15.0,
            mid: 10.0,
            long: 12.0,
        }
    }
}

fn parse_number(value: &str) -> Result<f64, ConfigError> {
    value.parse::<f64>().map_err(|x| {
        ConfigError::InvalidValue {
            value: value.to_owned(),
            err: x,
        }
    })
}

impl Plugin for MyLoadPlugin {
    fn name(&self) -> &str {
        "myplugin"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ | PluginCapabilities::CONFIG
    }

    fn config_keys(&self) -> Vec<String> {
        vec!["Short".to_string(), "Mid".to_string(), "Long".to_string()]
    }

    fn config_callback(&mut self, key: String, value: String) -> Result<(), Error> {
        match key.as_str() {
            "Short" => {
                self.short = parse_number(&value)?;
                Ok(())
            }
            "Mid" => {
                self.mid = parse_number(&value)?;
                Ok(())
            }
            "Long" => {
                self.long = parse_number(&value)?;
                Ok(())
            }
            _ => Err(ConfigError::UnrecognizedKey(key.clone()).into()),
        }
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
        ValueListBuilder::new(self.name(), "load")
            .values(values)
            .submit()
    }
}

collectd_plugin!(MyLoadPlugin, MyLoadPlugin::new);
