#!/bin/bash

set -euo pipefail

source $HOME/.cargo/env
cargo build --all --features 'serde'
cargo test --all --features 'serde'

cargo build --all --features "serde bindgen"
cargo test --all --features "serde bindgen"

cp target/debug/examples/libloadrust.so /usr/lib/collectd/loadrust.so

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
