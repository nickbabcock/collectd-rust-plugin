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
