#!/bin/bash

set -euo pipefail

ci/setup.sh

if [[ "${VERSION}" != "collectd-57" ]]; then
    wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -
fi

apt-get install -y llvm-3.9-dev libclang-3.9-dev clang-3.9

if [[ "${VERSION}" == "collectd-54" ]]; then
    cp -r /usr/include/collectd/liboconfig /usr/include/collectd/core/.
fi

ci/test.sh
