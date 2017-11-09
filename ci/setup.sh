#!/bin/bash

set -euo pipefail

apt update
apt install -y --no-install-recommends collectd collectd-dev
apt install -y wget curl build-essential
curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.cargo/env
cargo install cargo-test-junit
