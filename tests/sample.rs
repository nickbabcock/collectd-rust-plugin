#[macro_use]
extern crate collectd_plugin;
extern crate failure;

mod tt {
    use collectd_plugin::*;
    use failure::Error;

    pub struct MyPlugin;

    impl PluginManager for MyPlugin {
        fn name() -> &'static str {
            "myplugin"
        }

        fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
            collectd_log_raw!(LogLevel::Info, b"test %d\0", 10);
            Ok(PluginRegistration::Multiple(vec![]))
        }
    }
}

collectd_plugin!(tt::MyPlugin);

#[test]
fn can_generate_blank_plugin() {
    assert!(true);
}
