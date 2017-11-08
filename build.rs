extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let bindings_builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DHAVE_CONFIG_H")
        .rust_target(bindgen::RustTarget::Stable_1_19)
        .blacklist_type("FP_NAN")
        .blacklist_type("FP_INFINITE")
        .blacklist_type("FP_ZERO")
        .blacklist_type("FP_SUBNORMAL")
        .blacklist_type("FP_NORMAL")
        .blacklist_type("max_align_t")
        .blacklist_type("hostname_g")
        .blacklist_type("module_register");

    #[cfg(feature = "collectd-57")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_57");

    #[cfg(feature = "collectd-55")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_55");

    #[cfg(feature = "collectd-54")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_54");

    let bindings = bindings_builder
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
