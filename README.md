# A Collectd Plugin Written in Rust

This repo demonstrates how to create a Collectd plugin written in Rust that uses [bindgen](https://github.com/rust-lang-nursery/rust-bindgen) to generate the ffi functions and an ergonomic rust structure on top of `value_list_t`.

Rust 1.19 or later is needed to build.

This plugin demonstrates how to expose values to collectd (in this case, it's
[load](https://en.wikipedia.org/wiki/Load_(computing))) using contrived numbers
that can be overridden using the standard collectd config:

```xml
# In this example configuration we provide short and long term load and leave
# Mid to the default value. Yes, this is very much contrived
<Plugin myplugin>
    Short "2"
    Long "5.5"
</Plugin>
```

## To Build

```bash
# Install collectd library so that rust bindgen works
apt install collectd-dev

# If you are not on ubuntu 16.10 or later, a recent clang version is required
# wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key|sudo apt-key add -
# apt-get install llvm-3.9-dev libclang-3.9-dev clang-3.9

# Install rust toolchain (not needed if already installed)
curl https://sh.rustup.rs -sSf | sh -s

# Build the library
cargo build

# Copy plugin (and rename it) to plugin directory
cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so

# Add "LoadPlugin myplugin" to collectd.conf
```
