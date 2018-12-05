use bindings::{
    data_set_t, hostname_g, plugin_dispatch_values, uc_get_rate, value_list_t, value_t, ARR_LENGTH,
    DS_TYPE_ABSOLUTE, DS_TYPE_COUNTER, DS_TYPE_DERIVE, DS_TYPE_GAUGE,
};
use chrono::prelude::*;
use chrono::Duration;
use errors::{ArrayError, CacheRateError, ReceiveError, SubmitError};
use memchr::memchr;
use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt;
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str::Utf8Error;

pub use self::cdtime::{nanos_to_collectd, CdTime};
pub use self::logger::{collectd_log, CollectdLoggerBuilder, LogLevel};
pub use self::oconfig::{ConfigItem, ConfigValue};

mod cdtime;
mod logger;
mod oconfig;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
#[allow(dead_code)]
enum ValueType {
    Counter = DS_TYPE_COUNTER,
    Gauge = DS_TYPE_GAUGE,
    Derive = DS_TYPE_DERIVE,
    Absolute = DS_TYPE_ABSOLUTE,
}

/// The value that a plugin reports can be any one of the following types
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

impl Value {
    /// Returns if an underlying value is nan
    ///
    /// ```
    /// # use collectd_plugin::Value;
    /// assert_eq!(true, Value::Gauge(::std::f64::NAN).is_nan());
    /// assert_eq!(false, Value::Gauge(0.0).is_nan());
    /// assert_eq!(false, Value::Derive(0).is_nan());
    /// ```
    pub fn is_nan(&self) -> bool {
        if let Value::Gauge(x) = *self {
            x.is_nan()
        } else {
            false
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Counter(x) | Value::Absolute(x) => write!(f, "{}", x),
            Value::Gauge(x) => write!(f, "{}", x),
            Value::Derive(x) => write!(f, "{}", x),
        }
    }
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

/// Name and value of a reported metric
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ValueReport<'a> {
    /// Name of the metric. If values has a length of 1, this is often just "value"
    pub name: &'a str,

    /// The value reported
    pub value: Value,

    /// Minimum value seen in an interval
    pub min: f64,

    /// Maximum value seen in an interval
    pub max: f64,
}

/// Contains values and metadata that collectd has collected from plugins
#[derive(Debug, PartialEq, Clone)]
pub struct ValueList<'a> {
    pub values: Vec<ValueReport<'a>>,

    /// The plugin that submitted this value. This would be your `PluginManager` when submitting
    /// values
    pub plugin: &'a str,

    /// Distinguishes entities that yield metrics. Each core would be a different instance of the
    /// same plugin, as each core reports "idle", "user", "system" metrics.
    pub plugin_instance: Option<&'a str>,

    /// This is the string found in types.db, determines how many values are expected and how they
    /// should be interpreted
    pub type_: &'a str,

    /// The type instance is used to separate values of identical type which nonetheless belong to
    /// one another. For instance, even though "free", "used", and "total" all have types of
    /// "Memory" they are different type instances.
    pub type_instance: Option<&'a str>,

    /// The hostname where the values were collectd
    pub host: &'a str,

    /// The timestamp at which the value was collected
    pub time: DateTime<Utc>,

    /// The interval in which new values are to be expected
    pub interval: Duration,

    // Keep the original list and set around for calculating rates on demand
    original_list: *const value_list_t,
    original_set: *const data_set_t,
}

impl<'a> ValueList<'a> {
    /// Collectd does not automatically convert `Derived` values into a rate. This is why many
    /// write plugins have a `StoreRates` config option so that these rates are calculated on
    /// demand from collectd's internal cache. This function will return a vector that can supercede
    /// the `values` field that contains the rate of all non-gauge values. Values that are gauges
    /// remain unchanged, so one doesn't need to resort back to `values` field as this function
    /// will return everything prepped for submission.
    pub fn rates(&self) -> Result<Cow<Vec<ValueReport<'a>>>, CacheRateError> {
        // As an optimization step, if we know all values are gauges there is no need to call out
        // to uc_get_rate as no values will be changed
        let all_gauges = self.values.iter().all(|x| match x.value {
            Value::Gauge(_) => true,
            _ => false,
        });

