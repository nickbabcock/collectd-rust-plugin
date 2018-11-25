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
EOF

service collectd start
sleep 15
service collectd status
service collectd stop

grep_test() {
    echo grep "$1" "$2"
    EXIT_CODE=0

    # With set -e, we don't want to exit immediately, but instead add context
    # of what grep failed, so we force the command to succeed while capturing
    # the failing command's exit status:
    # https://stackoverflow.com/a/45729843/433785
    grep "$1" "$2" || EXIT_CODE=$? && true
    if [[ $EXIT_CODE -ne 0 ]]; then
        echo "Not found: $1 in $2"
        echo "contents of $2:"
        cat "$2"
    fi
    return $EXIT_CODE
}

grep_test 'epoch,shortterm,midterm,longterm' /var/lib/collectd/csv/localhost/loadrust/load*
grep_test 'collectd logging configuration: Some' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: rust logging configuration: Some' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: flushing: timeout: no timeout, identifier: no identifier' /var/lib/collectd/log
grep_test 'testwriteplugin: write_log: yes drop is called' /var/lib/collectd/log
exit $?
