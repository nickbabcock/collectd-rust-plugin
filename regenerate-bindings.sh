#!/bin/bash
set -euo pipefail

generate () {
    echo $1 $2 $3
    docker build -t collectd-rust-plugin --build-arg UBUNTU_VERSION=$1 --build-arg COLLECTD_VERSION=$2 .
    docker run --rm collectd-rust-plugin bash -c "
        source ~/.cargo/env &&
        rustup component add rustfmt 2>/dev/null &&
        cargo --quiet install bindgen &&
        cd /tmp &&
        bindgen --rust-target 1.21 \
            --whitelist-type cdtime_t \
            --whitelist-type data_set_t \
            --whitelist-function 'plugin_.*' \
            --whitelist-function 'uc_get_rate' \
            --whitelist-var 'OCONFIG_TYPE_.*' \
            --whitelist-var 'LOG_.*' \
            --whitelist-var 'DS_TYPE_.*' \
            --whitelist-var DATA_MAX_NAME_LEN \
            wrapper.h -- -DHAVE_CONFIG_H -DCOLLECTD_$3" > src/bindings-$3.rs
}

generate 14.04 5.4 54
generate 16.04 5.5 55
generate 18.04 5.7 57

