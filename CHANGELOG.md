## 0.15.0 - 2024-08-06

- Fix null collectd values causing a panic
- Fix 32-bit compilation errors
- Prefer `SeqCst` ordering in abundance of caution
- Update dependencies:
  - Bindgen from 58.1 to 69.1
  - Bitflags from 1.0 to 2.4
  - env_logger from 0.8 to 0.10

## 0.14.0 - 2021-06-15

- Submit and receive metadata values through `ValueListBuilder::metadata` and `ValueList::meta` respectively
- Only collectd 5.7+ is supported (ie: 5.4 and 5.5 API versions have been dropped). This means that `COLLECTD_VERSION` has a default fallback and is no longer required.
- Collectd 5.12 now supported
- Some error structs migrated to named fields and away from tuple structs
- `env_logger` updated to 0.8

## 0.13.0 - 2020-05-09

- Add `PluginManager::shutdown` to clean up resources allocated in `PluginManager::initialize`
- Expose additional public error types:
  - ConfigError
  - CacheRateError
  - ReceiveError
  - SubmitError

## 0.12.0 - 2020-04-25

* Disable additional logging features by default. The only env_logger feature related to filtering (what collectd-plugin enables) is the regex feature, so that feature can be enabled through the `regex_log_filter` feature.  The rest of disabled to keep dependencies to a minimum
* Allow autodetection of collectd version to work with 5.10 and 5.11
* Update bindgen requirement from 0.51.0 to 0.53.1

## 0.11.0 - 2019-10-30

* Support collectd-5.9 through 5.7 interface (so `COLLECTD_VERSION=5.7` can bind to collectd-5.9)
* Upgrade to env_logger 0.7 from 0.6

## 0.10.0 - 2019-08-02

* Bump to 2018 edition of rust
* Bump minimum required rust to 1.33
* Add additional include path when linking against a custom collectd version
* Bindgen updated from 0.47.0 to 0.51.0

## 0.9.1 - 2019-02-04

- Compile on non-x86 platforms
- Add `COLLECTD_PATH` environment variable for detecting collectd version from collectd's source directory (most useful with the `bindgen` feature).
- Output panic info into collectd logs

## 0.9.0 - 2018-12-12

Big release with a couple backwards incompatible changes. Let's break it down.

### Removing the `failure` crate

#### The why

While initially promising, `failure` seems to be too heavy-handed of a requirement to force onto the user. There seems to be some sort of consensus that failure should be used for libraries.

References:

