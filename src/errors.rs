use std::error;
use std::fmt;
use std::panic::PanicInfo;
use std::str::Utf8Error;

/// Error that occurred while translating the collectd config to rust structures.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// The config type (eg: string, number, etc) denoted is unrecognized
    UnknownType(i32),

    /// The config string contains invalid UTF-8 characters
    StringDecode(Utf8Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConfigError::UnknownType(type_) => {
                write!(f, "unknown value ({}) for config enum", type_)
            }
            ConfigError::StringDecode(ref _e) => {
                write!(f, "unable to convert config string to utf8")
            }
        }
    }
}

impl error::Error for ConfigError {
    fn description(&self) -> &str {
        "error interpreting configuration values"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ConfigError::StringDecode(ref e) => Some(e),
            ConfigError::UnknownType(_) => None,
        }
    }
}

/// Error that occurred when converting a rust UTF-8 string to an array of `c_char` for collectd
/// ingestion.
#[derive(Debug, Clone)]
pub enum ArrayError {
    /// The UTF-8 string contained a null character
    NullPresent(usize, String),

    /// The UTF-8 string is too long to be ingested
    TooLong(usize),
}

impl fmt::Display for ArrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ArrayError::NullPresent(pos, ref s) => {
                write!(f, "null encountered (pos: {}) in string: {}", pos, s)
            }
            ArrayError::TooLong(len) => write!(f, "length of {} is too long", len),
        }
    }
}

impl error::Error for ArrayError {
    fn description(&self) -> &str {
        "error generating array"
    }
}

/// Error that occurred while receiving values from collectd to write
#[derive(Debug, Clone)]
pub enum ReceiveError {
    /// A plugin submitted a field that contained invalid UTF-8 characters
    Utf8(String, &'static str, Utf8Error),
}

impl fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ReceiveError::Utf8(ref plugin, ref field, ref _err) => {
                write!(f, "plugin: {} submitted bad field: {}", plugin, field)
            }
        }
    }
}

impl error::Error for ReceiveError {
    fn description(&self) -> &str {
        "error generating a value list"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ReceiveError::Utf8(ref _plugin, ref _field, ref err) => Some(err),
        }
    }
}

/// Errors that occur when submitting values to collectd
#[derive(Debug, Clone)]
pub enum SubmitError {
    /// Contains the exit status that collectd returns when a submission fails
    Dispatch(i32),

    Field(&'static str, ArrayError),
}

impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SubmitError::Dispatch(code) => {
                write!(f, "plugin_dispatch_values returned an error: {}", code)
            }
            SubmitError::Field(ref field, ref _err) => write!(f, "error submitting {}", field),
        }
    }
}

impl error::Error for SubmitError {
    fn description(&self) -> &str {
        "error generating array"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            SubmitError::Dispatch(_code) => None,
            SubmitError::Field(_field, ref err) => Some(err),
        }
    }
}

/// If a plugin advertises that it supports a certain functionality, but doesn't implement the
/// necessary `Plugin` function, this error is returned.
#[derive(Clone, Copy, Debug)]
pub struct NotImplemented;

impl fmt::Display for NotImplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "function is not implemented")
    }
}

impl error::Error for NotImplemented {
    fn description(&self) -> &str {
        "function is not implemented"
    }
}

/// Errors that occur when retrieving rates
#[derive(Clone, Debug)]
pub struct CacheRateError;

impl fmt::Display for CacheRateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unable to retrieve rate (see collectd logs for additional details)"
        )
    }
}

impl error::Error for CacheRateError {
    fn description(&self) -> &str {
        "unable to retrieve rate (see collectd logs for additional details)"
    }
}

/// Errors that occur on the boundary between collectd and a plugin
#[derive(Debug)]
pub enum FfiError<'a> {
    /// Error for implementing Rust's panic hook
    PanicHook(&'a PanicInfo<'a>),

    /// Represents a plugin that panicked. A plugin that panics has a logic bug that should be
    /// fixed so that the plugin can better log and recover, else collectd decides
    Panic,

    /// An error from the plugin. This is a "normal" error that the plugin has caught. Like if the
    /// database is down and the plugin has the proper error mechanisms
    Plugin(Box<dyn error::Error>),

    /// An error occurred outside the path of a plugin
    Collectd(Box<dyn error::Error>),

    /// When logging, collectd handed us a log level that was outside the known range
    UnknownSeverity(i32),

    /// Collectd gave us multiple configs to deserialize
    MultipleConfig,

    /// Collectd gave us field that contains invalid UTF-8 characters
    Utf8(&'static str, Utf8Error),
}

impl<'a> fmt::Display for FfiError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FfiError::Collectd(_) => write!(f, "unexpected collectd behavior"),
            FfiError::UnknownSeverity(severity) => {
                write!(f, "unrecognized severity level: {}", severity)
            }
            FfiError::MultipleConfig => write!(f, "duplicate config section"),
            FfiError::Panic => write!(f, "plugin panicked"),
            FfiError::PanicHook(info) => {
                write!(f, "plugin panicked: ")?;
                if let Some(location) = info.location() {
                    write!(f, "({}: {}): ", location.file(), location.line(),)?;
                }

                if let Some(payload) = info.payload().downcast_ref::<&str>() {
                    write!(f, "{}", payload)?;
                }

                Ok(())
            }
            FfiError::Plugin(_) => write!(f, "plugin encountered an error"),
            FfiError::Utf8(field, ref _e) => write!(f, "UTF-8 error for field: {}", field),
        }
    }
}

impl<'a> error::Error for FfiError<'a> {
    fn description(&self) -> &str {
        "collectd plugin error"
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FfiError::Collectd(ref e) => Some(e.as_ref()),
            FfiError::Plugin(ref e) => Some(e.as_ref()),
            FfiError::Utf8(_field, ref e) => Some(e),
            _ => None,
        }
    }
}