        if all_gauges {
            return Ok(Cow::Borrowed(&self.values));
        }

        let ptr = unsafe { uc_get_rate(self.original_set, self.original_list) };
        if !ptr.is_null() {
            let nv = unsafe { slice::from_raw_parts(ptr, self.values.len()) }
                .iter()
                .zip(self.values.iter())
                .map(|(rate, report)| match report.value {
                    Value::Gauge(_) => *report,
                    _ => ValueReport {
                        value: Value::Gauge(*rate),
                        ..*report
                    },
                }).collect();
            Ok(Cow::Owned(nv))
        } else {
            Err(CacheRateError)
        }
    }

    pub fn from<'b>(
        set: &'b data_set_t,
        list: &'b value_list_t,
    ) -> Result<ValueList<'b>, ReceiveError> {
        let p = from_array(&list.plugin)
            .map_err(|e| ReceiveError::Utf8(String::from(""), "plugin name", e))?;
        let ds_len = length(set.ds_num);
        let list_len = length(list.values_len);

        let values: Result<Vec<ValueReport>, ReceiveError> =
            unsafe { slice::from_raw_parts(list.values, list_len) }
                .iter()
                .zip(unsafe { slice::from_raw_parts(set.ds, ds_len) })
                .map(|(val, source)| unsafe {
                    let v = match ::std::mem::transmute(source.type_) {
                        ValueType::Gauge => Value::Gauge(val.gauge),
                        ValueType::Counter => Value::Counter(val.counter),
                        ValueType::Derive => Value::Derive(val.derive),
                        ValueType::Absolute => Value::Absolute(val.absolute),
                    };

                    let name = from_array(&source.name)
                        .map_err(|e| ReceiveError::Utf8(String::from(p), "data source name", e))?;

                    Ok(ValueReport {
                        name,
                        value: v,
                        min: source.min,
                        max: source.max,
                    })
                }).collect();

        assert!(list.time > 0);
        assert!(list.interval > 0);

        let plugin_instance = from_array(&list.plugin_instance)
            .map_err(|e| ReceiveError::Utf8(String::from(p), "plugin_instance", e))
            .map(empty_to_none)?;

        let type_ =
            from_array(&list.type_).map_err(|e| ReceiveError::Utf8(String::from(p), "type", e))?;

        let type_instance = from_array(&list.type_instance)
            .map_err(|e| ReceiveError::Utf8(String::from(p), "type instance", e))
            .map(empty_to_none)?;

        let host =
            from_array(&list.host).map_err(|e| ReceiveError::Utf8(String::from(p), "host", e))?;

        Ok(ValueList {
            values: values?,
            plugin_instance,
            plugin: p,
            type_,
            type_instance,
            host,
            time: CdTime::from(list.time).into(),
            interval: CdTime::from(list.interval).into(),
            original_list: list,
            original_set: set,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
struct SubmitValueList<'a> {
    values: &'a [Value],
    plugin_instance: Option<&'a str>,
    plugin: &'a str,
    type_: &'a str,
    type_instance: Option<&'a str>,
    host: Option<&'a str>,
    time: Option<DateTime<Utc>>,
    interval: Option<Duration>,
}

/// Creates a value list to report values to collectd.
#[derive(Debug, PartialEq, Clone)]
pub struct ValueListBuilder<'a> {
    list: SubmitValueList<'a>,
}

