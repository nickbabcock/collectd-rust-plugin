use std::cell::RefCell;
use std::error;
use collectd_plugin::Plugin;

#[derive(Debug, Default)]
pub struct MyPlugin {
    name: RefCell<String>,
}

impl Plugin for MyPlugin {
    fn read_values(&self) -> Result<(), Box<dyn error::Error>> {
        let mut n = self.names.borrow_mut();
        n.pop();
    }
}

fn main() {
}
