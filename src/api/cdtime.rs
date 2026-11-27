//! # `CdTime`
//!
//! Collectd stores time information in a custom type: `cdtime_t`. Below is a snippet from
//! collectd's docs about why this custom format was chosen.
//!
//! The time is stored at a 2<sup>-30</sup> second resolution, i.e. the most significant 34 bit are used to
//! store the time in seconds, the least significant bits store the sub-second part in something
//! very close to nanoseconds. *The* big advantage of storing time in this manner is that comparing
//! times and calculating differences is as simple as it is with `time_t`, i.e. a simple integer
//! comparison / subtraction works.

use crate::bindings::cdtime_t;
use chrono::prelude::*;
use chrono::Duration;

/// `CdTime` allows for ergonomic interop between collectd's `cdtime_t` and chrono's `Duration` and
/// `DateTime`. The single field represents epoch nanoseconds.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CdTime(pub u64);

/// Times before 1970 become 1970, as collectd can't represent them.
impl<Tz: TimeZone> From<DateTime<Tz>> for CdTime {
    fn from(dt: DateTime<Tz>) -> Self {
        match u64::try_from(dt.timestamp()) {
            Ok(secs) => {
                let nanos = u64::from(dt.timestamp_subsec_nanos());
                CdTime(secs.saturating_mul(1_000_000_000).saturating_add(nanos))
            }
            Err(_) => CdTime(0),
        }
    }
}

impl From<CdTime> for DateTime<Utc> {
    fn from(v: CdTime) -> DateTime<Utc> {
        let CdTime(ns) = v;
        let secs = ns / 1_000_000_000;
        let left = ns % 1_000_000_000;
        Utc.timestamp_opt(secs as i64, left as u32)
            .latest()
            .unwrap_or_default()
    }
}

/// Negative durations become zero. Durations too long to fit in nanoseconds (about 292 years)
/// are cut down to the longest one that fits.
impl From<Duration> for CdTime {
    fn from(d: Duration) -> Self {
        let nanos = d
            .num_nanoseconds()
            .unwrap_or(if d < Duration::zero() { 0 } else { i64::MAX });
        CdTime(u64::try_from(nanos).unwrap_or(0))
    }
}

/// Durations too long to fit in nanoseconds (about 292 years) are cut down to the longest one
/// that fits.
impl From<CdTime> for Duration {
    fn from(v: CdTime) -> Self {
        let CdTime(ns) = v;
        Duration::nanoseconds(i64::try_from(ns).unwrap_or(i64::MAX))
    }
}

impl From<cdtime_t> for CdTime {
    fn from(d: cdtime_t) -> Self {
        CdTime(collectd_to_nanos(d))
    }
}

impl From<CdTime> for cdtime_t {
    fn from(d: CdTime) -> Self {
        let CdTime(x) = d;
        nanos_to_collectd(x)
    }
}

/// Convert epoch nanoseconds into collectd's 2<sup>-30</sup> second resolution
pub fn nanos_to_collectd(nanos: u64) -> cdtime_t {
    let secs = nanos / 1_000_000_000;

    // Collectd stores seconds in 34 bits. Later times become the latest time collectd can hold.
    if secs >= 1 << 34 {
        return cdtime_t::MAX;
    }

    (secs << 30) | ((((nanos % 1_000_000_000) << 30) + 500_000_000) / 1_000_000_000)
}

/// Convert collectd's 2^-30 second resolution into epoch nanoseconds
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
        let res = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 1);
        assert_eq!(res.unwrap(), dt);
    }

    #[test]
    fn test_out_of_range_to_collectd() {
        let before_1970 = Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 59).unwrap();
        assert_eq!(CdTime::from(before_1970), CdTime(0));

        assert_eq!(CdTime::from(Duration::seconds(-1)), CdTime(0));
        assert_eq!(CdTime::from(Duration::MIN), CdTime(0));
        assert_eq!(CdTime::from(Duration::MAX), CdTime(i64::MAX as u64));

        let far_future = Utc.with_ymd_and_hms(3000, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(cdtime_t::from(CdTime::from(far_future)), cdtime_t::MAX);
        assert_eq!(nanos_to_collectd(u64::MAX), cdtime_t::MAX);

        let longest = Duration::nanoseconds(i64::MAX);
        assert_eq!(Duration::from(CdTime(u64::MAX)), longest);
    }

    #[test]
    fn test_datetime_to_collectd() {
        let res = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 1);
        let cd = CdTime::from(res.unwrap());
        assert_eq!(cd.0, 1_000_000_000);
    }
}
