//! Next-due computation for both schedule kinds (plan/M2.md, Time semantics).

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use fa_domain::tasks::{due_instant, next_in_season};
use fa_domain::{Schedule, TaskDefinition};

const BATCH: u16 = 400;
const MAX_BATCHES: usize = 25;

fn parse_set(
    rrule: &str,
    start: NaiveDate,
    due_time: Option<NaiveTime>,
    tz: Tz,
) -> Result<rrule::RRuleSet, String> {
    let body = rrule.trim();
    if body.contains('\n') || body.to_ascii_uppercase().contains("DTSTART") {
        return Err("give only the RRULE body, such as FREQ=WEEKLY;BYDAY=SA".into());
    }
    let body = body.strip_prefix("RRULE:").unwrap_or(body);
    let t = due_time.unwrap_or(NaiveTime::MIN);
    let text = format!(
        "DTSTART;TZID={}:{}\nRRULE:{body}",
        tz.name(),
        start.and_time(t).format("%Y%m%dT%H%M%S")
    );
    text.parse::<rrule::RRuleSet>().map_err(|e| e.to_string())
}

/// Checks an RRULE body (no DTSTART), such as `FREQ=WEEKLY;BYDAY=SA`.
pub fn validate_rrule(body: &str) -> Result<(), String> {
    let start = NaiveDate::from_ymd_opt(2000, 1, 1).ok_or("bad date")?;
    parse_set(body, start, None, Tz::UTC).map(|_| ())
}

/// Instance dates (local) at or after `from`, in order, at most `n`.
fn instances_from(
    rrule: &str,
    start: NaiveDate,
    due_time: Option<NaiveTime>,
    tz: Tz,
    from: NaiveDate,
    n: u16,
) -> Vec<NaiveDate> {
    let Ok(set) = parse_set(rrule, start, due_time, tz) else {
        return Vec::new();
    };
    let Some(from_dt) = tz
        .from_local_datetime(&from.and_time(NaiveTime::MIN))
        .earliest()
    else {
        return Vec::new();
    };
    set.after(from_dt.with_timezone(&rrule::Tz::Tz(tz)))
        .all(n)
        .dates
        .into_iter()
        .map(|d| d.naive_local().date())
        .collect()
}

/// The first rrule instance date at or after `from`, in season when a window is given.
fn first_rrule_date(
    rrule: &str,
    d: &TaskDefinition,
    tz: Tz,
    from: NaiveDate,
    pred: impl Fn(NaiveDate) -> bool,
) -> Option<NaiveDate> {
    let mut cursor = from;
    for _ in 0..MAX_BATCHES {
        let batch = instances_from(rrule, d.start_date, d.due_time, tz, cursor, BATCH);
        let last = *batch.last()?;
        let in_season = |x: &NaiveDate| {
            d.season
                .is_none_or(|(f, u)| fa_domain::tasks::in_season(fa_domain::tasks::mmdd(*x), f, u))
        };
        if let Some(hit) = batch.iter().copied().find(|x| in_season(x) && pred(*x)) {
            return Some(hit);
        }
        cursor = last + Duration::days(1);
    }
    None
}

fn local_date(t: DateTime<Utc>, tz: Tz) -> NaiveDate {
    t.with_timezone(&tz).date_naive()
}

/// The first `next_due_at` of a definition at `now`.
pub fn first_due(d: &TaskDefinition, tz: Tz, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match &d.schedule {
        Schedule::AfterDone { .. } => {
            let date = match d.season {
                Some((f, u)) => next_in_season(d.start_date, f, u),
                None => d.start_date,
            };
            Some(due_instant(date, d.due_time, tz))
        }
        Schedule::Rrule { rrule } => {
            let today = local_date(now, tz);
            let date = first_rrule_date(rrule, d, tz, today, |x| {
                due_instant(x, d.due_time, tz) >= now
            })?;
            Some(due_instant(date, d.due_time, tz))
        }
    }
}

/// For rrule: the next in-season instance strictly after the occurrence due at `current`.
pub fn advance_rrule(d: &TaskDefinition, tz: Tz, current: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let Schedule::Rrule { rrule } = &d.schedule else {
        return None;
    };
    let after = local_date(current, tz) + Duration::days(1);
    let date = first_rrule_date(rrule, d, tz, after, |_| true)?;
    Some(due_instant(date, d.due_time, tz))
}

