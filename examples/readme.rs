use collectd_plugin::{
    ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration, Value,
    ValueListBuilder, collectd_plugin
};
use std::error;

#[derive(Default)]
struct MyPlugin;

// A manager decides the name of the family of plugins and also registers one or more plugins based
// on collectd's configuration files
impl PluginManager for MyPlugin {
    // A plugin needs a unique name to be referenced by collectd
    fn name() -> &'static str {
        "myplugin"
    }

    // Our plugin might have configuration section in collectd.conf, which will be passed here if
    // present. Our contrived plugin doesn't care about configuration so it returns only a single
    // plugin (itself).
    fn plugins(_config: Option<&[ConfigItem<'_>]>) -> Result<PluginRegistration, Box<dyn error::Error>> {
        Ok(PluginRegistration::Single(Box::new(MyPlugin)))
    }
}

impl Plugin for MyPlugin {
    // We define that our plugin will only be reporting / submitting values to writers
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&self) -> Result<(), Box<dyn error::Error>> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
        // of values that you submit at a time depends on types.db in collectd configurations
        let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(Self::name(), "load")
            .values(&values)
            .submit()?;

        Ok(())
    }
}

// We pass in our plugin manager type
collectd_plugin!(MyPlugin);
