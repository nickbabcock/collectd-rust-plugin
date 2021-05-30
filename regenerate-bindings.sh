#!/bin/bash
set -euo pipefail

generate () {
    echo $1 $2 $3
    docker build -t collectd-rust-plugin --build-arg UBUNTU_VERSION=$1 --build-arg COLLECTD_VERSION=$2 .
    docker run --rm collectd-rust-plugin bash -c "
        source ~/.cargo/env &&
        rustup component add rustfmt 2>/dev/null &&
        cd /tmp &&
        COLLECTD_OVERWRITE=1 cargo build --features bindgen >/dev/null &&
        cat src/bindings-$3.rs" > src/bindings-$3.rs
}

generate 18.04 5.7 57

