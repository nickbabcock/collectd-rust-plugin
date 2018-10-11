#!/bin/bash

set -euo pipefail

source $HOME/.cargo/env
cargo test --all
cargo test --all --no-default-features
cargo test --all --features "bindgen"

cp target/debug/examples/libloadrust.so /usr/lib/collectd/loadrust.so
cp target/debug/examples/libwrite_log.so /usr/lib/collectd/write_log.so

cat <<EOF | tee /etc/collectd/collectd.conf
Hostname "localhost"
LoadPlugin loadrust
LoadPlugin write_log
LoadPlugin csv
LoadPlugin logfile

<Plugin logfile>
    LogLevel info
    File "/var/lib/collectd/log"
</Plugin>
<Plugin csv>
  DataDir "/var/lib/collectd/csv"
  StoreRates false
</Plugin>
<Plugin loadrust>
</Plugin>
<Plugin write_log>
</Plugin>
EOF

service collectd start
sleep 15
service collectd status

grep 'epoch,shortterm,midterm,longterm' /var/lib/collectd/csv/localhost/loadrust/load*
grep 'collectd logging configuration: None' /var/lib/collectd/log
grep 'testwriteplugin: write_log: rust logging configuration: None' /var/lib/collectd/log
exit $?
