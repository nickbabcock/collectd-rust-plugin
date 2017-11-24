#[macro_use]
extern crate collectd_plugin;
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;

use std::num::ParseFloatError;
use collectd_plugin::{Plugin, Value, ValueListBuilder, PluginCapabilities};
use failure::Error;
use std::sync::Mutex;

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "value {} is not a number", value)]
    InvalidValue {
        value: String,
        #[cause] err: ParseFloatError,
    },

    #[fail(display = "config key {} not recognized", _0)] UnrecognizedKey(String),
}

#[derive(Debug, PartialEq, Default)]
struct MyLoadPlugin {
    short: Option<f64>,
    mid: Option<f64>,
    long: Option<f64>,
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
        PluginCapabilities::READ & PluginCapabilities::CONFIG
    }

    fn config_keys(&self) -> Vec<String> {
        vec!["Short".to_string(), "Mid".to_string(), "Long".to_string()]
    }

    fn config_callback(&mut self, key: String, value: String) -> Result<(), Error> {
        match key.as_str() {
            "Short" => {
                self.short = Some(parse_number(&value)?);
                Ok(())
            }
            "Mid" => {
                self.mid = Some(parse_number(&value)?);
                Ok(())
            }
            "Long" => {
                self.long = Some(parse_number(&value)?);
                Ok(())
            }
            _ => Err(ConfigError::UnrecognizedKey(key.clone()).into()),
        }
    }

    fn report_values(&mut self) -> Result<(), Error> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first followed by mid-term and long-term. The number of
        // values that you submit at a time depends on types.db in collectd configurations
        let values: Vec<Value> = vec![
            Value::Gauge(self.short.unwrap_or(15.0)),
            Value::Gauge(self.mid.unwrap_or(10.0)),
            Value::Gauge(self.long.unwrap_or(12.0)),
        ];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new("myplugin", "load")
            .values(values)
            .submit()
    }
}

lazy_static! {
    static ref PLUGIN: Mutex<MyLoadPlugin> = Mutex::new(MyLoadPlugin::default());
}

collectd_plugin!(PLUGIN);