/// For rrule: the first in-season instance at or after the date of `current` (season moves).
pub fn move_into_season(
    d: &TaskDefinition,
    tz: Tz,
    current: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let date = local_date(current, tz);
    match &d.schedule {
        Schedule::AfterDone { .. } => {
            let (f, u) = d.season?;
            Some(due_instant(next_in_season(date, f, u), d.due_time, tz))
        }
        Schedule::Rrule { rrule } => {
            let date = first_rrule_date(rrule, d, tz, date, |_| true)?;
            Some(due_instant(date, d.due_time, tz))
        }
    }
}

/// For after_done: the next due instant after a completion or skip at `t`.
pub fn after_done_next(d: &TaskDefinition, tz: Tz, t: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let Schedule::AfterDone { interval_days } = &d.schedule else {
        return None;
    };
    let mut date = local_date(t, tz) + Duration::days(i64::from(*interval_days));
    if let Some((f, u)) = d.season {
        date = next_in_season(date, f, u);
    }
    Some(due_instant(date, d.due_time, tz))
}

/// Is the due instant's local date inside the definition's season?
pub fn due_in_season(d: &TaskDefinition, tz: Tz, due: DateTime<Utc>) -> bool {
    match d.season {
        None => true,
        Some((f, u)) => {
            fa_domain::tasks::in_season(fa_domain::tasks::mmdd(local_date(due, tz)), f, u)
        }
    }
}

