use std::env;
use std::path::PathBuf;

fn main() {
    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings(out_path.join("bindings.rs"));
}

#[cfg(feature = "bindgen")]
fn bindings(loc: PathBuf) {
    extern crate bindgen;
    let bindings_builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DHAVE_CONFIG_H")
        .rust_target(bindgen::RustTarget::Stable_1_19)
        .whitelist_type("cdtime_t")
        .whitelist_type("data_set_t")
        .whitelist_function("plugin_.*")
        .whitelist_var("OCONFIG_TYPE_.*")
        .whitelist_var("LOG_.*")
        .whitelist_var("DS_TYPE_.*")
        .whitelist_var("DATA_MAX_NAME_LEN");

    #[cfg(feature = "collectd-57")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_57");

    #[cfg(feature = "collectd-55")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_55");

    #[cfg(feature = "collectd-54")]
    let bindings_builder = bindings_builder.clang_arg("-DCOLLECTD_54");

    let bindings = bindings_builder
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(loc)
        .expect("Couldn't write bindings!");
}

#[cfg(not(feature = "bindgen"))]
fn bindings(loc: PathBuf) {
    use std::fs;

    // Default to using collectd-57 bindings
    #[allow(unused_variables)]
    let path = PathBuf::from("src/bindings-57.rs");

    #[cfg(feature = "collectd-55")]
    let path = PathBuf::from("src/bindings-55.rs");

    #[cfg(feature = "collectd-54")]
    let path = PathBuf::from("src/bindings-54.rs");

    fs::copy(path, loc).expect("File to copy");
}
