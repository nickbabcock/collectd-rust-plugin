extern crate regex;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use regex::Regex;

enum CollectdVersion {
    Collectd54,
    Collectd55,
    Collectd57,
}

fn main() {
    let re = Regex::new(r"collectd (\d+\.\d+)\.\d+").expect("Valid collectd regex");
    println!("cargo:rerun-if-env-changed=COLLECTD_VERSION");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let collectd_version = env::var_os("COLLECTD_VERSION")
        .map(|x| x.into_string().expect("COLLECTD_VERSION to be a valid string"))
        .unwrap_or_else(|| {
                Command::new("collectd")
                    .args(&["-h"])
                    .output()
                    .map(|x| String::from_utf8(x.stdout).expect("Collectd output to be utf8"))
                    .map(|x|
                        re.captures(&x)
                            .expect("Version info to be present in collectd")
                            .get(1)
                            .map(|x| String::from(x.as_str()))
                            .unwrap()
                    )
                    .expect("Collectd -h to execute successfully")
            });

    let version = match collectd_version.as_str() {
        "5.8" | "5.7" => {
            println!("cargo:rustc-cfg=collectd57");
            CollectdVersion::Collectd57
        },
        "5.6" | "5.5" => {
            println!("cargo:rustc-cfg=collectd55");
            CollectdVersion::Collectd55
        },
        "5.4" => {
            println!("cargo:rustc-cfg=collectd54");
            CollectdVersion::Collectd54
        },
        x => panic!("Unrecognized collectd version: {}", x),
    };

    bindings(out_path.join("bindings.rs"), version);
}

#[cfg(feature = "bindgen")]
fn bindings(loc: PathBuf, version: CollectdVersion) {
    extern crate bindgen;
    let arg = match version {
        CollectdVersion::Collectd54 => "-DCOLLECTD_54",
        CollectdVersion::Collectd55 => "-DCOLLECTD_55",
        CollectdVersion::Collectd57 => "-DCOLLECTD_57",
    };

    bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DHAVE_CONFIG_H")
        .clang_arg(arg)
        .rust_target(bindgen::RustTarget::Stable_1_19)
        .whitelist_type("cdtime_t")
        .whitelist_type("data_set_t")
        .whitelist_function("plugin_.*")
        .whitelist_var("OCONFIG_TYPE_.*")
        .whitelist_var("LOG_.*")
        .whitelist_var("DS_TYPE_.*")
        .whitelist_var("DATA_MAX_NAME_LEN")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(loc)
        .expect("Couldn't write bindings!");
}

#[cfg(not(feature = "bindgen"))]
fn bindings(loc: PathBuf, version: CollectdVersion) {
    use std::fs;

    let path = match version {
        CollectdVersion::Collectd54 => "src/bindings-54.rs",
        CollectdVersion::Collectd55 => "src/bindings-55.rs",
        CollectdVersion::Collectd57 => "src/bindings-57.rs",
    };

    fs::copy(PathBuf::from(path), loc).expect("File to copy");
}
