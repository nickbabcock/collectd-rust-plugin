use std::env;
use std::path::PathBuf;

enum CollectdVersion {
    Collectd57,
}

fn main() {
    let collectd_version = detect_collectd_version();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let version = match collectd_version.as_str() {
        "5.11" | "5.10" | "5.9" | "5.8" | "5.7" => {
            println!("cargo:rustc-cfg=collectd57");
            CollectdVersion::Collectd57
        }
        x => panic!("Unrecognized collectd version: {}", x),
    };

    bindings(out_path.join("bindings.rs"), version);
}

#[cfg(feature = "stub")]
fn detect_collectd_version() -> String {
    String::from("5.7")
}

#[cfg(not(feature = "stub"))]
fn detect_collectd_version() -> String {
    use regex::Regex;
    use std::process::Command;

    println!("cargo:rerun-if-env-changed=COLLECTD_VERSION");
    println!("cargo:rerun-if-env-changed=COLLECTD_PATH");

    if let Some(path) = env::var_os("COLLECTD_PATH") {
        let re = Regex::new(r"^(\d+\.\d+).\d+").expect("Valid collectd regex");
        let mut script = PathBuf::from(path.clone());
        script.push("version-gen.sh");
        return script
            .as_path()
            .canonicalize()
            .and_then(|p| Command::new(p).current_dir(path).output())
            .map(|x| String::from_utf8(x.stdout).expect("Collectd output to be utf8"))
            .map(|x| {
                re.captures(&x)
                    .expect("Version info to be present in version-gen.sh")
                    .get(1)
                    .map(|x| String::from(x.as_str()))
                    .unwrap()
            })
            .expect(
                "version-gen.sh did not execute successfully. \
                 Ensure that COLLECTD_PATH points to a collectd source directory",
            );
    }

    let re = Regex::new(r"collectd (\d+\.\d+)\.\d+").expect("Valid collectd regex");

    env::var_os("COLLECTD_VERSION")
        .map(|x| {
            x.into_string()
                .expect("COLLECTD_VERSION to be a valid string")
        }).unwrap_or_else(|| {
            Command::new("collectd")
                .args(&["-h"])
                .output()
                .map(|x| String::from_utf8(x.stdout).expect("Collectd output to be utf8"))
                .map(|x| {
                    re.captures(&x)
                        .expect("Version info to be present in collectd")
                        .get(1)
                        .map(|x| String::from(x.as_str()))
                        .unwrap()
                }).unwrap_or_else(|_| String::from("5.7"))
        })
}

#[cfg(feature = "bindgen")]
fn bindings(loc: PathBuf, version: CollectdVersion) {
    let mut builder = bindgen::Builder::default().header("wrapper.h");

    if let Some(path) = env::var_os("COLLECTD_PATH") {
        let mut linker = String::from("-I");
        linker.push_str(&path.to_string_lossy());
        linker.push_str("/src");

        // collectd loves to reference "collectd.h" when they are in very different directories so
        // we add another path
        let mut linker2 = String::from("-I");
        linker2.push_str(&path.to_string_lossy());
        linker2.push_str("/src/daemon");
        builder = builder
            .clang_arg(linker)
            .clang_arg(linker2)
            .clang_arg("-DCOLLECTD_PATH");
    } else {
        let arg = match version {
            CollectdVersion::Collectd57 => "-DCOLLECTD_57",
        };

        builder = builder.clang_arg("-DHAVE_CONFIG_H").clang_arg(arg);
    }

    builder
        .rust_target(bindgen::RustTarget::Stable_1_33)
        .whitelist_type("cdtime_t")
        .whitelist_type("data_set_t")
        .whitelist_function("plugin_.*")
        .whitelist_function("uc_get_rate")
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
        CollectdVersion::Collectd57 => "src/bindings-57.rs",
    };

    fs::copy(PathBuf::from(path), loc).expect("File to copy");
}
