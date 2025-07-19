use crate::bindings::{plugin_log, LOG_DEBUG, LOG_ERR, LOG_INFO, LOG_NOTICE, LOG_WARNING};
use crate::errors::FfiError;
use log::{error, log_enabled, Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::cell::Cell;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

/// A builder for configuring and installing a collectd logger.
/// 
/// It is recommended to instantiate the logger in `PluginManager::plugins`.
///
/// The use case of multiple rust plugins that instantiate a global logger is supported. Each
/// plugin will have its own copy of a global logger. Thus plugins won't interefere with others and
/// their logging.
pub struct CollectdLoggerBuilder {
    plugin: Option<&'static str>,
    filter_level: LevelFilter,
    format: Box<FormatFn>,
}

type FormatFn = dyn Fn(&mut dyn Write, &Record<'_>) -> io::Result<()> + Sync + Send;

impl CollectdLoggerBuilder {
    /// Creates a new CollectdLoggerBuilder that will forward log messages to collectd.
    ///
    /// By default, accepts all log levels and uses the default format.
    /// See [`CollectdLoggerBuilder::filter_level`] to filter and [`CollectdLoggerBuilder::format`] for custom formatting.
    pub fn new() -> Self {
        Self {
            plugin: None,
            filter_level: LevelFilter::Trace,
            format: Format::default().into_boxed_fn(),
        }
    }

    /// Sets a prefix using the plugin manager's name.
    pub fn prefix_plugin<T: crate::plugins::PluginManager>(mut self) -> Self {
        self.plugin = Some(T::name());
        self
    }

    /// Sets the log level filter - only messages at this level and above will be forwarded to collectd.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use collectd_plugin::CollectdLogger;
    /// use log::LevelFilter;
    ///
    /// CollectdLogger::new()
    ///     .prefix("myplugin")
    ///     .filter_level(LevelFilter::Info)
    ///     .try_init()?;
    ///
    /// log::debug!("This will be filtered out");
    /// log::info!("This will go to collectd");
    /// ```
    pub fn filter_level(mut self, level: LevelFilter) -> Self {
        self.filter_level = level;
        self
    }

    /// Sets the format function for formatting the log output.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use collectd_plugin::CollectdLoggerBuilder;
    /// use std::io::Write;
    ///
    /// CollectdLoggerBuilder::new()
    ///     .prefix("myplugin")
    ///     .format(|buf, record| {
    ///         write!(buf, "[{}] {}", record.level(), record.args())
    ///     })
    ///     .try_init()?;
    /// ```
    pub fn format<F>(mut self, format: F) -> Self
    where
        F: Fn(&mut dyn Write, &Record<'_>) -> io::Result<()> + Sync + Send + 'static,
    {
        self.format = Box::new(format);
        self
    }

    /// The returned logger implements the `Log` trait and can be installed
    /// manually or nested within another logger.
    pub fn build(self) -> CollectdLogger {
        CollectdLogger {
            plugin: self.plugin,
            filter_level: self.filter_level,
            format: self.format,
        }
    }

    /// Initializes this logger as the global logger.
    ///
    /// All log messages will go to collectd via `plugin_log`. This is the only
    /// appropriate destination for logs in collectd plugins.
    ///
    /// # Errors
    ///
    /// This function will fail if it is called more than once, or if another
    /// library has already initialized a global logger.
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        let logger = self.build();
        log::set_max_level(logger.filter_level);
        log::set_boxed_logger(Box::new(logger))
    }
}

#[derive(Default)]
struct Format {
    custom_format: Option<Box<FormatFn>>,
}

impl Format {
    fn into_boxed_fn(self) -> Box<FormatFn> {
        if let Some(fmt) = self.custom_format {
            fmt
        } else {
            Box::new(move |buf, record| {
                if let Some(path) = record.module_path() {
                    write!(buf, "{}: ", path)?;
                }

                write!(buf, "{}", record.args())
            })
        }
    }
}

/// The actual logger implementation that sends messages to collectd.
pub struct CollectdLogger {
    plugin: Option<&'static str>,
    filter_level: LevelFilter,
    format: Box<FormatFn>,
}

impl log::Log for CollectdLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.filter_level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Log records are written to a thread local storage before being submitted to
        // collectd. The buffers are cleared afterwards
        thread_local!(static LOG_BUF: Cell<Vec<u8>> = const { Cell::new(Vec::new()) });
        LOG_BUF.with(|cell| {
            // Replaces the cell's contents with the default value, which is an empty vector.
            // Should be very cheap to move in and out of
            let mut write_buffer = cell.take();
            if let Some(plugin) = self.plugin {
                // writing the formatting to the vec shouldn't fail unless we ran out of
                // memory, but in that case, we have a host of other problems.
                let _ = write!(write_buffer, "{}: ", plugin);
            }

            if (self.format)(&mut write_buffer, record).is_ok() {
                let lvl = LogLevel::from(record.level());

                // Force a trailing NUL so that we can use fast path
                write_buffer.push(b'\0');
                {
                    let cs = unsafe { CStr::from_bytes_with_nul_unchecked(&write_buffer[..]) };
                    unsafe { plugin_log(lvl as i32, cs.as_ptr()) };
                }
            }

            write_buffer.clear();
            cell.set(write_buffer);
        });
    }

    fn flush(&self) {}
}

/// Logs an error with a description and all the causes. If rust's logging mechanism has been
/// registered, it is the preferred mechanism. If the Rust logging is not configured (and
/// considering that an error message should be logged) we log it directly to collectd
pub fn log_err(desc: &str, err: &FfiError<'_>) {
    let mut msg = format!("{} error: {}", desc, err);

    // We join all the causes into a single string. Some thoughts
    //  - When an error occurs, one should expect there is some performance price to pay
    //    for additional, and much needed, context
    //  - While nearly all languages will display each cause on a separate line for a
    //    stacktrace, I'm not aware of any collectd plugin doing the same. So to keep
    //    convention, all causes are logged on the same line, semicolon delimited.
    let mut ie = err.source();
    while let Some(cause) = ie {
        let _ = write!(msg, "; {}", cause);
        ie = cause.source();
    }

    if log_enabled!(Level::Error) {
        error!("{}", msg);
    } else {
        collectd_log(LogLevel::Error, &msg);
    }
}

/// Sends message and log level to collectd. This bypasses any configuration setup via
/// the global logger, so collectd configuration soley determines if a level is logged
/// and where it is delivered. Messages that are too long are truncated (1024 was the max length as
/// of collectd-5.7).
///
/// In general, prefer using the `log` crate macros with `CollectdLogger`.
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
/// Since this is a low level wrapper, prefer using the `log` crate with `CollectdLogger`. The only times you would
/// prefer `collectd_log_raw`:
///
/// - Collectd was not compiled with `COLLECTD_DEBUG` (chances are, your collectd is compiled with
///   debugging enabled) and you are logging a debug message.
/// - The performance price of string formatting in rust instead of C is too large (this shouldn't
///   be the case)
///
/// Undefined behavior can result from any of the following:
///
/// - If the format string is not null terminated
/// - If any string arguments are not null terminated. Use `CString::as_ptr()` to ensure null
///   termination
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
            $crate::bindings::plugin_log(level, ($fmt).as_ptr() as *const ::std::os::raw::c_char);
        }
    });
    ($lvl:expr, $fmt:expr, $($arg:expr),*) => ({
        let level: $crate::LogLevel = $lvl;
        let level = level as i32;
        unsafe {
            $crate::bindings::plugin_log(level, ($fmt).as_ptr() as *const ::std::os::raw::c_char, $($arg)*);
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
