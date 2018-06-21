[![Build Status](https://travis-ci.org/nickbabcock/collectd-rust-plugin.svg?branch=master)](https://travis-ci.org/nickbabcock/collectd-rust-plugin) [![](https://docs.rs/collectd-plugin/badge.svg)](https://docs.rs/collectd-plugin) [![Rust](https://img.shields.io/badge/rust-1.24%2B-blue.svg?maxAge=3600)](https://github.com/nickbabcock/collectd-rust-plugin) [![Version](https://img.shields.io/crates/v/collectd-plugin.svg?style=flat-square)](https://crates.io/crates/collectd-plugin)

# A Collectd Plugin Written in Rust

Collectd is a ubiquitous system statistics collection daemon.
`collectd_plugin` leverages Collectd's ability to dynamically load plugins and
creates an ergonomic, yet extremely low cost abstraction API to interface with
Collectd.

Features:

- No unnecessary allocations when submitting / receiving values, logging
- Register multiple plugin instances
- Automatic deserialization of plugin configs via [Serde](https://github.com/serde-rs/serde) (optional) feature
- Deployment: compile against collectd version and scp to server
- Referenced Rust libraries are statically linked

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
collectd-plugin = "0.5.2"
```

If you want [Serde](https://github.com/serde-rs/serde) support (recommended), include the serde feature:

```toml
[dependencies.collectd-plugin]
version = "0.5.2"
features = ["serde"]
```

Then put this in your crate root:

```rust
extern crate collectd_plugin;
```

This repo is tested on the following:

- Collectd 5.4 (Ubuntu 14.04)
- Collectd 5.5 (Ubuntu 16.04)
- Collectd 5.7 (and above) (Ubuntu 17.04)

## Quickstart

[See what to add to your project's Cargo file](#to-build)

Below is a complete plugin that dummy reports [load](https://en.wikipedia.org/wiki/Load_(computing)) values to collectd, as it registers a `READ` hook. For an implementation that reimplements Collectd's own load plugin, see [examples/load](https://github.com/nickbabcock/collectd-rust-plugin/tree/master/examples/load.rs)

```rust
#[macro_use]
extern crate collectd_plugin;
extern crate failure;

use collectd_plugin::{ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration,
                      Value, ValueListBuilder};
use failure::Error;

#[derive(Default)]
struct MyPlugin;

// A manager decides the name of the family of plugins and also registers one or more plugins based
// on collectd's configuration files
impl PluginManager for MyPlugin {
    // A plugin needs a unique name to be referenced by collectd
    fn name() -> &'static str {
        "myplugin"
    }

    // Our plugin might have configuration section in collectd.conf, which will be passed here if
    // present. Our contrived plugin doesn't care about configuration so it returns only a single
    // plugin (itself).
    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
        Ok(PluginRegistration::Single(Box::new(MyPlugin)))
    }
}

impl Plugin for MyPlugin {
    // We define that our plugin will only be reporting / submitting values to writers
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&mut self) -> Result<(), Error> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
        // of values that you submit at a time depends on types.db in collectd configurations
        let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(Self::name(), "load")
            .values(&values)
            .submit()
    }
}

// We pass in our plugin manager type
collectd_plugin!(MyPlugin);
```

## Motivation

There are five main ways to extend collectd:

- Write plugin against the C api: `<collectd/core/daemon/plugin.h>`
- Write plugin for [collectd-python](https://collectd.org/documentation/manpages/collectd-python.5.shtml)
- Write plugin for [collectd-java](https://collectd.org/wiki/index.php/Plugin:Java)
- Write a cli for the [exec plugin](https://collectd.org/documentation/manpages/collectd-exec.5.shtml)
- Write a service that [writes to a unix socket](https://collectd.org/wiki/index.php/Plugin:UnixSock)

And my thoughts:

- I'm not confident enough to write C without leaks and there isn't a great package manager for C.
- Python and Java aren't self contained, aren't necessarily deployed on the server, are more heavy weight, and I suspect that maintenance plays second fiddle to the C api.
- The exec plugin is costly as it creates a new process for every collection
- Depending on the circumstances, writing to a unix socket could be good fit, but I enjoy the ease of deployment, and the collectd integration -- there's no need to re-invent logging scheme, configuration, and system init files.

Rust's combination of ecosystem, package manager, C ffi, single file dynamic library, and optimized code made it seem like a natural choice.

## To Build

To ensure a successful build, adapt the below to your project's Cargo file.

```toml
[lib]
crate-type = ["cdylib"]
name = "<your plugin name>"

[features]
bindgen = ["collectd-plugin/bindgen"]
default = []
```

- A collectd version is required. You can specify environment variable `COLLECTD_VERSION` as `5.4`, `5.5`, or `5.7`, or rely on `collectd-rust-plugin` auto detecting the version by executing `collectd -h`.
- The bindgen feature is optional (it will re-compute the Rust bindings from C code, which shouldn't be necessary). Make sure you have an appropriate version of clang installed and `collectd-dev`
- Collectd expects plugins to not be prefixed with `lib`, so `cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so`
- Add `LoadPlugin myplugin` to collectd.conf

## Plugin Configuration

The load plugin in
[examples/load](https://github.com/nickbabcock/collectd-rust-plugin/tree/master/examples/load.rs)
demonstrates how to expose configuration values to Collectd.

```xml
# In this example configuration we provide short and long term load and leave
# Mid to the default value. Yes, this is very much contrived
<Plugin loadrust>
    ReportRelative true
</Plugin>
```

## Benchmarking Overhead

To measure the overhead of adapting Collectd's datatypes when writing and reporting values:

```bash
cargo bench --features stub
```

If you'd like to use the timings on my machine:

- 100ns to create and submit a `ValueListBuilder`
- 150ns to create a `ValueList` for plugins that write values

Unless you are reporting or writing millions of metrics every interval (in which case you'll most likely hit an earlier snap), you'll be fine.
