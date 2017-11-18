extern crate chrono;
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod bindings;
mod api;
mod errors;

use std::os::raw::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use bindings::{plugin_log, plugin_register_config, plugin_register_read, LOG_INFO, LOG_WARNING};
use api::{Value, ValueListBuilder};
use std::mem;
use failure::Error;
use errors::ConfigError;


static mut SHORT_VALUE: Option<f64> = None;
static mut MID_VALUE: Option<f64> = None;
static mut LONG_VALUE: Option<f64> = None;

/// Collectd hooks into our plugin by calling the `module_register` function, so we let collectd
/// know about our read function.
#[no_mangle]
pub extern "C" fn module_register() {
    let s = CString::new("myplugin").unwrap();

    // Convert our configuration keys into valid c-strings
    let config_keys: Vec<CString> = vec![
        CString::new("Short").unwrap(),
        CString::new("Mid").unwrap(),
        CString::new("Long").unwrap(),
    ];

    // Now grab all the pointers to the c strings for ffi
    let mut pointers: Vec<*const c_char> = config_keys.iter().map(|arg| arg.as_ptr()).collect();

    unsafe {
        plugin_register_read(s.as_ptr(), Some(my_read));
        plugin_register_config(
            s.as_ptr(),
            Some(my_config),
            pointers.as_mut_ptr(),
            pointers.len() as i32,
        );
    }

    // We must forget the vector as collectd hangs on to the info and if we were to drop it,
    // collectd would segfault trying to read the newly freed up data structure
    mem::forget(pointers);
    mem::forget(config_keys);
}

/// Every key value pair that Collectd finds in our configuration is passed into this function.
#[no_mangle]
pub unsafe extern "C" fn my_config(key: *const c_char, value: *const c_char) -> c_int {
    let key = CStr::from_ptr(key).to_owned();
    let value = CStr::from_ptr(value).to_owned();

    match parse_config(key.clone(), value.clone()) {
        Ok(()) => 0,
        Err(ref e) => {
            let cs = CString::new(format!("Error: {}", e)).unwrap();
            plugin_log(LOG_WARNING as i32, cs.as_ptr());
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn my_read() -> c_int {
    log_entrance();

    // Create a list of values to submit to collectd. We'll be sending in a vector representing the
    // "load" type. Short-term load is first followed by mid-term and long-term. The number of
    // values that you submit at a time depends on types.db in collectd configurations
    let values: Vec<Value> = unsafe {
        vec![
            Value::Gauge(SHORT_VALUE.unwrap_or(15.0)),
            Value::Gauge(MID_VALUE.unwrap_or(10.0)),
            Value::Gauge(LONG_VALUE.unwrap_or(12.0)),
        ]
    };

    // Submit our values to collectd. A plugin can submit any number of times.
    let submission = ValueListBuilder::new("myplugin", "load")
        .values(values)
        .submit();

    // If collectd submission failed return a -1. Collectd will backoff calling
    // our plugin
    match submission {
        Ok(_) => 0,
        Err(ref e) => {
            let cs = CString::new(format!("submission error: {}", e)).unwrap();
            unsafe {
                plugin_log(LOG_WARNING as i32, cs.as_ptr());
            }
            -1
        }
    }
}

fn parse_config(key: CString, value: CString) -> Result<(), Error> {
    let key = key.into_string()?;
    let value = value.into_string()?;
    let keyed = unsafe {
        match key.as_str() {
            "Short" => Ok(&mut SHORT_VALUE),
            "Mid" => Ok(&mut MID_VALUE),
            "Long" => Ok(&mut LONG_VALUE),
            _ => Err(ConfigError::UnrecognizedKey(key.clone())),
        }
    }?;

    let val = value.parse::<f64>().map_err(|x| {
        ConfigError::InvalidValue {
            key: key.clone(),
            value: value.clone(),
            err: x,
        }
    })?;
    *keyed = Some(val);
    Ok(())
}

fn log_entrance() {
    let s = "Entering myplugin read function";
    let cs = CString::new(s).unwrap();
    unsafe {
        plugin_log(LOG_INFO as i32, cs.as_ptr());
    }
}
