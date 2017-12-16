#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate num_cpus;
extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use collectd_plugin::{ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration,
                      Value, ValueListBuilder};
use failure::Error;

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
#[serde(deny_unknown_fields)]
struct LoadConfig {
    report_relative: Option<bool>
}

#[derive(Debug, PartialEq)]
struct RelativeLoadPlugin {
    num_cpus: f64,
}

#[derive(Debug, PartialEq)]
struct AbsoluteLoadPlugin;

struct LoadManager;

impl PluginManager for LoadManager {
    fn name() -> &'static str {
        "load-rust"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        let config: LoadConfig =
            collectd_plugin::de::from_collectd(config.unwrap_or_else(Default::default))?;

        if config.report_relative.unwrap_or(false) {
            let cpus = num_cpus::get();
            Ok(PluginRegistration::Single(Box::new(RelativeLoadPlugin { num_cpus: cpus as f64 })))
        } else {
            Ok(PluginRegistration::Single(Box::new(AbsoluteLoadPlugin)))
        }
    }
}

fn get_load() -> Result<[f64; 3], Error> {
    let mut load: [f64; 3] = [0.0; 3];

    unsafe {
        if libc::getloadavg(load.as_mut_ptr(), 3) != 3 {
            Err(failure::err_msg("load: getloadavg failed"))
        } else {
            Ok(load)
        }
    }
}

impl Plugin for AbsoluteLoadPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first followed by mid-term and long-term. The number of
        // values that you submit at a time depends on types.db in collectd configurations
        let values: Vec<Value> = get_load()?.iter().map(|&x| Value::Gauge(x)).collect();

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(LoadManager::name(), "load")
            .values(values)
            .submit()
    }
}

impl Plugin for RelativeLoadPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        let values: Vec<Value> = get_load()?.iter().map(|&x| Value::Gauge(x / self.num_cpus)).collect();
        ValueListBuilder::new(LoadManager::name(), "load")
            .values(values)
            .type_instance("relative")
            .submit()
    }
}

collectd_plugin!(LoadManager);
