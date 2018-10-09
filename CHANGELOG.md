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
