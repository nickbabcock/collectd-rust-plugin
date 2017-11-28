#!/bin/bash

set -euo pipefail

source $HOME/.cargo/env
cargo build --features $VERSION
cargo test --features $VERSION
cargo test-junit --name TestResults --features $VERSION

cargo build --features "$VERSION bindgen"
cargo test --features "$VERSION bindgen"
cargo test-junit --name TestResults-bindgen --features "$VERSION bindgen"

cp target/debug/libmyplugin.so /usr/lib/collectd/myplugin.so

cat <<EOF | tee /etc/collectd/collectd.conf
Hostname "localhost"
LoadPlugin myplugin
LoadPlugin csv
<Plugin csv>
  DataDir "/var/lib/collectd/csv"
  StoreRates false
</Plugin>
<Plugin myplugin>
  Short "2"
  Long "5.5"
</Plugin>
EOF

service collectd start
sleep 15
service collectd status

grep '2.000000,10.000000,5.500000' /var/lib/collectd/csv/localhost/myplugin/load*
exit $?
