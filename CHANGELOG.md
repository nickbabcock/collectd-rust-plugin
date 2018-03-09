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
