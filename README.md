[![ci](https://github.com/nickbabcock/collectd-rust-plugin/actions/workflows/ci.yml/badge.svg)](https://github.com/nickbabcock/collectd-rust-plugin/actions/workflows/ci.yml) [![](https://docs.rs/collectd-plugin/badge.svg)](https://docs.rs/collectd-plugin) [![Version](https://img.shields.io/crates/v/collectd-plugin.svg?style=flat-square)](https://crates.io/crates/collectd-plugin)

# Write a Collectd Plugin in Rust

[Collectd](https://collectd.org/) is a ubiquitous system statistics collection
daemon. This Rust library leverages collectd's ability to dynamically load
plugins and creates an ergonomic, yet extremely low cost abstraction API to
interface with collectd. Works with collectd 5.7+.

Features:

- No unnecessary allocations when submitting / receiving values, logging
- Register multiple plugin instances
- Automatic deserialization of plugin configs via [Serde](https://github.com/serde-rs/serde) (can opt-out)
- Deployment: compile against collectd version and scp to server
- Referenced Rust libraries are statically linked
- Help writing thread safe plugins thanks to the Rust compiler

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
collectd-plugin = "0.14.0"
```

[Serde](https://github.com/serde-rs/serde) support is enabled by default for configuration parsing.

## Quickstart

[See what to add to your project's Cargo file](#to-build)

Below is a complete plugin that dummy reports [load](https://en.wikipedia.org/wiki/Load_(computing)) values to collectd, as it registers a `READ` hook. For an implementation that reimplements collectd's own load plugin, see [examples/load](https://github.com/nickbabcock/collectd-rust-plugin/tree/master/examples/load.rs)

```rust
use collectd_plugin::{
    collectd_plugin, ConfigItem, Plugin, PluginCapabilities, PluginManager, PluginRegistration,
    Value, ValueListBuilder,
};
use std::error;

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
    fn plugins(
        _config: Option<&[ConfigItem<'_>]>,
    ) -> Result<PluginRegistration, Box<dyn error::Error>> {
        Ok(PluginRegistration::Single(Box::new(MyPlugin)))
    }
}

impl Plugin for MyPlugin {
    // We define that our plugin will only be reporting / submitting values to writers
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&self) -> Result<(), Box<dyn error::Error>> {
        // Create a list of values to submit to collectd. We'll be sending in a vector representing the
        // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
        // of values that you submit at a time depends on types.db in collectd configurations
        let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

        // Submit our values to collectd. A plugin can submit any number of times.
        ValueListBuilder::new(Self::name(), "load")
            .values(&values)
            .submit()?;

        Ok(())
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

- `collectd-rust-plugin` assumes a `5.7`-compatible API (`5.7` works up to at least `5.12`). This can be configured via any of the following:
  - Specify the `bindgen` feature with `COLLECTD_PATH` pointing at the [root git directory for collectd](https://github.com/collectd/collectd)
    - The `bindgen` feature also works if `collectd-dev` is installed
  - `COLLECTD_VERSION` may be used in the future when `collectd-rust-plugin` reintroduces support for compiling against different collectd versions that are not API-compatible
  - When collectd is present on the system, the version is derived from executing `collectd -h`
- collectd expects plugins to not be prefixed with `lib`, so `cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so`
- Add `LoadPlugin myplugin` to collectd.conf

## Plugin Configuration

The load plugin in
[examples/load](https://github.com/nickbabcock/collectd-rust-plugin/tree/master/examples/load.rs)
demonstrates how to expose configuration values to collectd.

```xml
# In this example configuration we provide short and long term load and leave
# Mid to the default value. Yes, this is very much contrived
<Plugin loadrust>
    ReportRelative true
</Plugin>
```

## Benchmarking Overhead

To measure the overhead of adapting collectd's datatypes when writing and reporting values:

```bash
cargo bench --features stub
```

If you'd like to use the timings on my machine:

- 60ns to create and submit a `ValueListBuilder`
- 130ns to create a `ValueList` for plugins that write values

Unless you are reporting or writing millions of metrics every interval (in which case you'll most likely hit an earlier bottleneck), you'll be fine.

## Plugins

Do you use collectd-rust-plugin? Feel free to add your plugin to the list.

- [pg-collectd](https://github.com/nickbabcock/pg-collectd): An alternative and opinionated postgres collectd writer
