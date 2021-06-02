#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::all))]

// In collectd 5.7 the max length of textual information was extended to 128 characters from 64
pub const ARR_LENGTH: usize = DATA_MAX_NAME_LEN as usize;

// We have to override the definition of hostname_g from bindgen as it found hostname_g to be an
// empty array. hostname_g is a global variable defined by collectd to be the hostname to use for
// the value lists. For collectd 5.7 plugins, providing the default hostname is not necessary as
// collectd will fill it in if missing. For those writing plugins for 5.5, we are not so lucky and
// have to explicitly use this global object.
extern "C" {
    #[link_name = "hostname_g"]
    pub static mut hostname_g: [::std::os::raw::c_char; ARR_LENGTH];
}

#[cfg(any(test, feature = "stub"))]
#[doc(hidden)]
#[allow(unused_variables)]
pub mod overrides {
    use super::*;

    #[no_mangle]
    pub extern "C" fn plugin_dispatch_values(vl: *const value_list_t) -> ::std::os::raw::c_int {
        0
    }

    #[no_mangle]
    pub static mut hostname_g: [::std::os::raw::c_char; ARR_LENGTH] = [0; ARR_LENGTH];

    #[no_mangle]
    pub extern "C" fn meta_data_create() -> *mut meta_data_t {
        std::ptr::null_mut()
    }
    #[no_mangle]
    pub extern "C" fn meta_data_type(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_toc(
        md: *mut meta_data_t,
        toc: *mut *mut *mut ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_add_string(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_add_signed_int(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: i64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_add_unsigned_int(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: u64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_add_double(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: f64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_add_boolean(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: bool,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_get_string(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *mut *mut ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_get_signed_int(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *mut i64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_get_unsigned_int(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *mut u64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_get_double(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *mut f64,
    ) -> ::std::os::raw::c_int {
        0
    }
    #[no_mangle]
    pub extern "C" fn meta_data_get_boolean(
        md: *mut meta_data_t,
        key: *const ::std::os::raw::c_char,
        value: *mut bool,
    ) -> ::std::os::raw::c_int {
        0
    }
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
