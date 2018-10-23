// This test is disabled by default due compiletest issues:
// https://github.com/laumann/compiletest-rs/issues/114
//
// If you do want to run this:
//
// ```
// cargo clean
// cargo test --features unstable
// ```
#[cfg(feature = "unstable")]
mod tests {
    extern crate compiletest_rs as compiletest;
    use std::path::PathBuf;

    fn run_mode(mode: &'static str) {
        let mut config = compiletest::Config::default();

        config.mode = mode.parse().expect("Invalid mode");
        config.src_base = PathBuf::from(format!("tests/{}", mode));
        config.link_deps(); // Populate config.target_rustcflags with dependencies on the path
        config.clean_rmeta(); // If your tests import the parent crate, this helps with E0464

        compiletest::run_tests(&config);
    }

    #[test]
    fn compile_test() {
        run_mode("compile-fail");
    }
}
