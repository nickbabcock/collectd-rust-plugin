extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let bindings_builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DHAVE_CONFIG_H")
        .rust_target(bindgen::RustTarget::Stable_1_19)
        .hide_type("FP_NAN")
        .hide_type("FP_INFINITE")
        .hide_type("FP_ZERO")
        .hide_type("FP_SUBNORMAL")
        .hide_type("FP_NORMAL")
        .hide_type("max_align_t")
        .hide_type("hostname_g")
        .hide_type("module_register");

    #[cfg(feature = "collectd-57")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_NEW");

    let bindings = bindings_builder
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
