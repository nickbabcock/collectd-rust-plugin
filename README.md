# A Collectd Plugin Written in Rust

This repo demonstrates how to create a Collectd plugin written in Rust that uses [bindgen](https://github.com/rust-lang-nursery/rust-bindgen) to generate the ffi functions. If you want to write a collectd plugin start with this repo as it defines common functions and provides an ergonomic Rust structure on top of `value_list_t`.

Rust 1.19 or later is needed to build.

This repo is tested on the following:

- Collectd 5.4 (Ubuntu 14.04)
- Collectd 5.5 (Ubuntu 16.04)
- Collectd 5.7 (Ubuntu 17.04)

## Quickstart

```rust
#[no_mangle]
pub extern "C" fn module_register() {
    // The entry function for our plugin. Our registered read function will be called at
    // intervals defined by collectd
    let s = CString::new("myplugin").unwrap();
    unsafe {
        plugin_register_read(s.as_ptr(), Some(my_read));
    }
}

#[no_mangle]
pub extern "C" fn my_read() -> c_int {
    // Create a list of values to submit to collectd. We'll be sending in a vector representing the
    // "load" type. Short-term load is first (15.0) followed by mid-term and long-term. The number
    // of values that you submit at a time depends on types.db in collectd configurations
    let values = vec![Value::Gauge(15.0), Value::Gauge(10.0), Value::Gauge(12.0)];

    // Submit our values to collectd. A plugin can submit any number of times.
    let submission = ValueListBuilder::new("myplugin", "load")
        .values(values)
        .submit();

    // If collectd submission failed return a -1. Collectd will backoff calling
    // our plugin. See `lib.rs` for examples on error logging.
    if submission.is_ok() { 0 } else { -1 }
}
```

## To Build

After cloning this repo, you'll need to ensure that a few dependencies are satisfied. Don't worry these aren't needed on the deployed server.

```bash
# Install collectd library so that rust bindgen works.
apt install collectd-dev

# If you are not on ubuntu 16.10 or later, a recent clang version is required
# wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key|sudo apt-key add -
# apt-get install llvm-3.9-dev libclang-3.9-dev clang-3.9

# Must supply the version of collectd you're building against
cargo build --features collectd-54

# Copy plugin (and rename it) to plugin directory as Collectd assumes a
# standard naming convention
cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so

# Add "LoadPlugin myplugin" to collectd.conf
```

## Plugin Configuration

This plugin demonstrates how to expose configuration values to Collectd (in
this case, it's [load](https://en.wikipedia.org/wiki/Load_(computing))) using
contrived numbers that can be overridden using the standard Collectd config:

```xml
# In this example configuration we provide short and long term load and leave
# Mid to the default value. Yes, this is very much contrived
<Plugin myplugin>
    Short "2"
    Long "5.5"
</Plugin>
```
