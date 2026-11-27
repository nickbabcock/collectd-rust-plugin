//! Plugins used by the end-to-end test to check that collectd never calls into a plugin after
//! the plugin has been dropped. One plugin registers every type of callback, as collectd removes
//! each type at a different point during shutdown. The other only registers a log callback.

use chrono::Duration;
use collectd_plugin::{
    collectd_log, collectd_plugin, ConfigItem, LogLevel, Plugin, PluginCapabilities, PluginManager,
    PluginRegistration, ValueList,
};
use std::cell::Cell;
use std::error;
use std::sync::atomic::{AtomicBool, Ordering};

// The checks only read these statics, never the plugin, so they work even if a plugin is called
// after it was freed
static ALL_DROPPED: AtomicBool = AtomicBool::new(false);
static LOG_ONLY_DROPPED: AtomicBool = AtomicBool::new(false);

thread_local! {
    // Logging from inside the log callback would call the log callback again
    static IN_LOG: Cell<bool> = const { Cell::new(false) };
}

fn check_alive(dropped: &AtomicBool, callback: &str) {
    if dropped.load(Ordering::SeqCst) {
        let msg = format!("lifecycle: {} called after drop", callback);
        collectd_log(LogLevel::Error, &msg);
    }
}

fn check_log_alive(dropped: &AtomicBool) {
    IN_LOG.with(|in_log| {
        if !in_log.replace(true) {
            check_alive(dropped, "log");
            in_log.set(false);
        }
    });
}

struct AllCallbacks;

impl Drop for AllCallbacks {
    fn drop(&mut self) {
        ALL_DROPPED.store(true, Ordering::SeqCst);
        collectd_log(LogLevel::Info, "lifecycle: dropped all");
    }
}

impl Plugin for AllCallbacks {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
            | PluginCapabilities::WRITE
            | PluginCapabilities::FLUSH
            | PluginCapabilities::LOG
    }

    fn read_values(&self) -> Result<(), Box<dyn error::Error>> {
        check_alive(&ALL_DROPPED, "read");
        Ok(())
    }

    fn write_values(&self, _list: ValueList<'_>) -> Result<(), Box<dyn error::Error>> {
        check_alive(&ALL_DROPPED, "write");
        Ok(())
    }

    fn flush(
        &self,
        _timeout: Option<Duration>,
        _identifier: Option<&str>,
    ) -> Result<(), Box<dyn error::Error>> {
        check_alive(&ALL_DROPPED, "flush");
        collectd_log(LogLevel::Info, "lifecycle: flush called");
        Ok(())
    }

    fn log(&self, _lvl: LogLevel, _msg: &str) -> Result<(), Box<dyn error::Error>> {
        check_log_alive(&ALL_DROPPED);
        Ok(())
    }
}

struct LogOnly;

impl Drop for LogOnly {
    fn drop(&mut self) {
        LOG_ONLY_DROPPED.store(true, Ordering::SeqCst);
        collectd_log(LogLevel::Info, "lifecycle: dropped log");
    }
}

impl Plugin for LogOnly {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::LOG
    }

    fn log(&self, _lvl: LogLevel, _msg: &str) -> Result<(), Box<dyn error::Error>> {
        check_log_alive(&LOG_ONLY_DROPPED);
        Ok(())
    }
}

struct Lifecycle;

impl PluginManager for Lifecycle {
    fn name() -> &'static str {
        "lifecycle"
    }

    fn plugins(
        _config: Option<&[ConfigItem<'_>]>,
    ) -> Result<PluginRegistration, Box<dyn error::Error>> {
        Ok(PluginRegistration::Multiple(vec![
            (String::from("all"), Box::new(AllCallbacks)),
            (String::from("log"), Box::new(LogOnly)),
        ]))
    }
}

collectd_plugin!(Lifecycle);
