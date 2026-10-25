//! Pure quiet-hours helpers (arch/Tasks.md §5).

use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::Tz;

use crate::tasks::local_to_utc;

/// A member's override wins only when both ends are set; otherwise the household default.
pub fn effective_quiet(
    pref_from: Option<NaiveTime>,
    pref_until: Option<NaiveTime>,
    household_from: NaiveTime,
    household_until: NaiveTime,
) -> (NaiveTime, NaiveTime) {
    match (pref_from, pref_until) {
        (Some(f), Some(u)) => (f, u),
        _ => (household_from, household_until),
    }
}

/// Is `t` within `[from, until)`, wrapping midnight when `from > until`? Equal ends mean no quiet hours.
pub fn in_quiet(t: NaiveTime, from: NaiveTime, until: NaiveTime) -> bool {
    match from.cmp(&until) {
        std::cmp::Ordering::Equal => false,
        std::cmp::Ordering::Less => t >= from && t < until,
        std::cmp::Ordering::Greater => t >= from || t < until,
    }
}

/// When the quiet period containing `now` ends; `now` itself when not in quiet hours.
pub fn quiet_end_after(
    now: DateTime<Utc>,
    from: NaiveTime,
    until: NaiveTime,
    tz: Tz,
) -> DateTime<Utc> {
    let local = now.with_timezone(&tz).naive_local();
    if !in_quiet(local.time(), from, until) {
        return now;
    }
    let end_today = local.date().and_time(until);
    // Past `from` on a wrapping window: the end is tomorrow.
    let end = if from > until && local.time() >= from {
        end_today + Duration::days(1)
    } else {
        end_today
    };
    local_to_utc(end, tz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn utc(y: i32, mo: u32, d: u32, h: u32, m: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_time(hm(h, m))
            .and_utc()
    }

    #[test]
    fn effective() {
        let (hf, hu) = (hm(21, 0), hm(7, 0));
        assert_eq!(effective_quiet(None, None, hf, hu), (hf, hu));
        assert_eq!(effective_quiet(Some(hm(22, 0)), None, hf, hu), (hf, hu));
        assert_eq!(
            effective_quiet(Some(hm(22, 0)), Some(hm(6, 0)), hf, hu),
            (hm(22, 0), hm(6, 0))
        );
    }

    #[test]
    fn quiet_across_midnight() {
        let (f, u) = (hm(21, 0), hm(7, 0));
        assert!(in_quiet(hm(21, 0), f, u));
        assert!(in_quiet(hm(23, 59), f, u));
        assert!(in_quiet(hm(0, 0), f, u));
        assert!(in_quiet(hm(6, 59), f, u));
        assert!(!in_quiet(hm(7, 0), f, u));
        assert!(!in_quiet(hm(12, 0), f, u));
        assert!(!in_quiet(hm(20, 59), f, u));
    }

    #[test]
    fn quiet_same_day_and_equal() {
        assert!(in_quiet(hm(13, 0), hm(13, 0), hm(15, 0)));
        assert!(!in_quiet(hm(15, 0), hm(13, 0), hm(15, 0)));
        assert!(!in_quiet(hm(10, 0), hm(13, 0), hm(15, 0)));
        for h in [0, 7, 12, 21] {
            assert!(!in_quiet(hm(h, 0), hm(7, 0), hm(7, 0)));
        }
    }

    #[test]
    fn end_after() {
        let tz = chrono_tz::Europe::London;
        let (f, u) = (hm(21, 0), hm(7, 0));
        // Winter: London is UTC. Evening goes to the next morning.
        assert_eq!(
            quiet_end_after(utc(2026, 1, 10, 22, 0), f, u, tz),
            utc(2026, 1, 11, 7, 0)
        );
        // After midnight goes to the same day's morning.
        assert_eq!(
            quiet_end_after(utc(2026, 1, 11, 2, 0), f, u, tz),
            utc(2026, 1, 11, 7, 0)
        );
        // Outside quiet hours returns now.
        let now = utc(2026, 1, 11, 12, 0);
        assert_eq!(quiet_end_after(now, f, u, tz), now);
        // Summer: 07:00 BST = 06:00 UTC.
        assert_eq!(
            quiet_end_after(utc(2026, 7, 1, 21, 0), f, u, tz),
            utc(2026, 7, 2, 6, 0)
        );
        // Same-day window.
        assert_eq!(
            quiet_end_after(utc(2026, 1, 11, 13, 30), hm(13, 0), hm(15, 0), tz),
            utc(2026, 1, 11, 15, 0)
        );
        // DST day: the night of 2026-03-28 to 29 ends at 07:00 BST = 06:00 UTC.
        assert_eq!(
            quiet_end_after(utc(2026, 3, 28, 23, 0), f, u, tz),
            utc(2026, 3, 29, 6, 0)
        );
    }
}
