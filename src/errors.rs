use std::ffi::NulError;

#[derive(Fail, Debug)]
pub enum ArrayError {
    #[fail(display = "Null encountered in string")] NullPresent(#[cause] NulError),

    #[fail(display = "Length of {} is too long", _0)] TooLong(usize),
}

impl From<NulError> for ArrayError {
    fn from(err: NulError) -> ArrayError {
        ArrayError::NullPresent(err)
    }
}

#[derive(Fail, Debug)]
pub enum SubmitError {
    #[fail(display = "plugin_dispatch_values returned an error: {}", _0)] DispatchError(i32),
}

#[derive(Fail, Debug)]
#[fail(display = "Function is not implemented")]
pub struct NotImplemented;