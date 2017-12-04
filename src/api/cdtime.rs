use bindings::cdtime_t;
use chrono::prelude::*;
use chrono::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CdTime(pub u64);

impl<Tz: TimeZone> From<DateTime<Tz>> for CdTime {
    fn from(dt: DateTime<Tz>) -> Self {
        let nanos = (dt.timestamp() as u64) + u64::from(dt.timestamp_subsec_nanos());
        CdTime(nanos_to_collectd(nanos))
    }
}

impl From<CdTime> for DateTime<Utc> {
    fn from(v: CdTime) -> DateTime<Utc> {
        let CdTime(ns) = v;
        let secs = ns / 1_000_000_000;
        let left = ns % 1_000_000_000;
        Utc.timestamp(secs as i64, left as u32)
    }
}

impl From<Duration> for CdTime {
    fn from(d: Duration) -> Self {
        CdTime(d.num_nanoseconds().unwrap() as u64)
    }
}

impl From<CdTime> for Duration {
    fn from(v: CdTime) -> Self {
        let CdTime(ns) = v;
        Duration::nanoseconds(ns as i64)
    }
}

impl From<cdtime_t> for CdTime {
    fn from(d: cdtime_t) -> Self {
        CdTime(collectd_to_nanos(d))
    }
}

impl Into<cdtime_t> for CdTime {
    fn into(self) -> cdtime_t {
        let CdTime(x) = self;
        nanos_to_collectd(x)
    }
}

/// The time is stored at a 2^-30 second resolution, i.e. the most significant 34 bit are used to
/// store the time in seconds, the least significant bits store the sub-second part in something
/// very close to nanoseconds. *The* big advantage of storing time in this manner is that comparing
/// times and calculating differences is as simple as it is with `time_t`, i.e. a simple integer
/// comparison / subtraction works.
fn nanos_to_collectd(nanos: u64) -> cdtime_t {
    ((nanos / 1_000_000_000) << 30)
        | ((((nanos % 1_000_000_000) << 30) + 500_000_000) / 1_000_000_000)
}

fn collectd_to_nanos(cd: cdtime_t) -> u64 {
    ((cd >> 30) * 1_000_000_000) + (((cd & 0x3fff_ffff) * 1_000_000_000 + (1 << 29)) >> 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nanos_to_collectd() {
        // Taken from utils_time_test.c

        assert_eq!(nanos_to_collectd(1439981652801860766), 1546168526406004689);
        assert_eq!(nanos_to_collectd(1439981836985281914), 1546168724171447263);
        assert_eq!(nanos_to_collectd(1439981880053705608), 1546168770415815077);
    }

    #[test]
    fn test_collectd_to_nanos() {
        assert_eq!(collectd_to_nanos(1546168526406004689), 1439981652801860766);
        assert_eq!(collectd_to_nanos(1546168724171447263), 1439981836985281914);
        assert_eq!(collectd_to_nanos(1546168770415815077), 1439981880053705608);
    }

    #[test]
    fn test_collectd_to_duration() {
        let v: cdtime_t = nanos_to_collectd(1_000_000_000);
        let dur = Duration::from(CdTime::from(v));
        assert_eq!(dur.num_seconds(), 1);
    }

    #[test]
    fn test_collectd_to_datetime() {
        let v: cdtime_t = nanos_to_collectd(1_000_000_000);
        let dt: DateTime<Utc> = CdTime::from(v).into();
        assert_eq!(Utc.ymd(1970, 1, 1).and_hms(0, 0, 1), dt);
    }
}
