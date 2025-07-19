use collectd_plugin::{
    collectd_plugin, CollectdLogger, CollectdLoggerBuilder, ConfigItem, Plugin, PluginCapabilities,
    PluginManager, PluginRegistration, ValueList,
};
use log::{debug, info};
use std::error;

#[derive(Default)]
struct MyPlugin;

impl PluginManager for MyPlugin {
    fn name() -> &'static str {
        "filtered_logging"
    }

    fn plugins(
        _config: Option<&[ConfigItem<'_>]>,
    ) -> Result<PluginRegistration, Box<dyn error::Error>> {
        // APPROACH: Use env_logger for filtering, forward to CollectdLogger
        //
        // Users can configure with: RUST_LOG=debug,my_module=trace,other_crate=warn

        let global_filter_level = log::LevelFilter::Info;
        let env_filter = env_logger::Builder::from_default_env()
            .filter_level(global_filter_level)
            .build();

        let collectd_logger = CollectdLoggerBuilder::new().prefix_plugin::<Self>().build();

        let filtered_logger = FilteredCollectdLogger {
            env_logger: env_filter,
            collectd_logger,
        };

        log::set_max_level(global_filter_level);
        log::set_boxed_logger(Box::new(filtered_logger))?;
        Ok(PluginRegistration::Single(Box::new(MyPlugin)))
    }
}

/// A logger that uses env_logger's filtering but outputs only to collectd
struct FilteredCollectdLogger {
    env_logger: env_logger::Logger,
    collectd_logger: CollectdLogger,
}

impl log::Log for FilteredCollectdLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        self.env_logger.enabled(metadata)
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            self.collectd_logger.log(record);
        }
    }

    fn flush(&self) {
        self.collectd_logger.flush();
    }
}

impl Plugin for MyPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::WRITE
    }

    fn write_values(&self, list: ValueList<'_>) -> Result<(), Box<dyn error::Error>> {
        // This will respect the RUST_LOG filtering
        debug!(
            "Processing {} values from {}",
            list.values.len(),
            list.plugin
        );
        info!("Received data from plugin: {}", list.plugin);
        Ok(())
    }
}

collectd_plugin!(MyPlugin);
