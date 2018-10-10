use bindings::{plugin_log, LOG_DEBUG, LOG_ERR, LOG_INFO, LOG_NOTICE, LOG_WARNING};
use env_logger::filter;
use log::{self, Level, LevelFilter, Metadata, Record, SetLoggerError};
use plugins::PluginManager;
use std::ffi::CString;

#[derive(Default)]
pub struct CollectdLoggerBuilder {
    filter: filter::Builder,
    plugin: Option<&'static str>,
}

impl CollectdLoggerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn try_init(&mut self) -> Result<(), SetLoggerError> {
        let logger = CollectdLogger {
            filter: self.filter.build(),
            plugin: self.plugin,
        };

        log::set_max_level(logger.filter());
        log::set_boxed_logger(Box::new(logger))
    }

    pub fn prefix_plugin<T: PluginManager>(&mut self) -> &mut Self {
        self.plugin = Some(T::name());
        self
    }

    pub fn filter_level(&mut self, level: LevelFilter) -> &mut Self {
        self.filter.filter_level(level);
        self
    }

    pub fn filter_module(&mut self, module: &str, level: LevelFilter) -> &mut Self {
        self.filter.filter_module(module, level);
        self
    }

    pub fn filter(&mut self, module: Option<&str>, level: LevelFilter) -> &mut Self {
        self.filter.filter(module, level);
        self
    }

    pub fn parse(&mut self, filters: &str) -> &mut Self {
        self.filter.parse(filters);
        self
    }
}

struct CollectdLogger {
    filter: filter::Filter,
    plugin: Option<&'static str>,
}

impl log::Log for CollectdLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.matches(record) {
            let lvl = LogLevel::from(record.level());
            let s = match self.plugin {
                Some(p) => format!("{}: {}", p, record.args()),
                None => format!("{}", record.args()),
            };
            collectd_log(lvl, &s);
        }
    }

    fn flush(&self) {}
}

impl CollectdLogger {
    /// Checks if this record matches the configured filter.
    pub fn matches(&self, record: &Record) -> bool {
        self.filter.matches(record)
    }

    /// Returns the maximum `LevelFilter` that this env logger instance is
    /// configured to output.
    pub fn filter(&self) -> LevelFilter {
        self.filter.filter()
    }
}

/// Sends message and log level to collectd. Collectd configuration determines if a level is logged
/// and where it is delivered. Messages that are too long are truncated (1024 was the max length as
/// of collectd-5.7).
///
/// # Panics
///
/// If a message containing a null character is given as a message this function will panic.
pub fn collectd_log(lvl: LogLevel, message: &str) {
    let cs = CString::new(message).expect("Collectd log to not contain nulls");
    unsafe {
        // Collectd will allocate another string behind the scenes before passing to plugins that
        // registered a log hook, so passing it a string slice is fine.
        plugin_log(lvl as i32, cs.as_ptr());
    }
}

/// A simple wrapper around the collectd's plugin_log, which in turn wraps `vsnprintf`.
///
/// ```ignore
/// collectd_log_raw!(LogLevel::Info, b"test %d\0", 10);
/// ```
///
/// Since this is a low level wrapper, prefer `collectd_log` mechanism. The only times you would
/// prefer `collectd_log_raw`:
///
/// - Collectd was not compiled with `COLLECTD_DEBUG` (chances are, your collectd is compiled with
/// debugging enabled) and you are logging a debug message.
/// - The performance price of string formatting in rust instead of C is too large (this shouldn't
/// be the case)
///
/// Undefined behavior can result from any of the following:
///
/// - If the format string is not null terminated
/// - If any string arguments are not null terminated. Use `CString::as_ptr()` to ensure null
/// termination
/// - Malformed format string or mismatched arguments
///
/// This expects an already null terminated byte string. In the future once the byte equivalent of
/// `concat!` is rfc accepted and implemented, this won't be a requirement. I also wish that we
/// could statically assert that the format string contained a null byte for the last character,
/// but alas, I think we've hit brick wall with the current Rust compiler.
///
/// For ergonomic reasons, the format string is converted to the appropriate C ffi type from a byte
/// string, but no other string arguments are afforded this luxury (so make sure you convert them).
#[macro_export]
macro_rules! collectd_log_raw {
    ($lvl:expr, $fmt:expr) => ({
        let level: $crate::LogLevel = $lvl;
        let level = level as i32;
        unsafe {
            $crate::bindings::plugin_log(level, ($fmt).as_ptr() as *const i8);
        }
    });
    ($lvl:expr, $fmt:expr, $($arg:expr),*) => ({
        let level: $crate::LogLevel = $lvl;
        let level = level as i32;
        unsafe {
            $crate::bindings::plugin_log(level, ($fmt).as_ptr() as *const i8, $($arg)*);
        }
    });
}

/// The available levels that collectd exposes to log messages.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum LogLevel {
    Error = LOG_ERR,
    Warning = LOG_WARNING,
    Notice = LOG_NOTICE,
    Info = LOG_INFO,
    Debug = LOG_DEBUG,
}

impl LogLevel {
    /// Attempts to convert a u32 representing a collectd logging level into a Rust enum
    pub fn try_from(s: u32) -> Option<LogLevel> {
        match s {
            LOG_ERR => Some(LogLevel::Error),
            LOG_WARNING => Some(LogLevel::Warning),
            LOG_NOTICE => Some(LogLevel::Notice),
            LOG_INFO => Some(LogLevel::Info),
            LOG_DEBUG => Some(LogLevel::Debug),
            _ => None,
        }
    }
}

impl From<Level> for LogLevel {
    fn from(lvl: Level) -> Self {
        match lvl {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warning,
            Level::Info => LogLevel::Info,
            Level::Debug | Level::Trace => LogLevel::Debug,
        }
    }
}
