#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy))]

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
#[allow(unused_variables)]
pub mod overrides {
    use super::*;

    #[export_name = "\x01plugin_dispatch_values"]
    pub extern "C" fn plugin_dispatch_values(vl: *const value_list_t) -> ::std::os::raw::c_int {
        0
    }

    #[export_name = "\x01hostname_g"]
    pub static mut hostname_g: [::std::os::raw::c_char; ARR_LENGTH] = [0; ARR_LENGTH];
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
