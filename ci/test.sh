#!/bin/bash

set -euo pipefail

source $HOME/.cargo/env
cargo build --all --features $VERSION
cargo test --all --features $VERSION
cargo test-junit --name TestResults --features $VERSION

cargo build --all --features "$VERSION bindgen"
cargo test --all --features "$VERSION bindgen"
cargo test-junit --name TestResults-bindgen --features "$VERSION bindgen"

cp target/debug/libloadrust.so /usr/lib/collectd/loadrust.so

cat <<EOF | tee /etc/collectd/collectd.conf
Hostname "localhost"
LoadPlugin loadrust
LoadPlugin csv
<Plugin csv>
  DataDir "/var/lib/collectd/csv"
  StoreRates false
</Plugin>
<Plugin loadrust>
</Plugin>
EOF

service collectd start
sleep 15
service collectd status

grep 'epoch,shortterm,midterm,longterm' /var/lib/collectd/csv/localhost/loadrust/load*
exit $?
