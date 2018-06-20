#![cfg(feature = "serde")]

#[macro_use]
extern crate collectd_plugin;
extern crate failure;
extern crate libc;
extern crate num_cpus;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use collectd_plugin::{
    ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration, Value,
    ValueListBuilder,
};
use failure::Error;

/// Our plugin will look for a ReportRelative True / False in the collectd config. Unknown
/// properties will cause a collectd failure as that means there is probably a typo.
#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
#[serde(deny_unknown_fields)]
struct LoadConfig {
    report_relative: Option<bool>,
}

/// Records load averages divided by the number of cpus
#[derive(Debug, PartialEq)]
struct RelativeLoadPlugin {
    num_cpus: f64,
}

/// Records load averages in terms of absolute values
#[derive(Debug, PartialEq)]
struct AbsoluteLoadPlugin;

/// One could implement `PluginManager` on `RelativeLoadPlugin` or `AbsoluteLoadPlugin`, but
/// demonstrate that that's not necessary, we create a separate unit struct.
struct LoadManager;

impl PluginManager for LoadManager {
    fn name() -> &'static str {
        "loadrust"
    }

    fn plugins(config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        // Deserialize the collectd configuration into our configuration struct
        let config: LoadConfig =
            collectd_plugin::de::from_collectd(config.unwrap_or_else(Default::default))?;

        // Grab the configuration. By default, this plugin reports absolute load values. For
        // demonstration purposes, there are two different plugin types (relative and absolute),
        // but one could easily fold the `num_cpus` field as an optional (or use `1` into a single
        // plugin struct)
        if config.report_relative.unwrap_or(false) {
            // Collectd's load plugin calculates number of CPUs on every report, but I'm not aware
            // of the number of CPUs dynamically changing, so we'll grab the value on start up and
            // keep it cached.
            let cpus = num_cpus::get();
            Ok(PluginRegistration::Single(Box::new(RelativeLoadPlugin {
                num_cpus: cpus as f64,
            })))
        } else {
            Ok(PluginRegistration::Single(Box::new(AbsoluteLoadPlugin)))
        }
    }
}

/// Returns load averages (short, mid, and long term). This implementation is not as cross platform
/// as collectd's, as getloadavg is not in POSIX, but getloadavg has been in glibc since 2000 and
/// found in BSD and Solaris, so I'd wager that this should cover 99.9% of use cases.
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
            .values(&values)
            .submit()
    }
}

impl Plugin for RelativeLoadPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        // Essentially the same as `AbsoluteLoadPlugin`, but divides each load value by the number
        // of cpus and submits the values as the type of "relative"
        let values: Vec<Value> = get_load()?
            .iter()
            .map(|&x| Value::Gauge(x / self.num_cpus))
            .collect();
        ValueListBuilder::new(LoadManager::name(), "load")
            .values(&values)
            .type_instance("relative")
            .submit()
    }
}

collectd_plugin!(LoadManager);
