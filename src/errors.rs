use std::num::ParseFloatError;
use std::ffi::NulError;

#[derive(Fail, Debug)]
pub enum ConfigError {
    #[fail(display = "value {} for key {} is not a number", key, value)]
    InvalidValue { key: String, value: String, #[cause] err: ParseFloatError },

    #[fail(display = "config key {} not recognized", _0)]
    UnrecognizedKey(String),
}

#[derive(Fail, Debug)]
pub enum ArrayError {
    #[fail(display = "Null encountered in string")]
    NullPresent(#[cause] NulError),

    #[fail(display = "Length of {} is too long", _0)]
    TooLong(usize)
}

impl From<NulError> for ArrayError {
    fn from(err: NulError) -> ArrayError {
        ArrayError::NullPresent(err)
    }
}

#[derive(Fail, Debug)]
pub enum SubmitError {
    #[fail(display = "plugin_dispatch_values returned an error: {}", _0)]
    DispatchError(i32)
}
