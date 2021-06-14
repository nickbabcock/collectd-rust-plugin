//! Collectd is a ubiquitous system statistics collection daemon.
//! `collectd_plugin` leverages Collectd's ability to dynamically load plugins and
//! creates an ergonomic, yet extremely low cost abstraction API to interface with
//! Collectd.
//!
//! Features:
//!
//! - No unnecessary allocations when submitting / receiving values, logging
//! - Register multiple plugin instances
//! - Automatic deserialization of plugin configs via [Serde](https://github.com/serde-rs/serde) (can opt-out)
//! - Deployment: compile against collectd version and scp to server
//! - Referenced Rust libraries are statically linked
//! - Help writing thread safe plugins thanks to the Rust compiler
//!
//! ## Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! collectd-plugin = "0.13.0"
//! ```
//!
//! Rust 1.33 or later is needed to build.
//!
//! Works with any collectd version 5.4+, but all users will need to specify the collectd api
//! version they want to target via the `COLLECTD_VERSION` environment variable (or rely on
//! `$(collectd -h)` or `COLLECTD_PATH` variable).
//!
//! | `COLLECTED_VERSION` |  Compatible Range |
//! |---------------------|-------------------|
//! | 5.4                 | [5.4, 5.5)        |
//! | 5.5                 | [5.5, 5.7)        |
//! | 5.7                 | [5.7,)            |
//!
//! ## Quickstart
//!
//! Below is a complete plugin that dummy reports [load](https://en.wikipedia.org/wiki/Load_(computing)) values to collectd, as it registers a `READ` hook. For an implementation that reimplements Collectd's own load plugin, see [plugins/load](https://github.com/nickbabcock/collectd-rust-plugin/tree/master/plugins/load)
//!
//! ```rust
//! use collectd_plugin::{
//!     ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration, Value,
//!     ValueListBuilder, collectd_plugin
//! };
//! use std::error;
//!
//! #[derive(Default)]
//! struct MyPlugin;
//!
//! // A manager decides the name of the family of plugins and also registers one or more plugins based
//! // on collectd's configuration files
//! impl PluginManager for MyPlugin {
//!     // A plugin needs a unique name to be referenced by collectd
//!     fn name() -> &'static str {
//!         "myplugin"
//!     }
//!
//!     // Our plugin might have configuration section in collectd.conf, which will be passed here if
//!     // present. Our contrived plugin doesn't care about configuration so it returns only a single
//!     // plugin (itself).
//!     fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Box<error::Error>> {
//!         Ok(PluginRegistration::Single(Box::new(MyPlugin)))
//!     }
//! }
//!
//! impl Plugin for MyPlugin {
//!     // We define that our plugin will only be reporting / submitting values to writers
//!     fn capabilities(&self) -> PluginCapabilities {
//!         PluginCapabilities::READ
//!     }
//!
//!     fn read_values(&self) -> Result<(), Box<error::Error>> {
//!         // Create a list of values to submit to collectd. We'll be sending in a vector representing the
//!         // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
//!         // of values that you submit at a time depends on types.db in collectd configurations
//!         let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];
//!
//!         // Submit our values to collectd. A plugin can submit any number of times.
//!         ValueListBuilder::new(Self::name(), "load")
//!             .values(&values)
//!             .submit()?;
//!
//!         Ok(())
//!     }
//! }
//!
//! // We pass in our plugin manager type
//! collectd_plugin!(MyPlugin);
//!
//! # fn main() {
//! # }
//! ```

#[cfg(feature = "serde")]
pub mod de;

#[cfg(feature = "serde")]
pub mod ser;

pub mod bindings;
pub mod internal;
#[macro_use]
mod api;
mod errors;
#[macro_use]
mod plugins;

pub use crate::api::{
    collectd_log, CdTime, CollectdLoggerBuilder, ConfigItem, ConfigValue, LogLevel, MetaValue,
    Value, ValueList, ValueListBuilder, ValueReport,
};
pub use crate::errors::{CacheRateError, ConfigError, ReceiveError, SubmitError};
pub use crate::plugins::{
    Plugin, PluginCapabilities, PluginManager, PluginManagerCapabilities, PluginRegistration,
};

#[cfg(doctest)]
doc_comment::doctest!("../README.md");
