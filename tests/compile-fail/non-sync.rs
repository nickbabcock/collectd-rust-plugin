use std::cell::RefCell;
use std::collections::HashSet;
use collectd_plugin::Plugin;
use failure::Error;

#[derive(Debug, Default)]
pub struct MyPlugin {
    names: RefCell<HashSet<String>>,
}

impl Plugin for MyPlugin {
//~^ error: std::cell::RefCell<std::collections::HashSet<std::string::String>>` cannot be shared between threads safely [E0277]
    fn read_values(&self) -> Result<(), Error> {
        let mut n = self.names.borrow_mut();
        n.insert(String::from("A"));
    }
}
