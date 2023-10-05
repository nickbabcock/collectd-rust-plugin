#[rustversion::attr(not(stable), ignore)]
#[test]
fn ui() {
    if std::env::var("CI").is_ok() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/compile-fail/*.rs");
    }
}
