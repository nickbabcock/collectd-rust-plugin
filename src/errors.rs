use std::error;
use std::fmt;
use std::str::Utf8Error;

#[derive(Debug, Clone)]
pub enum ConfigError {
    UnknownType(i32),
    StringDecode(Utf8Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        "error interpretting configuration values"
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ConfigError::StringDecode(ref e) => Some(e),
            ConfigError::UnknownType(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArrayError {
    NullPresent(usize, String),
    TooLong(usize),
}

impl fmt::Display for ArrayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct CollectdUtf8Error(pub &'static str, pub Utf8Error);

impl fmt::Display for CollectdUtf8Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "utf8 error for field: {}", self.0)
    }
}

impl error::Error for CollectdUtf8Error {
    fn description(&self) -> &str {
        "collectd sent invalid utf8"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug, Clone)]
pub enum ReceiveError {
    Utf8(String, &'static str, Utf8Error),
}

impl fmt::Display for ReceiveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn cause(&self) -> Option<&error::Error> {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn cause(&self) -> Option<&error::Error> {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "function is not implemented")
    }
}

impl error::Error for NotImplemented {
    fn description(&self) -> &str {
        "function is not implemented"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// Errors that occur when retrieving rates
#[derive(Clone, Debug)]
pub struct CacheRateError;

impl fmt::Display for CacheRateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// Errors that occur on the boundary between collectd and a plugin
#[derive(Debug)]
pub enum FfiError {
    /// Represents a plugin that panicked. A plugin that panics has a logic bug that should be
    /// fixed so that the plugin can better log and recover, else collectd decides
    Panic,

    /// An error from the plugin. This is a "normal" error that the plugin has caught. Like if the
    /// database is down and the plugin has the proper error mechanisms
    Plugin(Box<error::Error>),

    /// An error ocurred outside the path of a plugin
    Collectd(Box<error::Error>),

    UnknownSeverity(i32),

    MultipleConfig,
}

impl fmt::Display for FfiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FfiError::Collectd(_) => write!(f, "unexpected collectd behavior"),
            FfiError::UnknownSeverity(severity) => write!(f, "unrecognized severity level: {}", severity),
            FfiError::MultipleConfig => write!(f, "duplicate config section"),
            FfiError::Panic => write!(f, "plugin panicked"),
            FfiError::Plugin(_) => write!(f, "plugin errored out")
        }
    }
}

impl error::Error for FfiError {
    fn description(&self) -> &str {
        "collectd plugin error"
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            FfiError::Collectd(ref e) => Some(e.as_ref()),
            FfiError::Plugin(ref e) => Some(e.as_ref()),
            _ => None,
        }
    }
}
