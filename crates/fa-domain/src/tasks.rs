//! Pure task helpers: list defaults, due instants, seasons, the Now score (plan/M2.md, arch/Tasks.md).

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::authz::ListCompletionPolicy;
use crate::entities::*;

/// Weights of the Now score (D-042), stored in `household.now_weights`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct NowWeights {
    pub due: f64,
    pub flag: f64,
    pub weather: f64,
}

impl Default for NowWeights {
    fn default() -> Self {
        Self {
            due: 1.0,
            flag: 0.5,
            weather: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListDefaults {
    pub completion_policy: ListCompletionPolicy,
    pub child_can_add: ChildCanAdd,
    pub dashboard_add: bool,
    pub calendar_default: CalendarDisplay,
}

/// Defaults by list kind (arch/Tasks.md §3.2).
pub fn list_defaults(kind: TaskListKind) -> ListDefaults {
    let (child_can_add, dashboard_add, calendar_default) = match kind {
        TaskListKind::Chores => (ChildCanAdd::None, false, CalendarDisplay::Hidden),
        TaskListKind::Homework => (ChildCanAdd::Own, false, CalendarDisplay::UntilDone),
        TaskListKind::General => (ChildCanAdd::None, true, CalendarDisplay::Hidden),
        TaskListKind::Email => (ChildCanAdd::None, false, CalendarDisplay::UntilDone),
    };
    ListDefaults {
        completion_policy: ListCompletionPolicy::Assignee,
        child_can_add,
        dashboard_add,
        calendar_default,
    }
}

/// `after_done` rolls over, `rrule` skips (D-038, D-049).
pub fn default_on_missed(kind: ScheduleKind) -> OnMissed {
    match kind {
        ScheduleKind::AfterDone => OnMissed::RollOver,
        ScheduleKind::Rrule => OnMissed::AutoSkip,
    }
}

/// A local wall-clock time as a UTC instant. On a DST gap this is the first valid
/// instant after it; on an overlap the earlier instant.
pub fn local_to_utc(local: NaiveDateTime, tz: Tz) -> DateTime<Utc> {
    let mut t = local;
    for _ in 0..=24 * 60 {
        if let Some(dt) = tz.from_local_datetime(&t).earliest() {
            return dt.with_timezone(&Utc);
        }
        t += Duration::minutes(1);
    }
    local.and_utc()
}

/// Due instant of a due date: `due_time`, or 23:59 when `None`.
pub fn due_instant(date: NaiveDate, due_time: Option<NaiveTime>, tz: Tz) -> DateTime<Utc> {
    let time = due_time
        .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).expect("23:59 is a valid time"));
    local_to_utc(date.and_time(time), tz)
}

/// Local 00:00 of `date - lead_days`.
pub fn visible_from(date: NaiveDate, lead_days: i32, tz: Tz) -> DateTime<Utc> {
    let d = date - Duration::days(i64::from(lead_days.max(0)));
    local_to_utc(d.and_time(NaiveTime::MIN), tz)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("expected MM-DD with a valid month and day")]
pub struct MmddError;

/// `"MM-DD"` to an MMDD int (`"04-01"` is 401). February 29th is valid.
pub fn parse_mmdd(s: &str) -> Result<i32, MmddError> {
    let (m, d) = s.split_once('-').ok_or(MmddError)?;
    if m.len() != 2 || d.len() != 2 {
        return Err(MmddError);
    }
    let m: u32 = m.parse().map_err(|_| MmddError)?;
    let d: u32 = d.parse().map_err(|_| MmddError)?;
    NaiveDate::from_ymd_opt(2024, m, d).ok_or(MmddError)?;
    Ok((m * 100 + d) as i32)
}

pub fn format_mmdd(md: i32) -> String {
    format!("{:02}-{:02}", md / 100, md % 100)
}

pub fn mmdd(date: NaiveDate) -> i32 {
    (date.month() * 100 + date.day()) as i32
}

/// Is `md` inside `[from, until]`? Wraps the year end when `from > until`.
pub fn in_season(md: i32, from: i32, until: i32) -> bool {
    if from <= until {
        (from..=until).contains(&md)
    } else {
        md >= from || md <= until
    }
}

/// `date` itself when in season, else the first date after it on which the season starts.
pub fn next_in_season(date: NaiveDate, from: i32, until: i32) -> NaiveDate {
    if in_season(mmdd(date), from, until) {
        return date;
    }
    let (m, d) = ((from / 100) as u32, (from % 100) as u32);
    (date.year()..date.year() + 9)
        .filter_map(|y| NaiveDate::from_ymd_opt(y, m, d))
        .find(|c| *c >= date)
        .unwrap_or(date)
}

/// Inputs of [`now_score`].
#[derive(Debug, Clone, Copy)]
pub struct ScoreInput {
    pub visible_from: Option<DateTime<Utc>>,
    pub due_at: Option<DateTime<Utc>>,
    pub priority_flag: PriorityFlag,
    pub origin: TaskOrigin,
}

/// Score of an undated open task.
pub const UNDATED_URGENCY: f64 = 0.3;

fn urgency(i: &ScoreInput, now: DateTime<Utc>) -> f64 {
    let Some(due) = i.due_at else {
        return UNDATED_URGENCY;
    };
    let day = Duration::days(1);
    let start = i.visible_from.unwrap_or(due - day);
    if now >= due {
        let span = (due - start).max(day).num_seconds() as f64;
        return (1.0 + (now - due).num_seconds() as f64 / span).min(2.0);
    }
    if now <= start {
        return 0.0;
    }
    (now - start).num_seconds() as f64 / (due - start).num_seconds() as f64
}

/// `w.due · f + w.flag · [high] + w.weather · [weather]`.
pub fn now_score(input: &ScoreInput, w: &NowWeights, now: DateTime<Utc>) -> f64 {
    let flag = if input.priority_flag == PriorityFlag::High {
        w.flag
    } else {
        0.0
    };
    let weather = if input.origin == TaskOrigin::Weather {
        w.weather
    } else {
        0.0
    };
    w.due * urgency(input, now) + flag + weather
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn utc(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
        d(y, m, day).and_time(hm(h, min)).and_utc()
    }
    const LONDON: Tz = chrono_tz::Europe::London;
    const NY: Tz = chrono_tz::America::New_York;

    #[test]
    fn defaults_by_kind() {
        let c = list_defaults(TaskListKind::Chores);
        assert_eq!(c.child_can_add, ChildCanAdd::None);
        assert!(!c.dashboard_add);
        assert_eq!(c.calendar_default, CalendarDisplay::Hidden);
        let h = list_defaults(TaskListKind::Homework);
        assert_eq!(h.child_can_add, ChildCanAdd::Own);
        assert_eq!(h.calendar_default, CalendarDisplay::UntilDone);
        let g = list_defaults(TaskListKind::General);
        assert!(g.dashboard_add);
        assert_eq!(g.completion_policy, ListCompletionPolicy::Assignee);
    }

    #[test]
    fn on_missed_defaults() {
        assert_eq!(
            default_on_missed(ScheduleKind::AfterDone),
            OnMissed::RollOver
        );
        assert_eq!(default_on_missed(ScheduleKind::Rrule), OnMissed::AutoSkip);
    }

    #[test]
    fn due_instant_plain_and_default_time() {
        // London in January is UTC; New York is UTC-5.
        assert_eq!(
            due_instant(d(2026, 1, 10), Some(hm(8, 0)), LONDON),
            utc(2026, 1, 10, 8, 0)
        );
        assert_eq!(
            due_instant(d(2026, 1, 10), None, LONDON),
            utc(2026, 1, 10, 23, 59)
        );
        assert_eq!(
            due_instant(d(2026, 1, 10), None, NY),
            utc(2026, 1, 11, 4, 59)
        );
    }

    #[test]
    fn due_instant_dst_gap_london() {
        // 2026-03-29: 01:00 to 02:00 does not exist; 01:30 becomes 02:00 BST = 01:00 UTC.
        assert_eq!(
            due_instant(d(2026, 3, 29), Some(hm(1, 30)), LONDON),
            utc(2026, 3, 29, 1, 0)
        );
        // After the change, 23:59 BST = 22:59 UTC.
        assert_eq!(
            due_instant(d(2026, 3, 29), None, LONDON),
            utc(2026, 3, 29, 22, 59)
        );
    }

    #[test]
    fn due_instant_dst_gap_new_york() {
        // 2026-03-08: 02:00 to 03:00 does not exist; 02:30 becomes 03:00 EDT = 07:00 UTC.
        assert_eq!(
            due_instant(d(2026, 3, 8), Some(hm(2, 30)), NY),
            utc(2026, 3, 8, 7, 0)
        );
        assert_eq!(
            due_instant(d(2026, 3, 8), Some(hm(12, 0)), NY),
            utc(2026, 3, 8, 16, 0)
        );
    }

    #[test]
    fn due_instant_dst_overlap_takes_earlier() {
        // 2026-10-25 London: 01:30 happens twice; the earlier is BST = 00:30 UTC.
        assert_eq!(
            due_instant(d(2026, 10, 25), Some(hm(1, 30)), LONDON),
            utc(2026, 10, 25, 0, 30)
        );
        // 2026-11-01 New York: 01:30 EDT = 05:30 UTC.
        assert_eq!(
            due_instant(d(2026, 11, 1), Some(hm(1, 30)), NY),
            utc(2026, 11, 1, 5, 30)
        );
    }

    #[test]
    fn visible_from_is_local_midnight() {
        assert_eq!(
            visible_from(d(2026, 1, 10), 0, LONDON),
            utc(2026, 1, 10, 0, 0)
        );
        assert_eq!(visible_from(d(2026, 1, 10), 3, NY), utc(2026, 1, 7, 5, 0));
        // Lead days across the London spring-forward: 03-30 minus 1 = 03-29 00:00 GMT.
        assert_eq!(
            visible_from(d(2026, 3, 30), 1, LONDON),
            utc(2026, 3, 29, 0, 0)
        );
        // 03-30 00:00 is BST: 03-29 23:00 UTC.
        assert_eq!(
            visible_from(d(2026, 3, 30), 0, LONDON),
            utc(2026, 3, 29, 23, 0)
        );
        // New York 03-09 00:00 is EDT.
        assert_eq!(visible_from(d(2026, 3, 9), 0, NY), utc(2026, 3, 9, 4, 0));
        assert_eq!(visible_from(d(2026, 3, 9), 1, NY), utc(2026, 3, 8, 5, 0));
    }

    #[test]
    fn mmdd_parse_format() {
        assert_eq!(parse_mmdd("04-01"), Ok(401));
        assert_eq!(parse_mmdd("10-31"), Ok(1031));
        assert_eq!(parse_mmdd("02-29"), Ok(229));
        for bad in [
            "4-1", "13-01", "02-30", "0401", "ab-cd", "", "04-1", "04-001",
        ] {
            assert!(parse_mmdd(bad).is_err(), "{bad}");
        }
        assert_eq!(format_mmdd(401), "04-01");
        assert_eq!(format_mmdd(1231), "12-31");
    }

    #[test]
    fn season_plain() {
        assert!(in_season(401, 401, 1031));
        assert!(in_season(1031, 401, 1031));
        assert!(!in_season(331, 401, 1031));
        assert!(!in_season(1101, 401, 1031));
        assert!(!in_season(103, 401, 1031));
    }

    #[test]
    fn season_wrapping_year_end() {
        let (f, u) = (parse_mmdd("11-01").unwrap(), parse_mmdd("02-28").unwrap());
        for md in [1101, 1231, 101, 228] {
            assert!(in_season(md, f, u), "{md}");
        }
        for md in [301, 701, 1031] {
            assert!(!in_season(md, f, u), "{md}");
        }
    }

    #[test]
    fn next_in_season_dates() {
        let (f, u) = (1101, 228);
        assert_eq!(next_in_season(d(2026, 12, 5), f, u), d(2026, 12, 5));
        assert_eq!(next_in_season(d(2027, 3, 1), f, u), d(2027, 11, 1));
        assert_eq!(next_in_season(d(2026, 6, 15), f, u), d(2026, 11, 1));
        // Non-wrapping: after the window, the next start is next year.
        assert_eq!(next_in_season(d(2026, 11, 2), 401, 1031), d(2027, 4, 1));
        assert_eq!(next_in_season(d(2026, 2, 1), 401, 1031), d(2026, 4, 1));
        assert_eq!(next_in_season(d(2026, 4, 1), 401, 1031), d(2026, 4, 1));
        // Feb 29th start skips to the next leap year.
        assert_eq!(next_in_season(d(2026, 6, 1), 229, 301), d(2028, 2, 29));
    }

    fn input(vf: Option<DateTime<Utc>>, due: Option<DateTime<Utc>>) -> ScoreInput {
        ScoreInput {
            visible_from: vf,
            due_at: due,
            priority_flag: PriorityFlag::Normal,
            origin: TaskOrigin::Manual,
        }
    }

    #[test]
    fn score_urgency_curve() {
        let w = NowWeights::default();
        let vf = utc(2026, 5, 1, 0, 0);
        let due = utc(2026, 5, 3, 0, 0);
        let i = input(Some(vf), Some(due));
        let s = |now| now_score(&i, &w, now);
        assert_eq!(s(utc(2026, 4, 30, 12, 0)), 0.0);
        assert!((s(utc(2026, 5, 2, 0, 0)) - 0.5).abs() < 1e-9);
        assert!((s(due) - 1.0).abs() < 1e-9);
        // One day overdue with a two-day window: 1 + 1/2.
        assert!((s(utc(2026, 5, 4, 0, 0)) - 1.5).abs() < 1e-9);
        assert_eq!(s(utc(2026, 5, 20, 0, 0)), 2.0);
    }

    #[test]
    fn score_short_window_uses_one_day_floor() {
        let w = NowWeights::default();
        let due = utc(2026, 5, 3, 12, 0);
        let i = input(Some(utc(2026, 5, 3, 6, 0)), Some(due));
        // Overdue by 12 h, span floor 1 day: 1.5.
        assert!((now_score(&i, &w, utc(2026, 5, 4, 0, 0)) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn score_undated_flag_weather() {
        let w = NowWeights::default();
        let now = utc(2026, 5, 1, 0, 0);
        assert!((now_score(&input(None, None), &w, now) - 0.3).abs() < 1e-9);
        let mut i = input(None, None);
        i.priority_flag = PriorityFlag::High;
        assert!((now_score(&i, &w, now) - 0.8).abs() < 1e-9);
        i.origin = TaskOrigin::Weather;
        assert!((now_score(&i, &w, now) - 1.3).abs() < 1e-9);
        let w2 = NowWeights {
            due: 2.0,
            flag: 0.0,
            weather: 0.0,
        };
        assert!((now_score(&i, &w2, now) - 0.6).abs() < 1e-9);
    }

    #[test]
    fn weights_serde_defaults() {
        let w = NowWeights::default();
        assert_eq!(
            serde_json::to_string(&w).unwrap(),
            r#"{"due":1.0,"flag":0.5,"weather":0.5}"#
        );
    }
}
