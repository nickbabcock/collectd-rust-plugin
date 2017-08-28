#![allow(dead_code)]

use bindings::{value_list_t, value_t, plugin_dispatch_values};
use std::os::raw::{c_char, c_int};
use std::ffi::{CString, NulError};
use std::fmt;
use std::error::Error;
use ptr;

#[derive(Debug)]
pub enum CArrayError {
    TooLong(usize),
    NullPresent(NulError),
}

impl fmt::Display for CArrayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CArrayError::TooLong(l) => {
                write!(f, "Length of {} is too long for collectd's max of 128", l)
            }
            CArrayError::NullPresent(ref err) => err.fmt(f),
        }
    }
}

impl Error for CArrayError {
    fn description(&self) -> &str {
        match *self {
            CArrayError::TooLong(_) => "String must be less than 128 bytes long",
            CArrayError::NullPresent(_) => "String must not contain a null character",
        }
    }
}

impl From<NulError> for CArrayError {
    fn from(e: NulError) -> Self {
        CArrayError::NullPresent(e)
    }
}

#[derive(Debug)]
pub enum Value {
    Counter(u64),
    Gauge(f64),
    Derive(i64),
    Absolute(u64),
}

impl Value {
    pub fn to_value_t(&self) -> value_t {
        match *self {
            Value::Counter(x) => value_t { counter: x },
            Value::Gauge(x) => value_t { gauge: x },
            Value::Derive(x) => value_t { derive: x },
            Value::Absolute(x) => value_t { absolute: x },
        }
    }
}

pub struct ValueListBuilder {
    values: Vec<Value>,
    plugin_instance: Option<String>,
    plugin: String,
    type_: String,
    type_instance: Option<String>,
    host: Option<String>,
    time: Option<u64>,
    interval: Option<u64>,
}

impl ValueListBuilder {
    pub fn new(plugin: &str, type_: &str) -> ValueListBuilder {
        ValueListBuilder {
            values: Vec::new(),
            plugin_instance: None,
            plugin: plugin.to_owned(),
            type_: type_.to_owned(),
            type_instance: None,
            host: None,
            time: None,
            interval: None,
        }
    }

    pub fn values(mut self, values: Vec<Value>) -> ValueListBuilder {
        self.values = values;
        self
    }

    pub fn plugin_instance(mut self, plugin_instance: String) -> ValueListBuilder {
        self.plugin_instance = Some(plugin_instance);
        self
    }

    pub fn type_instance(mut self, type_instance: String) -> ValueListBuilder {
        self.type_instance = Some(type_instance);
        self
    }

    pub fn host(mut self, host: String) -> ValueListBuilder {
        self.host = Some(host);
        self
    }

    pub fn time(mut self, time: u64) -> ValueListBuilder {
        self.time = Some(time);
        self
    }

    pub fn interval(mut self, interval: u64) -> ValueListBuilder {
        self.interval = Some(interval);
        self
    }

    pub fn submit(self) -> Result<c_int, CArrayError> {
        let mut v: Vec<value_t> = self.values.iter().map(|x| x.to_value_t()).collect();
        let plugin_instance = self.plugin_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; 128]))?;

        let type_instance = self.type_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; 128]))?;

        let host = self.host
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; 128]))?;


        let list = value_list_t {
            values: v.as_mut_ptr(),
            values_len: v.len(),
            plugin_instance: plugin_instance,
            plugin: to_array_res(&self.plugin)?,
            type_: to_array_res(&self.type_)?,
            type_instance: type_instance,
            host: host,
            time: self.time.unwrap_or(0),
            interval: self.interval.unwrap_or(0),
            meta: ptr::null_mut(),
        };

        Ok(unsafe { plugin_dispatch_values(&list) })
    }
}

fn to_array_res(s: &str) -> Result<[c_char; 128], CArrayError> {
    let value = CString::new(s)?;
    let data = value.as_bytes_with_nul();
    if data.len() > 128 {
        return Err(CArrayError::TooLong(s.len()));
    }

    let mut arr = [0; 128];
    for (i, &c) in data.into_iter().enumerate() {
        arr[i] = c as c_char;
    }
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use std::mem;
    use super::*;
    #[test]
    fn it_works2() {
        assert_eq!(8, mem::size_of::<value_t>());
    }
}