impl<'a> ValueListBuilder<'a> {
    /// Primes a value list for submission. `plugin` will most likely be the name from the
    /// `PluginManager` and `type_` is the datatype found in types.db
    pub fn new<T: Into<&'a str>, U: Into<&'a str>>(plugin: T, type_: U) -> ValueListBuilder<'a> {
        ValueListBuilder {
            list: SubmitValueList {
                values: &[],
                plugin_instance: None,
                plugin: plugin.into(),
                type_: type_.into(),
                type_instance: None,
                host: None,
                time: None,
                interval: None,
            },
        }
    }

    /// A set of observed values that belong to the same plugin and type instance
    pub fn values(mut self, values: &'a [Value]) -> ValueListBuilder<'a> {
        self.list.values = values;
        self
    }

    /// Distinguishes entities that yield metrics. Each core would be a different instance of the
    /// same plugin, as each core reports "idle", "user", "system" metrics.
    pub fn plugin_instance<T: Into<&'a str>>(mut self, plugin_instance: T) -> ValueListBuilder<'a> {
        self.list.plugin_instance = Some(plugin_instance.into());
        self
    }

    /// The type instance is used to separate values of identical type which nonetheless belong to
    /// one another. For instance, even though "free", "used", and "total" all have types of
    /// "Memory" they are different type instances.
    pub fn type_instance<T: Into<&'a str>>(mut self, type_instance: T) -> ValueListBuilder<'a> {
        self.list.type_instance = Some(type_instance.into());
        self
    }

    /// Override the machine's hostname that the observed values will be attributed to. Best to
    /// override when observing values from another machine
    pub fn host<T: Into<&'a str>>(mut self, host: T) -> ValueListBuilder<'a> {
        self.list.host = Some(host.into());
        self
    }

    /// The timestamp at which the value was collected. Overrides the default time, which is when
    /// collectd receives the values from `submit`. Use only if there is a significant delay is
    /// metrics gathering or if submitting values from the past.
    pub fn time(mut self, dt: DateTime<Utc>) -> ValueListBuilder<'a> {
        self.list.time = Some(dt);
        self
    }

    /// The interval in which new values are to be expected. This is typically handled at a global
    /// or plugin level. Use at your own discretion.
    pub fn interval(mut self, interval: Duration) -> ValueListBuilder<'a> {
        self.list.interval = Some(interval);
        self
    }

    /// Submits the observed values to collectd and returns errors if encountered
    pub fn submit(self) -> Result<(), SubmitError> {
        let mut v: Vec<value_t> = self.list.values.into_iter().map(|&x| x.into()).collect();
        let plugin_instance = self
            .list
            .plugin_instance
            .map(|x| to_array_res(x).map_err(|e| SubmitError::Field("plugin_instance", e)))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let type_instance = self
            .list
            .type_instance
            .map(|x| to_array_res(x).map_err(|e| SubmitError::Field("type_instance", e)))
            .unwrap_or_else(|| Ok([0i8; ARR_LENGTH]))?;

        let host = self
            .list
            .host
            .map(|x| to_array_res(x).map_err(|e| SubmitError::Field("host", e)))
            .unwrap_or_else(|| {
                // If a custom host is not provided by the plugin, we default to the global
                // hostname. In versions prior to collectd 5.7, it was required to propagate the
                // global hostname (hostname_g) in the submission. In collectd 5.7, one could
                // submit an empty array or hostname_g and they would equate to the same thing. In
                // collectd 5.8, hostname_g had the type signature changed so it could no longer be
                // submitted and would cause garbage to be read (and thus could have very much
                // unintended side effects)
                if cfg!(collectd57) {
                    Ok([0i8; ARR_LENGTH])
                } else {
                    unsafe { Ok(hostname_g) }
                }
            })?;

        #[cfg(collectd57)]
        let len = v.len();

        #[cfg(not(collectd57))]
        let len = v.len() as i32;

        let plugin = to_array_res(self.list.plugin).map_err(|e| SubmitError::Field("plugin", e))?;

        let type_ = to_array_res(self.list.type_).map_err(|e| SubmitError::Field("type", e))?;

        let list = value_list_t {
            values: v.as_mut_ptr(),
            values_len: len,
            plugin_instance,
            plugin,
            type_,
            type_instance,
            host,
            time: self.list.time.map(CdTime::from).unwrap_or(CdTime(0)).into(),
            interval: self
                .list
                .interval
                .map(CdTime::from)
                .unwrap_or(CdTime(0))
                .into(),
            meta: ptr::null_mut(),
        };

        match unsafe { plugin_dispatch_values(&list) } {
            0 => Ok(()),
            i => Err(SubmitError::Dispatch(i)),
        }
    }
}

