#[macro_use]
extern crate collectd_plugin;
extern crate failure;

use collectd_plugin::{Plugin, PluginCapabilities, Value, ValueListBuilder};
use failure::Error;

struct MyPlugin;

impl MyPlugin {
    fn new() -> Self {
        MyPlugin
    }
}

impl Plugin for MyPlugin {
    type Config = ();
    fn name(&self) -> &str {
        "myplugin"
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
        // of values that you submit at a time depends on types.db in collectd configurations
        let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(self.name(), "load")
            .values(values)
            .submit()
    }
}

collectd_plugin!(MyPlugin, MyPlugin::new);
