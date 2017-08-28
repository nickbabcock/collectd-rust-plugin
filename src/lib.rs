mod bindings;
mod api;

use std::os::raw::c_int;
use std::ffi::CString;
use std::ptr;
use bindings::{plugin_log, plugin_register_read, LOG_INFO};
use api::{Value, ValueListBuilder};

/// Collectd hooks into our plugin by calling the `module_register` function, so we let collectd
/// know about our read function.
#[no_mangle]
pub extern "C" fn module_register() {
    let s = CString::new("myplugin").unwrap();
    unsafe {
        plugin_register_read(s.as_ptr(), Some(my_read));
    }
}

#[no_mangle]
pub extern "C" fn my_read() -> c_int {
    log_entrance();

    let values: Vec<Value> = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

    ValueListBuilder::new("myplugin", "load")
        .values(values)
        .submit()
        .expect("value list to be constructed correctly")
}

fn log_entrance() {
    let s = "Entering myplugin read function";
    let cs = CString::new(s).unwrap();
    unsafe {
        plugin_log(LOG_INFO as i32, cs.as_ptr());
    }
}
