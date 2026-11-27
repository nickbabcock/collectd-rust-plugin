#!/bin/bash

set -euo pipefail

cp target/debug/examples/libloadrust.so /usr/lib/collectd/loadrust.so
cp target/debug/examples/libwrite_logrs.so /usr/lib/collectd/write_logrs.so
cp target/debug/examples/libmyerror.so /usr/lib/collectd/myerror.so
cp target/debug/examples/liblifecycle.so /usr/lib/collectd/lifecycle.so
cp target/debug/examples/libload_with_metadata.so /usr/lib/collectd/load_with_metadata.so

cat <<EOF | tee /etc/collectd/collectd.conf
Hostname "localhost"
LoadPlugin loadrust
LoadPlugin write_logrs
LoadPlugin csv
LoadPlugin logfile
LoadPlugin myerror
LoadPlugin lifecycle
LoadPlugin load_with_metadata

<Plugin logfile>
    LogLevel info
    File "/var/lib/collectd/log"
</Plugin>
<Plugin write_logrs>
    LogTimings "INFO"
</Plugin>
<Plugin csv>
  DataDir "/var/lib/collectd/csv"
  StoreRates false
</Plugin>
<Plugin loadrust>
</Plugin>
EOF

systemctl restart collectd
sleep 25
systemctl status collectd
systemctl stop collectd

# A crash during shutdown shows up as a non-zero exit status
EXIT_STATUS=$(systemctl show collectd -p ExecMainStatus --value)
if [[ "$EXIT_STATUS" != "0" ]]; then
    echo "collectd exited with status $EXIT_STATUS"
    cat /var/lib/collectd/log
    exit 1
fi

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
grep_test 'A raw log with argument: 10' /var/lib/collectd/log
grep_test 'A raw log with arguments: 10 20' /var/lib/collectd/log
grep_test 'collectd logging configuration: Some' /var/lib/collectd/log
grep_test 'write_logrs: write_logrs: rust logging configuration: Some' /var/lib/collectd/log
grep_test 'write_logrs: write_logrs: percent check: 100%s %n done' /var/lib/collectd/log
grep_test 'raw percent check: 100%s %n done' /var/lib/collectd/log
grep_test 'null check: kept$' /var/lib/collectd/log
grep_test 'write_logrs: write_logrs: flushing: timeout: no timeout, identifier: no identifier' /var/lib/collectd/log
grep_test 'write_logrs: write_logrs: yes drop is called' /var/lib/collectd/log
grep_test 'myerror: collectd_plugin::api::logger: read error: plugin encountered an error; bailing' /var/lib/collectd/log
grep_test 'read-function of plugin `myerror'"'"' failed.' /var/lib/collectd/log
grep_test 'myerror: collectd_plugin::api::logger: read error: plugin panicked' /var/lib/collectd/log
grep_test 'myerror: collectd_plugin::api::logger: panic hook error: plugin panicked: (examples/myerror.rs: 42): Oh dear what is wrong!?' /var/lib/collectd/log

grep_test 'write_logrs: write_logrs: plugin: load_with_metadata, metadata: {"environment": String("prod"), "version": UnsignedInt(5)}' /var/lib/collectd/log

grep_test 'lifecycle: flush called' /var/lib/collectd/log
grep_test 'lifecycle: dropped all' /var/lib/collectd/log
grep_test 'lifecycle: dropped log' /var/lib/collectd/log

if grep -E 'called after drop|called after it was dropped' /var/lib/collectd/log; then
    echo "A plugin was called after it was dropped"
    exit 1
fi

exit $?
