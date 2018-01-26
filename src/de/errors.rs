use serde::de;
use std::fmt::{self, Display};

#[derive(Fail, Debug)]
pub enum DeError {
    #[fail(display = "No more values left, this should never happen")] NoMoreValuesLeft,
    #[fail(display = "Error from deserialization: {}", _0)] SerdeError(String),
    #[fail(display = "Expecting values to contain a single entry")] ExpectSingleValue,
    #[fail(display = "Expecting string")] ExpectString,
    #[fail(display = "Expecting string of length one, received `{}`", _0)] ExpectChar(String),
    #[fail(display = "Expecting boolean")] ExpectBoolean,
    #[fail(display = "Expecting number")] ExpectNumber,
    #[fail(display = "Expecting struct")] ExpectStruct,
    #[fail(display = "Needs an object to deserialize a struct")] ExpectObject,
    #[fail(display = "Could not deserialize as datatype not supported")] DataTypeNotSupported,
}

// Since the failure crate can't automatically implement serde::de::Error (see issue
// <https://github.com/withoutboats/failure/issues/108>) we define a thin wrapper around our actual
// error type.
#[derive(Debug)]
pub struct Error(pub DeError);

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(DeError::SerdeError(msg.to_string()))
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        "an with deserialization error"
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
