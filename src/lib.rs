#[macro_use]
extern crate bitflags;
extern crate chrono;
#[macro_use]
extern crate failure;

pub mod bindings;
mod api;
mod errors;
mod plugins;

pub use api::{collectd_log, LogLevel, Value, ValueListBuilder};
pub use errors::{ArrayError, SubmitError};
pub use plugins::{Plugin, PluginCapabilities};
