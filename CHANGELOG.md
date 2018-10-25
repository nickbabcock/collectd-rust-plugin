## Unreleased - TBA

This is a breaking change that is going to affect everyone. It's a bummer, but hopefully by the end of this blurb, you will be convinced that the breaking change is worth it.

### Thread Safety

For background, collectd has the right to call any of the `Plugin` methods in parallel and concurrently. Previously, a `Plugin` only needed to be `Sync` (a plugin is safe to have its references shared between threads), but this allowed for undefined behavior with code that isn't thread safe. The compiler only allowed it as transcended `unsafe` (ffi) boundaries. We can do better. `Send` is now required (so a plugin can be transferred across thread boundaries). We don't control collectd and how they manage our Plugin, so if collectd offloads our plugin to another thread, the plugin functionality must remain the same.

Also all `Plugin` methods now force interior mutability by changing `&mut self` to `&self` to ensure thread safety in conjunction with the `Sync + Send` requirement. For instance, `Vec<u8>` is `Sync + Send` but one can't `Vec::push` across multiple threads, so synchronization primitives will now be required if mutability is desired.

Everything is more clear with examples. Here is the error one will receive if they attempt to mutate something that is not thread safe (this code was previously allowed)

```rust
#[derive(Debug, Default)]
pub struct MyPlugin {
    names: HashSet<String>,
}

impl Plugin for MyPlugin {
    fn read_values(&self) -> Result<(), Error> {
        self.names.insert(String::from("A"));
    }
}
```

The compiler error:

```
error[E0596]: cannot borrow field `self.names` of immutable binding as mutable
   --> src/plugin.rs:102:9
    |
97  |     fn read_values(&self) -> Result<(), Error> {
    |                    ----- use `&mut self` here to make mutable
...
102 |         self.names.insert(String::from("A"));
    |         ^^^^^^^^^^ cannot mutably borrow field of immutable binding
```

The fix is to use a synchronization primitive like a `Mutex`, which will allow one to mutate the inner data by taking an exclusive lock on the data.

```rust
#[derive(Debug, Default)]
pub struct MyPlugin {
    names: Mutex<HashSet<String>>,
}

impl Plugin for MyPlugin {
    fn read_values(&self) -> Result<(), Error> {
        let mut n = self.names.lock().unwrap();
        n.insert(String::from("A"));
    }
}
```

Now if collectd happens to call `read_values` in parallel, there is no way we would stumble into undefined behavior, as the mutex ensures that thread B waits until thread A is done with the data (`names` in this instance). `Mutex` may not be right for you, so make sure employ correct synchronization primitives

As a final demostration of why the changes were necessary the following should not compile:

```rust
#[derive(Debug, Default)]
pub struct MyPlugin {
    names: RefCell<HashSet<String>>,
}

impl Plugin for MyPlugin {
    fn read_values(&self) -> Result<(), Error> {
        let mut n = self.names.borrow_mut();
        n.insert(String::from("A"));
    }
}
```