/// Collectd stores textual data in fixed sized arrays, so this function will convert a string
/// slice into array compatible with collectd's text fields. Be aware that `ARR_LENGTH` is 64
/// before collectd 5.7
fn to_array_res(s: &str) -> Result<[c_char; ARR_LENGTH], ArrayError> {
    // By checking if the length is greater than or *equal* to, we guarantee a trailing null
    if s.len() >= ARR_LENGTH {
        return Err(ArrayError::TooLong(s.len()));
    }

    let bytes = s.as_bytes();

    // Using memchr to find a null and work around it is 10x faster than
    // using a CString to get the bytes_with_nul and cut the time to submit
    // values to collectd in half.
    if let Some(ind) = memchr(0, bytes) {
        return Err(ArrayError::NullPresent(ind, s.to_string()));
    }

    let mut arr = [0; ARR_LENGTH];
    arr[0..bytes.len()].copy_from_slice(bytes);
    Ok(unsafe { ::std::mem::transmute(arr) })
}

/// Turns a fixed size character array into string slice, if possible
///
/// # Examples
///
/// ```
/// use collectd_plugin::from_array;
/// use collectd_plugin::bindings::ARR_LENGTH;
/// use std::os::raw::c_char;
///
/// let mut name: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
/// name[0] = b'h' as c_char;
/// name[1] = b'i' as c_char;
/// assert_eq!(Ok("hi"), from_array(&name));
/// ```
pub fn from_array(s: &[c_char; ARR_LENGTH]) -> Result<&str, Utf8Error> {
    unsafe {
        let a = s as *const [i8; ARR_LENGTH] as *const i8;
        CStr::from_ptr(a).to_str()
    }
}

/// Returns if the string is empty or not
///
/// # Examples
///
/// ```
/// use collectd_plugin::empty_to_none;
///
/// assert_eq!(None, empty_to_none(""));
///
/// let s = "hi";
/// assert_eq!(Some("hi"), empty_to_none(s));
/// ```
pub fn empty_to_none(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(collectd57)]
pub fn length(len: usize) -> usize {
    len
}

#[cfg(not(collectd57))]
pub fn length(len: i32) -> usize {
    len as usize
}

#[cfg(collectd57)]
pub fn get_default_interval() -> u64 {
    0
}

#[cfg(not(collectd57))]
pub fn get_default_interval<T>() -> *const T {
    use std::ptr;
    ptr::null()
}

#[cfg(test)]
mod tests {
    use self::cdtime::nanos_to_collectd;
    use super::*;
    use bindings::data_source_t;
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
    fn test_submit() {
        let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];
        let result = ValueListBuilder::new("my-plugin", "load")
            .values(&values)
            .submit();
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn test_recv_value_list_conversion() {
        let empty: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        let mut metric: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        metric[0] = b'h' as c_char;
        metric[1] = b'o' as c_char;

        let mut name: [c_char; ARR_LENGTH] = [0; ARR_LENGTH];
        name[0] = b'h' as c_char;
        name[1] = b'i' as c_char;

        let val = data_source_t {
            name,
            type_: DS_TYPE_GAUGE as i32,
            min: 10.0,
            max: 11.0,
        };

        let mut v = vec![val];

        let conv = data_set_t {
            type_: metric,
            ds_num: 1,
            ds: v.as_mut_ptr(),
        };

        let mut vs = vec![value_t { gauge: 3.0 }];

        let list_t = value_list_t {
            values: vs.as_mut_ptr(),
            values_len: 1,
            time: nanos_to_collectd(1_000_000_000),
            interval: nanos_to_collectd(1_000_000_000),
            host: metric,
            plugin: name,
            plugin_instance: metric,
            type_: metric,
            type_instance: empty,
            meta: ptr::null_mut(),
        };

        let actual = ValueList::from(&conv, &list_t).unwrap();
        assert_eq!(
            actual,
            ValueList {
                values: vec![ValueReport {
                    name: "hi",
                    value: Value::Gauge(3.0),
                    min: 10.0,
                    max: 11.0,
                }],
                plugin_instance: Some("ho"),
                plugin: "hi",
                type_: "ho",
                type_instance: None,
                host: "ho",
                time: Utc.ymd(1970, 1, 1).and_hms(0, 0, 1),
                interval: Duration::seconds(1),
                original_list: &list_t,
                original_set: &conv,
            }
        );
    }
}