- [Do I really need failure/error-chain?](https://www.reddit.com/r/rust/comments/8lt8k6/do_i_really_need_failureerrorchain/)
- [Current state of error handling in Rust?](https://www.reddit.com/r/rust/comments/9m5w9a/current_state_of_error_handling_in_rust/)
- [Redefining Failure](https://epage.github.io/blog/2018/03/redefining-failure/)
- [Goodbye failure](https://paulkernfeld.com/2018/10/27/improving-ndarray-csv.html)

#### The fix

All `failure::Error` should be replaced with `Box<error::Error>`. A change in the type signature may seem scary but the work entailed to migrate should be minimal. For instance, here is the (condensed) diff for migrating the readme example to the new format.

```diff
- extern crate failure;
- use failure::Error;
+ use std::error;

-   fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Error> {
+   fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Box<error::Error>> {

-   fn read_values(&self) -> Result<(), Error> {
+   fn read_values(&self) -> Result<(), Box<error::Error>> {
```

Removing the `failure` crate cut compile times and the dependency tree in half.

### Plugins implement `UnwindSafe + RefUnwindSafe`

#### The why

> It is currently undefined behavior to unwind from Rust code into foreign code, so [catching panics] is useful when Rust is called from another language (normally C). This can run arbitrary Rust code, capturing a panic and allowing a graceful handling of the error. [[source]](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)

Panics can be caused by silly mistakes like

```rust
let a = Instant::now();
// ...
a.duration_since(Instant::now()); // will panic!
```

This would cause a SIGABRT in collectd, and no one likes a crashing program.

#### The fix

By forcing `Plugin` to be `UnwindSafe + RefUnwindSafe`, we can catch panics before they cross the ffi border. Instead of crashing, collectd will log an error. Most plugins should already be `UnwindSafe + RefUnwindSafe`.

### Error Messages

Collectd plugin is doubling down on Rust's native logging. Now, if registered, all errors (even if they originated in between collectd and the plugin), will use prefer Rust's logging. If not registered, errors are instead sent directly to collectd and may lack context.

Given the following plugin that alternates between panicking and returning an error:

```rust
#[derive(Default)]
struct MyErrorManager;

#[derive(Default)]
struct MyErrorPlugin {
    state: AtomicBool,
}

impl PluginManager for MyErrorPlugin {
    fn name() -> &'static str {
        "myerror"
    }

    fn plugins(_config: Option<&[ConfigItem]>) -> Result<PluginRegistration, Box<error::Error>> {
        CollectdLoggerBuilder::new()
            .prefix_plugin::<Self>()
            .filter_level(LevelFilter::Info)
            .try_init()
            .expect("really the only thing that should create a logger");

        Ok(PluginRegistration::Single(Box::new(
            MyErrorPlugin::default(),
        )))
    }
}

impl Plugin for MyErrorPlugin {
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::READ
    }

    fn read_values(&self) -> Result<(), Box<error::Error>> {
        if self.state.fetch_xor(true, Ordering::SeqCst) {
            panic!("Oh dear what is wrong!?")
        } else {
            Err(failure::err_msg("bailing"))?
        }
    }
}
```

One will find the following lines in the log:

```
myerror: collectd_plugin::api::logger: read error: plugin encountered an error; bailing
read-function of plugin `myerror' failed
myerror: collectd_plugin::api::logger: read error: plugin panicked
```

Nested causes will be semi-colon joined to be printed on a single line.

Collectd will back off exponentially on failing plugins.

### Other changes

* Migrate most of `collectd_plugin!` macro internally. This reduces the number lines in the macro from 300 to 30
* All of the internal logic used in `collectd_plugin!` is no longer exposed as a public API, so this drastically curtails the public API. If part of the API was removed that you depended upon, open an issue to discuss. Do not depend on anything in the `internal` module
* Guard against null messages panicking logging plugins (though null messages should never happen)

## 0.8.4 - 2018-11-25

* Add: serde config deserialize newtype structs
* Add: serde config deserialize unit variant enums (like `log` crate `LogLevel`)
* Improve error message on build failure to suggest providing the `COLLECTD_VERSION` environment variable if missing
* Simplify rust logging delegation by cutting out `Cursor` in preference of writing a `Vec<u8>` directly. Could yield a small performance increase for log heavy workloads.
* Update env_logger from 0.5 to 0.6 (no behavioral changes)

## 0.8.3 - 2018-11-05

* Fix double free segfault on shutdown for plugins that register more than on callback (often times write + flush). `collectd-rust-plugin` delegates to a plugin's drop implementation on shutdown to make sure all resources are cleaned up. The previous behavior would have collectd calling a plugin's drop implementation for each callback register. Dropping the same plugin twice is undefined behavior and would often segfault. Collectd [recommends](https://collectd.org/wiki/index.php/User_data_t) only registering the drop function once to avoid a double free scenario. `collectd-rust-plugin` now understands that when multiple callbacks are desired, to only execute drop on one of them.

## 0.8.2 - 2018-11-04

* Fix segfault on plugins that implement flush when given a null identifier. The proper behavior now includes a check to see if the identifier is `NULL` (and convert it to an `Option` appropriately) before interpretting it as a `str`.

## 0.8.1 - 2018-10-30

* Compatibility with collectd 5.8. Collectd 5.8 changed the signature of `hostname_g` so it can no longer be submitted as a host value. We allowed it and resulted in collectd interpretting garbage. The fix is to switch to submitting empty arrays for collectd 5.7+, which will default to `hostname_g` internally.

## 0.8.0 - 2018-10-25

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
