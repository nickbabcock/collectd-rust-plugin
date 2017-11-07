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

    /// A COUNTER value is for continuous incrementing counters like the ifInOctets counter in a router.
    /// The COUNTER data source assumes that the observed value never decreases, except when it
    /// overflows. The update function takes the overflow into account. If a counter is reset to
    /// zero, for example because an application was restarted, the wrap-around calculation may
    /// result in a huge rate. Thus setting a reasonable maximum value is essential when using
    /// COUNTER data sources. Because of this, COUNTER data sources are only recommended for
    /// counters that wrap-around often, for example 32 bit octet counters of a busy switch port.
    Counter(u64),

    /// A GAUGE value is simply stored as-is. This is the right choice for values which may
    /// increase as well as decrease, such as temperatures or the amount of memory used
    Gauge(f64),

    /// DERIVE will store the derivative of the observed values source. If the data type has a
    /// minimum of zero, negative rates will be discarded. Using DERIVE is a good idea for
    /// measuring cgroup's cpuacct.usage as that stores the total number of CPU nanoseconds by all
    /// tasks in the cgroup; the change (derivative) in CPU nanoseconds is more interesting than
    /// the current value.
    Derive(i64),

    /// ABSOLUTE is for counters which get reset upon reading. This is used for fast counters which
    /// tend to overflow. So instead of reading them normally you reset them after every read to
    /// make sure you have a maximum time available before the next overflow.
    Absolute(u64),
}

// Interestingly, I couldn't get `From<Value> for value_t` to work, as any attempts would reference
// value_t's typedef of value_u.
impl Into<value_t> for Value {
    fn into(self) -> value_t {
        match self {
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
        let mut v: Vec<value_t> = self.values.into_iter().map(|x| x.into()).collect();
        let plugin_instance = self.plugin_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let type_instance = self.type_instance
            .map(|x| to_array_res(&x))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        // In collectd 5.7, it is no longer required to supply hostname_g for default hostname,
        // an empty array will get replaced with the hostname. However, since we're collectd 5.5
        // compatible, we use hostname_g in both circumstances, as it is not harmful
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

/// Collectd stores textual data in fixed sized arrays, so this function will convert a string
/// slice into array compatible with collectd's text fields. Be aware that `ARR_LENGTH` is 64
/// before collectd 5.7
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
    use super::*;
    use std::os::raw::c_char;

    #[test]
    fn test_to_array() {
        let actual = to_array_res("Hi");
        assert!(actual.is_ok());
        assert_eq!(&actual.unwrap()[..2], &[b'H' as c_char, b'i' as c_char]);
    }

    #[test]
    fn test_to_array_res_nul() {
        let actual = to_array_res("hi\0");
        assert!(actual.is_err());
    }

    #[test]
    fn test_to_array_res_too_long() {
        let actual = to_array_res("Hello check this out, I am a long string and there is no signs of stopping; well, maybe one day I will stop when I get too longggggggggggggggggggggggggggggggggggg");
        assert!(actual.is_err());
    }
}
