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

<Plugin write_log>
    LogTimings "INFO"
</Plugin>
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
service collectd stop

grep_test() {
    echo "looking for $1 in $2"
    grep $1 $2
}

grep_test 'epoch,shortterm,midterm,longterm' /var/lib/collectd/csv/localhost/loadrust/load*
grep_test 'collectd logging configuration: Some' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: rust logging configuration: Some' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: flushing: timeout: no timeout, identifier: no identifier' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: yes drop is called' /var/lib/collectd/log
exit $?
