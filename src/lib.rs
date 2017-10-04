#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

mod bindings;
mod api;

use std::os::raw::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use bindings::{plugin_log, plugin_register_config, plugin_register_read, LOG_INFO, LOG_WARNING};
use api::{Value, ValueListBuilder};
use std::mem;

pub mod errors {
    use std::ffi::IntoStringError;
    use std::num::ParseFloatError;
    error_chain! {
        foreign_links {
            IntoString(IntoStringError);
            ParseNumber(ParseFloatError);
        }
        errors {
            InvalidValue(key: String, value: String) {
                description("value could not be parsed")
                display("value {} for key {} is not a number", key, value)
            }
            UnrecognizedKey(key: String) {
                description("config key not recognized")
                display("config key {} not recognized", key)
            }
        }
    }
}

use self::errors::*;

lazy_static! {
    static ref KEYS: Vec<&'static str> = {
        vec!["Short", "Mid", "Long"]
    };
}

static mut SHORT_VALUE: Option<f64> = None;
static mut MID_VALUE: Option<f64> = None;
static mut LONG_VALUE: Option<f64> = None;

/// Collectd hooks into our plugin by calling the `module_register` function, so we let collectd
/// know about our read function.
#[no_mangle]
pub extern "C" fn module_register() {
    let s = CString::new("myplugin").unwrap();

    // Convert our configuration keys into valid c-strings
    let cstrs: Vec<CString> = KEYS.clone()
        .into_iter()
        .map(|arg| CString::new(arg).unwrap())
        .collect();

    // Now grab all the pointers to the c strings for ffi
    let mut pointers: Vec<*const c_char> = cstrs.iter().map(|arg| arg.as_ptr()).collect();

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
    mem::forget(cstrs);
}

#[no_mangle]
pub unsafe extern "C" fn my_config(key: *const c_char, value: *const c_char) -> c_int {
    let key = CStr::from_ptr(key).to_owned();
    let value = CStr::from_ptr(value).to_owned();

    match parse_config(key.clone(), value.clone()) {
        Ok(ret) => ret,
        Err(ref e) => {
            let cs = CString::new(e.to_string()).unwrap();
            plugin_log(LOG_WARNING as i32, cs.as_ptr());
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn my_read() -> c_int {
    log_entrance();

    let values: Vec<Value> = unsafe {
        vec![
            Value::Gauge(SHORT_VALUE.unwrap_or(15.0)),
            Value::Gauge(MID_VALUE.unwrap_or(10.0)),
            Value::Gauge(LONG_VALUE.unwrap_or(12.0)),
        ]
    };

    ValueListBuilder::new("myplugin", "load")
        .values(values)
        .submit()
        .expect("value list to be constructed correctly")
}


fn parse_config(key: CString, value: CString) -> Result<c_int> {
    let key = key.into_string()?;
    let value = value.into_string()?;
    let keyed = unsafe {
        match key.as_str() {
            "Short" => Ok(&mut SHORT_VALUE),
            "Mid" => Ok(&mut MID_VALUE),
            "Long" => Ok(&mut LONG_VALUE),
            _ => Err(ErrorKind::UnrecognizedKey(key.clone())),
        }
    }?;

    let val = value
        .parse::<f64>()
        .chain_err(|| ErrorKind::InvalidValue(key.clone(), value.clone()))?;
    *keyed = Some(val);
    Ok(0)
}

fn log_entrance() {
    let s = "Entering myplugin read function";
    let cs = CString::new(s).unwrap();
    unsafe {
        plugin_log(LOG_INFO as i32, cs.as_ptr());
    }
}
