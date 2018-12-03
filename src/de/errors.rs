use serde::de;
use std::error;
use std::fmt::{self, Display};

#[derive(Clone, Debug)]
pub enum DeError {
    NoMoreValuesLeft,
    SerdeError(String),
    ExpectSingleValue,
    ExpectString,
    ExpectChar(String),
    ExpectBoolean,
    ExpectNumber,
    ExpectStruct,
    ExpectObject,
    DataTypeNotSupported,
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

impl error::Error for Error {
    fn description(&self) -> &str {
        "an with deserialization error"
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            DeError::NoMoreValuesLeft => write!(f, "no more values left, this should never happen"),
            DeError::SerdeError(ref s) => write!(f, "error from deserialization: {}", s),
            DeError::ExpectSingleValue => write!(f, "expecting values to contain a single entry"),
            DeError::ExpectString => write!(f, "expecting string"),
            DeError::ExpectChar(ref s) => {
                write!(f, "expecting string of length one, received `{}`", s)
            }
            DeError::ExpectBoolean => write!(f, "expecting boolean"),
            DeError::ExpectNumber => write!(f, "expecting number"),
            DeError::ExpectStruct => write!(f, "expecting struct"),
            DeError::ExpectObject => write!(f, "needs an object to deserialize a struct"),
            DeError::DataTypeNotSupported => {
                write!(f, "could not deserialize as datatype not supported")
            }
        }
    }
}
