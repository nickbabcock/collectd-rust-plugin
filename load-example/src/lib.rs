#[macro_use]
extern crate collectd_plugin;
#[macro_use]
extern crate failure;

use collectd_plugin::{collectd_log, ConfigItem, ConfigValue, LogLevel, Plugin, PluginCapabilities,
                      PluginManager, PluginRegistration, Value, ValueListBuilder};
use failure::Error;

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "config key {} not recognized", _0)] UnrecognizedKey(String),
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
        let mut short = 15.0;
        let mut mid = 10.0;
        let mut long = 12.0;
        let line = format!("{:?}", config);
        collectd_log(LogLevel::Info, &line);
        if let Some(fields) = config {
            for f in fields.iter() {
                if f.values.len() > 1 {
                    return Err(format_err!(
                        "{} does not support more than one entry",
                        f.key
                    ));
                }

                let value = &f.values[0];
                if let ConfigValue::Number(x) = *value {
                    match f.key {
                        "Short" => short = x,
                        "Mid" => mid = x,
                        "Long" => long = x,
                        y => return Err(format_err!("{} is not recognized", y)),
                    }
                } else {
                    return Err(format_err!("{} is not a number: {:?}", f.key, value));
                };
            }
        }

        let plugin = MyLoadPlugin {
            short: short,
            mid: mid,
            long: long,
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
