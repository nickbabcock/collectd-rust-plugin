# A Collectd Plugin Written in Rust

This repo demonstrates how to create a Collectd plugin written in Rust that uses [bindgen](https://github.com/rust-lang-nursery/rust-bindgen) to generate the ffi functions. If you want to write a collectd plugin start with this repo as it defines common functions and provides an ergonomic Rust structure on top of `value_list_t`.

Rust 1.19 or later is needed to build.

This repo is tested on the following:

- Collectd 5.4 (Ubuntu 14.04)
- Collectd 5.5 (Ubuntu 16.04)
- Collectd 5.7 (Ubuntu 17.04)

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
