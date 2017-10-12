#![allow(dead_code)]

use bindings::{hostname_g, plugin_dispatch_values, value_list_t, value_t, ARR_LENGTH};
use std::os::raw::c_char;
use std::ffi::CString;
use ptr;

pub mod errors {
    use std::ffi::NulError;
    use bindings::ARR_LENGTH;
    error_chain! {
        foreign_links {
            NullPresent(NulError);
        }

        errors {
            TooLong(t: usize) {
                description("String is too long")
                display("Length of {} is too long. Max: {}", t, ARR_LENGTH)
            }
            DispatchError(ret: i32) {
                description("plugin_dispatch_values returned an error status code")
                display("plugin_dispatch_values returned an error status code: {}", ret)
            }
        }
    }
}

use self::errors::*;

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn submit(self) -> Result<()> {
        let mut v: Vec<value_t> = self.values.iter().map(|x| x.to_value_t()).collect();
        let plugin_instance = self.plugin_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let type_instance = self.type_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let host = self.host
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| unsafe { Ok(hostname_g) })?;

        #[cfg(feature = "collectd-57")]
        let len = v.len();

        #[cfg(not(feature = "collectd-57"))]
        let len = v.len() as i32;

        let list = value_list_t {
            values: v.as_mut_ptr(),
            values_len: len,
            plugin_instance: plugin_instance,
            plugin: to_array_res(&self.plugin)?,
            type_: to_array_res(&self.type_)?,
            type_instance: type_instance,
            host: host,
            time: self.time.unwrap_or(0),
            interval: self.interval.unwrap_or(0),
            meta: ptr::null_mut(),
        };

        match unsafe { plugin_dispatch_values(&list) } {
            0 => Ok(()),
            i => Err(Error::from(ErrorKind::DispatchError(i))),
        }
    }
}

fn to_array_res(s: &str) -> Result<[c_char; ARR_LENGTH]> {
    let value = CString::new(s)?;
    let data = value.as_bytes_with_nul();
    if data.len() > ARR_LENGTH {
        return Err(ErrorKind::TooLong(s.len()).into());
    }

    let mut arr = [0; ARR_LENGTH];
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
