#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[cfg(feature = "collectd-57")]
pub const ARR_LENGTH: usize = 128;

#[cfg(not(feature = "collectd-57"))]
pub const ARR_LENGTH: usize = 64;

extern "C" {
    #[link_name = "hostname_g"]
    pub static mut hostname_g: [::std::os::raw::c_char; ARR_LENGTH];
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
