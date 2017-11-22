use failure::Error;

pub trait Plugin {
    fn name(&self) -> &str;

    fn can_config(&self) -> bool {
        false
    }

    fn config_keys(&self) -> Vec<String> {
        unimplemented!()
    }

    fn config_callback(&mut self, _key: String, _value: String) -> Result<(), Error> {
        unimplemented!()
    }

    fn can_report(&self) -> bool {
        false
    }

    fn report_values(&self) -> Result<(), Error> {
        unimplemented!()
    }
}



#[macro_export]
macro_rules! collectd_plugin {
    ($plugin:ident) => {
        use std::os::raw::{c_char, c_int};
        use collectd_plugin::bindings::{plugin_register_config, plugin_register_read};
        use std::ffi::{CString, CStr};
        use std::mem;
        use collectd_plugin::{LogLevel, collectd_log};

        #[no_mangle]
        pub extern "C" fn module_register() {
            let pl = $plugin.lock().unwrap();

            // Use expects for assertions -- no one really should be passing us strings that
            // contain nulls
            let s = CString::new(pl.name()).expect("Plugin name to not contain nulls");

            unsafe {
                if pl.can_report() {
                    plugin_register_read(s.as_ptr(), Some(my_plugin_read));
                }

                if pl.can_config() {
                    let ck: Vec<CString> = pl.config_keys()
                        .into_iter()
                        .map(|x| CString::new(x).expect("Config key to not contain nulls"))
                        .collect();

                    // Now grab all the pointers to the c strings for ffi
                    let mut pointers: Vec<*const c_char> = ck.iter().map(|arg| arg.as_ptr()).collect();

                    plugin_register_config(
                        s.as_ptr(),
                        Some(my_config),
                        pointers.as_mut_ptr(),
                        pointers.len() as i32,
                    );

                    // We must forget the vector as collectd hangs on to the info and if we were to
                    // drop it, collectd would segfault trying to read the newly freed up data
                    // structure
                    mem::forget(ck);
                    mem::forget(pointers);
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn my_plugin_read() -> c_int {
            if let Err(ref e) = $plugin.lock().unwrap().report_values() {
                collectd_log(LogLevel::Error, &format!("read error: {}", e));
                return -1;
            }
            0
        }

        #[no_mangle]
        pub unsafe extern "C" fn my_config(key: *const c_char, value: *const c_char) -> c_int {
            if let Ok(key) = CStr::from_ptr(key).to_owned().into_string() {
                if let Ok(value) = CStr::from_ptr(value).to_owned().into_string() {
                    if let Err(ref e) = $plugin.lock().unwrap().config_callback(key, value) {
                        collectd_log(LogLevel::Error, &format!("config error: {}", e));
                        return -1;
                    } else {
                        return 0;
                    }
                }
            }
            -1
        }
    };
}