/// Up to `n` upcoming due instants at or after `from` (for the definition editor preview).
pub fn preview_dates(
    schedule: &Schedule,
    start_date: NaiveDate,
    due_time: Option<NaiveTime>,
    tz: Tz,
    from: DateTime<Utc>,
    n: usize,
) -> Vec<DateTime<Utc>> {
    match schedule {
        Schedule::Rrule { rrule } => {
            let today = local_date(from, tz);
            instances_from(rrule, start_date, due_time, tz, today, BATCH)
                .into_iter()
                .map(|d| due_instant(d, due_time, tz))
                .filter(|t| *t >= from)
                .take(n)
                .collect()
        }
        Schedule::AfterDone { interval_days } => {
            let step = Duration::days(i64::from((*interval_days).max(1)));
            let mut date = start_date;
            let mut out = Vec::new();
            for _ in 0..4000 {
                let t = due_instant(date, due_time, tz);
                if t >= from {
                    out.push(t);
                    if out.len() >= n {
                        break;
                    }
                }
                date += step;
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_domain::*;

    const LONDON: Tz = chrono_tz::Europe::London;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }
    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn utc(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
        d(y, m, day).and_time(hm(h, min)).and_utc()
    }

    pub(crate) fn def(schedule: Schedule, start: NaiveDate) -> TaskDefinition {
        TaskDefinition {
            id: TaskDefinitionId::new(),
            household_id: HouseholdId::new(),
            list_id: TaskListId::new(),
            title: "t".into(),
            notes: None,
            steps_template: vec![],
            schedule,
            start_date: start,
            due_time: None,
            lead_days: 0,
            on_missed: OnMissed::AutoSkip,
            strategy: Strategy::Fixed,
            advance_on: AdvanceOn::AnyCompletion,
            skip_advances: true,
            rotation_cursor: 0,
            priority_flag: PriorityFlag::Normal,
            season: None,
            season_condition: None,
            condition_ok: None,
            condition_checked_at: None,
            last_done_at: None,
            next_due_at: None,
            paused: false,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn weekly_sat() -> Schedule {
        Schedule::Rrule {
            rrule: "FREQ=WEEKLY;BYDAY=SA".into(),
        }
    }

    #[test]
    fn rrule_validation() {
        assert!(validate_rrule("FREQ=WEEKLY;BYDAY=SA").is_ok());
        assert!(validate_rrule("FREQ=NEVER").is_err());
        assert!(validate_rrule("FREQ=DAILY\nRRULE:FREQ=DAILY").is_err());
        assert!(validate_rrule("DTSTART:20260101T000000Z").is_err());
    }

    #[test]
    fn weekly_saturday_first_and_next() {
        let mut x = def(weekly_sat(), d(2026, 1, 1));
        x.due_time = Some(hm(18, 0));
        // 2026-01-03 is a Saturday.
        let first = first_due(&x, LONDON, utc(2026, 1, 1, 12, 0)).unwrap();
        assert_eq!(first, utc(2026, 1, 3, 18, 0));
        assert_eq!(
            advance_rrule(&x, LONDON, first).unwrap(),
            utc(2026, 1, 10, 18, 0)
        );
    }

    #[test]
    fn dst_week_keeps_wall_clock_time() {
        let mut x = def(weekly_sat(), d(2026, 3, 1));
        x.due_time = Some(hm(18, 0));
        // 2026-03-28 is the Saturday before the London change (GMT); 04-04 is BST.
        let before = utc(2026, 3, 28, 18, 0);
        assert_eq!(
            advance_rrule(&x, LONDON, before).unwrap(),
            utc(2026, 4, 4, 17, 0)
        );
    }

    #[test]
    fn no_due_time_means_end_of_day() {
        let x = def(weekly_sat(), d(2026, 1, 1));
        assert_eq!(
            first_due(&x, LONDON, utc(2026, 1, 1, 0, 0)).unwrap(),
            utc(2026, 1, 3, 23, 59)
        );
    }

    #[test]
    fn after_done_moves_from_completion_date() {
        let mut x = def(Schedule::AfterDone { interval_days: 7 }, d(2026, 1, 1));
        assert_eq!(
            first_due(&x, LONDON, utc(2026, 1, 1, 0, 0)).unwrap(),
            utc(2026, 1, 1, 23, 59)
        );
        assert_eq!(
            after_done_next(&x, LONDON, utc(2026, 1, 5, 9, 0)).unwrap(),
            utc(2026, 1, 12, 23, 59)
        );
        x.season = Some((401, 1031));
        assert_eq!(
            after_done_next(&x, LONDON, utc(2026, 1, 5, 9, 0)).unwrap(),
            due_instant(d(2026, 4, 1), None, LONDON)
        );
    }

    #[test]
    fn season_moves_rrule_to_first_in_season_instance() {
        let mut x = def(weekly_sat(), d(2026, 1, 3));
        x.season = Some((401, 1031));
        let t = move_into_season(&x, LONDON, utc(2026, 2, 7, 23, 59)).unwrap();
        // 2026-04-04 is the first Saturday in April.
        assert_eq!(t, due_instant(d(2026, 4, 4), None, LONDON));
        assert!(!due_in_season(&x, LONDON, utc(2026, 2, 7, 23, 59)));
        assert!(due_in_season(&x, LONDON, t));
    }

    #[test]
    fn season_wrapping_year_end_rrule() {
        let mut x = def(
            Schedule::Rrule {
                rrule: "FREQ=DAILY".into(),
            },
            d(2026, 1, 1),
        );
        x.season = Some((1101, 228));
        assert_eq!(
            advance_rrule(&x, LONDON, due_instant(d(2026, 12, 30), None, LONDON)).unwrap(),
            due_instant(d(2026, 12, 31), None, LONDON)
        );
        assert_eq!(
            advance_rrule(&x, LONDON, due_instant(d(2027, 2, 28), None, LONDON)).unwrap(),
            due_instant(d(2027, 11, 1), None, LONDON)
        );
    }

    #[test]
    fn preview_lists_future_dates() {
        let v = preview_dates(
            &weekly_sat(),
            d(2026, 1, 1),
            Some(hm(9, 0)),
            LONDON,
            utc(2026, 1, 4, 0, 0),
            3,
        );
        assert_eq!(
            v,
            vec![
                utc(2026, 1, 10, 9, 0),
                utc(2026, 1, 17, 9, 0),
                utc(2026, 1, 24, 9, 0)
            ]
        );
    }

    #[test]
    fn exhausted_rrule_has_no_next() {
        let x = def(
            Schedule::Rrule {
                rrule: "FREQ=DAILY;COUNT=2".into(),
            },
            d(2026, 1, 1),
        );
        assert!(advance_rrule(&x, LONDON, due_instant(d(2026, 1, 2), None, LONDON)).is_none());
    }
}
