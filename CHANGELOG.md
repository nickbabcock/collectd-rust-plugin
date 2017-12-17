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
