#![allow(dead_code)]

use bindings::{hostname_g, plugin_dispatch_values, plugin_log, value_list_t, value_t, ARR_LENGTH,
               LOG_DEBUG, LOG_ERR, LOG_INFO, LOG_NOTICE, LOG_WARNING, DS_TYPE_COUNTER, DS_TYPE_GAUGE, DS_TYPE_DERIVE, DS_TYPE_ABSOLUTE, data_source_t, data_set_t};
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use chrono::prelude::*;
use chrono::Duration;
use std::ffi::{CString, CStr};
use failure::{Error, ResultExt};
use errors::{ArrayError, SubmitError};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum LogLevel {
    Error = LOG_ERR,
    Warning = LOG_WARNING,
    Notice = LOG_NOTICE,
    Info = LOG_INFO,
    Debug = LOG_DEBUG,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum ValueType {
    Counter = DS_TYPE_COUNTER,
    Gauge = DS_TYPE_GAUGE,
    Derive = DS_TYPE_DERIVE,
    Absolute = DS_TYPE_ABSOLUTE,
}

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

#[derive(Debug, PartialEq, Clone)]
pub struct DataSource {
    pub name: String,
    pub value_type: ValueType,
    pub min: f64,
    pub max: f64,
}

impl From<data_source_t> for DataSource {
    fn from(val: data_source_t) -> DataSource {
        unsafe {
            DataSource {
                name: from_array(val.name),
                value_type: ::std::mem::transmute(val.type_),
                min: val.min,
                max: val.max,
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DataSet {
    pub metric: String,
    pub sources: Vec<DataSource>,
}

impl From<data_set_t> for DataSet {
    fn from(val: data_set_t) -> DataSet {
        unsafe {
            #[cfg(feature = "collectd-57")]
            let len = val.ds_num;

            #[cfg(not(feature = "collectd-57"))]
            let len = val.ds_num as usize;

            let ds = slice::from_raw_parts(val.ds, len)
                .iter()
                .map(|x| DataSource::from(*x))
                .collect();

            DataSet {
                metric: from_array(val.type_),
                sources: ds,
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ValueList {
    pub values: Vec<Value>,
    pub plugin_instance: Option<String>,
    pub plugin: String,
    pub type_: String,
    pub type_instance: Option<String>,
    pub host: Option<String>,
    pub time: Option<DateTime<Utc>>,
    pub interval: Option<Duration>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ValueListBuilder {
    list: ValueList
}

impl ValueListBuilder {
    pub fn new(plugin: &str, type_: &str) -> ValueListBuilder {
        ValueListBuilder {
            list: ValueList {
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
    }

    /// A set of observed values that belong to the same plugin and type instance
    pub fn values(mut self, values: Vec<Value>) -> ValueListBuilder {
        self.list.values = values;
        self
    }

    /// Distinguishes entities that yield metrics. Each core would be a different instance of the
    /// same plugin, as each core reports "idle", "user", "system" metrics.
    pub fn plugin_instance(mut self, plugin_instance: String) -> ValueListBuilder {
        self.list.plugin_instance = Some(plugin_instance);
        self
    }

    /// The type instance is used to separate values of identical type which nonetheless belong to
    /// one another. For instance, even though "free", "used", and "total" all have types of
    /// "Memory" they are different type instances.
    pub fn type_instance(mut self, type_instance: String) -> ValueListBuilder {
        self.list.type_instance = Some(type_instance);
        self
    }

    /// Override the machine's hostname that the observed values will be attributed to. Best to
    /// override when observing values from another machine
    pub fn host(mut self, host: String) -> ValueListBuilder {
        self.list.host = Some(host);
        self
    }

    /// The timestamp at which the value was collected. Overrides the default time, which is when
    /// collectd receives the values from `submit`. Use only if there is a significant delay is
    /// metrics gathering or if submitting values from the past.
    pub fn time(mut self, dt: DateTime<Utc>) -> ValueListBuilder {
        self.list.time = Some(dt);
        self
    }

    /// The interval in which new values are to be expected. This is typically handled at a global
    /// or plugin level. Use at your own discretion.
    pub fn interval(mut self, interval: Duration) -> ValueListBuilder {
        self.list.interval = Some(interval);
        self
    }

    /// Submits the observed values to collectd and returns errors if encountered
    pub fn submit(self) -> Result<(), Error> {
        let mut v: Vec<value_t> = self.list.values.into_iter().map(|x| x.into()).collect();
        let plugin_instance = self.list.plugin_instance
            .map(|x| to_array_res(&x).context("plugin_instance"))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let type_instance = self.list.type_instance
            .map(|x| to_array_res(&x).context("type_instance"))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        // In collectd 5.7, it is no longer required to supply hostname_g for default hostname,
        // an empty array will get replaced with the hostname. However, since we're collectd 5.5
        // compatible, we use hostname_g in both circumstances, as it is not harmful
        let host = self.list.host
            .map(|x| to_array_res(&x).context("host"))
            .unwrap_or_else(|| unsafe { Ok(hostname_g) })?;

        #[cfg(feature = "collectd-57")]
        let len = v.len();

        #[cfg(not(feature = "collectd-57"))]
        let len = v.len() as i32;

        let list = value_list_t {
            values: v.as_mut_ptr(),
            values_len: len,
            plugin_instance: plugin_instance,
            plugin: to_array_res(&self.list.plugin)?,
            type_: to_array_res(&self.list.type_)?,
            type_instance: type_instance,
            host: host,
            time: self.list.time
                .map(|dt| nanos_to_collectd((dt.timestamp() as u64) + u64::from(dt.timestamp_subsec_nanos())))
                .unwrap_or(0),
            interval: self.list.interval
                .map(|d| nanos_to_collectd(d.num_nanoseconds().unwrap() as u64))
                .unwrap_or(0),
            meta: ptr::null_mut(),
        };

        match unsafe { plugin_dispatch_values(&list) } {
            0 => Ok(()),
            i => Err(SubmitError::DispatchError(i).into()),
        }
    }
}

/// Collectd stores textual data in fixed sized arrays, so this function will convert a string
/// slice into array compatible with collectd's text fields. Be aware that `ARR_LENGTH` is 64
/// before collectd 5.7
fn to_array_res(s: &str) -> Result<[c_char; ARR_LENGTH], ArrayError> {
    let value = CString::new(s)?;
    let data = value.as_bytes_with_nul();
    if data.len() > ARR_LENGTH {
        return Err(ArrayError::TooLong(s.len()));
    }

    let mut arr = [0; ARR_LENGTH];
    for (i, &c) in data.into_iter().enumerate() {
        arr[i] = c as c_char;
    }
    Ok(arr)
}

fn from_array(mut s: [c_char; ARR_LENGTH]) -> String {
    unsafe {
        // Safe way to make sure everything is null terminated
        s[ARR_LENGTH - 1] = 0;
        let a = ::std::mem::transmute(&s);
        CStr::from_ptr(a).to_owned().into_string().unwrap()
    }
}

/// The time is stored at a 2^-30 second resolution, i.e. the most significant 34 bit are used to
/// store the time in seconds, the least significant bits store the sub-second part in something
/// very close to nanoseconds. *The* big advantage of storing time in this manner is that comparing
/// times and calculating differences is as simple as it is with `time_t`, i.e. a simple integer
/// comparison / subtraction works.
fn nanos_to_collectd(nanos: u64) -> u64 {
    ((nanos / 1_000_000_000) << 30)
        | ((((nanos % 1_000_000_000) << 30) + 500_000_000) / 1_000_000_000)
}

/// Sends message and log level to collectd. Collectd configuration determines if a level is logged
/// and where it is delivered.
///
/// # Panics
///
/// If a message containing a null character is given as a message this function will panic.
pub fn collectd_log(lvl: LogLevel, message: &str) {
    let cs = CString::new(message).expect("Collectd log to not contain nulls");
    unsafe {
        plugin_log(lvl as i32, cs.as_ptr());
    }
}

#[cfg(feature = "collectd-57")]
pub fn get_default_interval() -> u64 {
    0
}

#[cfg(not(feature = "collectd-57"))]
pub fn get_default_interval<T>() -> *const T {
    use std::ptr;
    ptr::null()
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
        let actual = to_array_res(
            "Hello check this out, I am a long string and there is no signs of stopping; well, maybe one day I will stop when I get too longggggggggggggggggggggggggggggggggggg",
        );
        assert!(actual.is_err());
    }

    #[test]
    fn test_nanos_to_collectd() {
        // Taken from utils_time_test.c

        assert_eq!(nanos_to_collectd(1439981652801860766), 1546168526406004689);
        assert_eq!(nanos_to_collectd(1439981836985281914), 1546168724171447263);
        assert_eq!(nanos_to_collectd(1439981880053705608), 1546168770415815077);
    }

    #[test]
    fn test_data_source_conversion() {
        let mut name: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        name[0] = b'h' as c_char;
        name[1] = b'i' as c_char;

        let val = data_source_t {
            name: name,
            type_: DS_TYPE_GAUGE as i32,
            min: 10.0,
            max: 10.0,
        };

        let actual = DataSource::from(val);
        assert_eq!(actual, DataSource {
            name: "hi".to_string(),
            value_type: ValueType::Gauge,
            min: 10.0,
            max: 10.0,
        });
    }

    #[test]
    fn test_data_set_conversion() {
        let mut metric: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        metric[0] = b'h' as c_char;
        metric[1] = b'o' as c_char;

        let mut name: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        name[0] = b'h' as c_char;
        name[1] = b'i' as c_char;

        let val = data_source_t {
            name: name,
            type_: DS_TYPE_GAUGE as i32,
            min: 10.0,
            max: 10.0,
        };

        let mut v = vec![val];

        let conv = data_set_t {
            type_: metric,
            ds_num: 1,
            ds: v.as_mut_ptr(),
        };

        let actual = DataSet::from(conv);
        assert_eq!(actual, DataSet {
            metric: "ho".to_string(),
            sources: vec![
                DataSource {
                    name: "hi".to_string(),
                    value_type: ValueType::Gauge,
                    min: 10.0,
                    max: 10.0,
                }
            ]
        });
    }
}
