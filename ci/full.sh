#!/bin/bash

set -euo pipefail

ci/setup.sh

if [[ "${COLLECTD_VERSION}" != "5.7" ]]; then
    wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -
fi

apt-get install -y llvm-3.9-dev libclang-3.9-dev clang-3.9

if [[ "${COLLECTD_VERSION}" == "5.4" ]]; then
    cp -r /usr/include/collectd/liboconfig /usr/include/collectd/core/.
fi

ci/test.sh
