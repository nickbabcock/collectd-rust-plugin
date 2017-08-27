# A Collectd Plugin Written in Rust

This repo demonstrates how to create a Collectd plugin written in Rust that uses [bindgen](https://github.com/rust-lang-nursery/rust-bindgen) to generate the ffi functions and an ergonomic rust structure ontop of `value_list_t`. Currently, the example is broken and has been [reported in the collectd repo](https://github.com/collectd/collectd/issues/2409) for further investigation.

Currently this plugin needs nightly so that bindgen generates Rust unions (but no Rust nightly features are used).

## To Build

```bash
# Install collectd library so that rust bindgen works
apt install collectd-dev

# Install rust toolchain (not needed if already installed)
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly

# Build the library
cargo build

# Copy plugin (and rename it) to plugin directory
cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so

# Add "LoadPlugin myplugin" to collectd.conf
```