`RefCell` does not implement `Sync`. [From the Rust book](https://doc.rust-lang.org/book/second-edition/ch15-05-interior-mutability.html)

> only for use in single-threaded scenarios and will give you a compile-time error if you try using it in a multithreaded context

But it did previously compile! This hints that we were not setting up enough context for the compiler to ensure code is threadsafe.

Thank you to the [Rust FFI Guide](https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html#setting-up-plugins) for inspiration on this bugfix.

### Calculating Rates

Many plugins that write out collectd values (write_graphite, write_tsdb, write_mongodb) contain an option called `StoreRates`, which is described with:

> Controls whether DERIVE and COUNTER metrics are converted to a rate before sending

If one is creating a plugin that writes, it is also a good idea to expose a `StoreRates` configuration option, so that users familiar with `StoreRates` behavior can migrate seamlessly -- else users will receive a shock when they see accumulated values (eg: total number of bytes sent on an interface instead of bytes per second). Prior to this release it was too cumbersome for one to reasonably calculate rates. This all changes with the availability of `ValueList::rates`, which will delegate to collectd's `uc_get_rate`

```rust
fn write_values(&self, list: ValueList) -> Result<(), Error> {
    // if the user configured `StoreRates` then, as needed, convert the given values to rates
    let values = if self.store_rates {
        list.rates()
    } else {
        Ok(::std::borrow::Cow::Borrowed(&list.values))
    }?;

    // do something with values
}
```

The reason why rates returns a `Cow<Vec<_>>` is because if all values are already `Value::Gauge` then there is no work to be done (or allocations needed), so it provides an optimisation.

### Other Changes

- Update bindgen requirement from 0.42.0 to 0.43.0
- Update failure requirement from 0.1.2 to 0.1.3
- Implement `Value::is_nan` to determine if the inner value is not a number.
- Implement `Serialize` for collectd Value. Note this serialization cannot be completed roundtrip (eg: serialize a Value and then deserialize it), as the inner value is directly serialized without additional type information. This is intended for writers outputting values for csv, postgres, etc.

## 0.7.0 - 2018-10-11

I know it's a little unheard for two release two minor versions to be so close to each other, but an important integration has been added. Users can opt in to have [`log`](https://docs.rs/log) statements forwarded to collectd's logger.

Here is the recommended way to log:

*Before*:

```rust
let line = format!("collectd logging!");
collectd_log(LogLevel::Info, &line);
```

*After*:

```rust
info!("collectd logging!");
```

To opt into this feature, utilize the `CollectdLoggerBuilder` to register the logger in a `PluginManager::plugins`.

```rust
#[derive(Default)]
struct MyPlugin;
impl PluginManager for MyPlugin {
    fn name() -> &'static str {
        "myplugin"
    }

    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
       CollectdLoggerBuilder::new()
           .prefix_plugin::<Self>()
           .filter_level(LevelFilter::Info)
           .try_init()
           .expect("really the only thing that should create a logger");
        unimplemented!()
    }
}
```

`CollectdLoggerBuilder` will look and feel very similar to [`env_logger`](https://docs.rs/env_logger).

The motivation for this feature came when I realized that in one of my collectd plugins, a dependency was logging an error but since no logger was setup, the message was discarded.

What's happening to `collectd_log`? Nothing right now. While it still has some uses, most should prefer using rust's native logging for a performance, ergonomic, and debugging win. If one needs to circumvent a potential rust logging filter, `collectd_log` is available.

## 0.6.1 - 2018-10-09

- Globalize module paths found in `collectd_plugin!` macro. Previously the macro only worked if the `PluginManager` was defined in the same module as `collectd_plugin!` usage (or if one included the necessary imports used internally). This inflexibility was not conducive to organizing larger collectd plugins.
- Update bindgen from 0.41.0 to 0.42.0

## 0.6.0 - 2018-10-03

- Bump failure dependency to 0.1.2
- Enable `serde` feature by default, as most plugins will have some sort of configuration, and the recommended course of action is to enable the serde feature. Instead of forcing users to hop through another step on their way to writing a collectd plugin, make `serde` feature enabled by default.

## 0.5.3 - 2018-06-20

No functionality changed in this release -- more like cleanup for those who received clippy warnings using collectd-plugin or like it when a library remove `unsafe` usages!

- `PluginCapabilities` takes `self` by value instead of by reference (clippy lint)
- `collectd_plugin!` macro no longer references `cfg(collectd57)`, which while set in the collectd-plugin's build.rs doesn't affect the downstream user. The only reason why `collectd_plugin!` referenced `cfg(collectd57)` was to switch up mutability of a parameter that changed in Collectd 5.7. Since the mutability of the parameter does not change the behavior, a swath of code was eliminated and the clippy lint ignored.
- Shrank the surface area of `unsafe` code:
  - Global static mutable boolean replaced with `AtomicBoolean`. This necessitated a move to require a minimum rust version of 1.24.0.
  - Prefer pointer casts instead of `transmute`
  - Remove `u32` to `LogLevel` via `transmute` instead there is a `LogLevel::try_from`
  - Instead of wrapping functions in `unsafe`, wrap the one or two statements that need unsafe.

## 0.5.2 - 2018-05-15

Another attempt to have documentation displayed correctly.

## 0.5.1 - 2018-05-15

* Update documentation. Since 0.5.0, the `COLLECTD_VERSION` environment variable needs to be supplied, or cargo heuristically determines the installed collectd package. In the case where neither condition applies, the build will fail. When docs.rs generates the documentation, it failed both conditions, so the docs didn't build. The fix was to tweak `Cargo.toml` to provide the needed arguments in docs.rs metadata.

## 0.5.0 - 2018-04-16

**Breaking Change**: Replace collectd cargo features with env variable. The `COLLECTD_VERSION` environment variable takes precedence, but if missing, `collectd_plugin` will attempt to autodetect the version by executing `collectd -h`. By going this route, we can ensure several invariants at build time:

- That the package builds. Previously, the `default` feature set would fail compilation with not intuitive error message. Now one has to supply the version or rely on autodetection.
- That one can't combine features, like specifying `collectd-54` and `collectd-57` at the same time. They are now mutually exclusive.
- I can now execute `cargo package` without running with `--no-verify` as `cargo package` doesn't allow feature selection (somewhat understandably).
- Create a compliant `examples/` directory for examples!

Valid `COLLECTD_VERSION` variables:

- `5.4`
- `5.5`
- `5.7`

## 0.4.4 - 2018-04-10

* Add serde deserialization for `LogLevel`
* Add `collectd_log_raw!` macro for lower level log formatting using `printf` formatting

## 0.4.3 - 2018-03-09

* Fix conversion from datetime to cdtime. This will fix those who set the time
  in a `ValueListBuilder` and receive a "uc_update: Value too old: name ="
  error in the logs

## 0.4.2 - 2018-03-08

* Errors now have all their causes concatenated (semicolon delimited) when logged instead of just the head cause
* Overhead of submitting values via `ValueListBuilder` reduced in half to ~100ns

## 0.4.1 - 2018-01-27

* (Breaking change) rename `RecvValueList` to `ValueList`
* Export `ValueReport` as part of API
* Avoid allocations for logging plugins
* Force `Plugin` implementations to implement `Sync`
* Add a example `write_graphite` plugin

## 0.4.0 - 2018-01-26

* Reduce pre-computed bindings with whitelisted types
* Improve serde deserialization of multi-keys
* Change deserialization return type from an alias of `Result` to `DeResult`

## 0.3.0 - 2017-12-17

* (Breaking change): Switch `collectd_plugin!` away from lazy_static mutex
* Preliminary Serde support for deserializing collectd configs
* Update `ValueListBuilder` to accept static string references instead of just owned strings to reduce unnecessary allocations
* Update `ValueListBuilder` to take a slice of values to submit instead of a vector
* Add several example plugins to the repo
* Add plugin hook for plugin initialization
* Add plugin hook for plugin log
* Add plugin hook for plugin write

## 0.2.0 - 2017-11-30

This is the initial release of this library on [crates.io as collectd-plugin](https://crates.io/crates/collectd-plugin)
