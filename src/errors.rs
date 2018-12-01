use failure::Error;
use std::error;

/// Errors that occur when converting Rust's text data to a format collectd expects
#[derive(Fail, Debug)]
pub enum ArrayError {
    /// Rust allows strings to contain a null character, but collectd does not.
    #[fail(display = "Null encountered (pos: _0) in string: _1")]
    NullPresent(usize, String),

    /// Is returned when a user tries to submit a field that contains text that is too long for
    /// collectd
    #[fail(display = "Length of {} is too long", _0)]
    TooLong(usize),
}

/// Errors that occur when submitting values to collectd
#[derive(Fail, Debug)]
pub enum SubmitError {
    /// Contains the exit status that collectd returns when a submission fails
    #[fail(display = "plugin_dispatch_values returned an error: {}", _0)]
    DispatchError(i32),
}

/// If a plugin advertises that it supports a certain functionality, but doesn't implement the
/// necessary `Plugin` function, this error is returned.
#[derive(Fail, Debug)]
#[fail(display = "Function is not implemented")]
pub struct NotImplemented;

/// Errors that occur when retrieving rates
#[derive(Fail, Debug)]
#[fail(display = "Unable to retrieve rate (see collectd logs for additional details)")]
pub struct CacheRateError;

/// Errors that occur on the boundary between collectd and a plugin
pub enum FfiError {
    /// Represents a plugin that panicked. A plugin that panics has a logic bug that should be
    /// fixed so that the plugin can better log and recover, else collectd decides
    Panic,

    /// An error from the plugin. This is a "normal" error that the plugin has caught. Like if the
    /// database is down and the plugin has the proper error mechanisms
    Plugin(Error),

    /// An error ocurred outside the path of a plugin
    Collectd(Box<error::Error>),
}
