use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use fa_calendar::expand::{ExpandOptions, Expansion, expand};
use fa_connectors::ics;

fn run(name: &str, tz: Tz, from: DateTime<Utc>, to: DateTime<Utc>) -> Expansion {
    let body = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let comps = ics::parse(&body).unwrap();
    expand(
        &comps,
        &ExpandOptions {
            household_tz: tz,
            from,
            to,
        },
    )
}

fn utc(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
}

fn wide(name: &str, tz: Tz) -> Expansion {
    run(name, tz, utc(2026, 1, 1, 0, 0), utc(2032, 1, 1, 0, 0))
}

fn of<'a>(e: &'a Expansion, uid: &str) -> Vec<&'a fa_store::NewOccurrence> {
    let mut v: Vec<_> = e
        .occurrences
        .iter()
        .filter(|o| o.remote_uid == uid)
        .collect();
    v.sort_by_key(|o| o.start_at);
    v
}

#[test]
fn rfc5545_examples() {
    let e = wide("rfc5545.ics", Tz::UTC);
    assert_eq!(of(&e, "daily-count@example.com").len(), 10);
    assert_eq!(of(&e, "weekly-byday@example.com").len(), 35);
    assert_eq!(of(&e, "monthly-day@example.com").len(), 6);
    let y = of(&e, "yearly@example.com");
    assert_eq!(y.len(), 5);
    assert!(y.iter().all(|o| o.all_day && o.instance_key.len() == 10));
    assert_eq!(y[1].start_date, NaiveDate::from_ymd_opt(2027, 6, 14));
    assert!(e.warnings.is_empty());
}

#[test]
fn window_clipping() {
    let e = run(
        "rfc5545.ics",
        Tz::UTC,
        utc(2026, 9, 3, 0, 0),
        utc(2026, 9, 6, 0, 0),
    );
    let d = of(&e, "daily-count@example.com");
    assert_eq!(d.len(), 3);
    assert_eq!(d[0].start_at, utc(2026, 9, 3, 13, 0));
    assert_eq!(d[0].instance_key, "2026-09-03T13:00:00Z");
}

#[test]
fn recurrence_id_move() {
    let e = run(
        "google_export.ics",
        Tz::UTC,
        utc(2026, 10, 1, 0, 0),
        utc(2027, 1, 1, 0, 0),
    );
    let v = of(&e, "abc123@example.com");
    assert_eq!(v.len(), 4);
    let moved = v.iter().find(|o| o.title.contains("moved")).unwrap();
    assert_eq!(moved.instance_key, "2026-11-02T16:00:00Z");
    assert_eq!(moved.start_at, utc(2026, 11, 3, 17, 0));
    assert!(!v.iter().any(|o| o.start_at == utc(2026, 11, 2, 16, 0)));
}

#[test]
fn outlook_windows_tzid_count_and_exdate() {
    let e = wide("outlook.ics", Tz::UTC);
    let v = of(&e, "outlook-1@example.com");
    assert_eq!(v.len(), 4);
    assert!(
        v.iter()
            .all(|o| o.start_at.format("%H:%M").to_string() == "12:00")
    );
    assert!(!v.iter().any(|o| o.start_at == utc(2026, 10, 28, 12, 0)));
    assert_eq!(v[0].source_tz.as_deref(), Some("America/New_York"));
    assert!(v[0].is_private);
    assert!(e.warnings.is_empty());
}

#[test]
fn school_feed() {
    let chicago: Tz = "America/Chicago".parse().unwrap();
    let e = wide("school.ics", chicago);
    let b = &of(&e, "break@school.example.com")[0].clone();
    assert!(b.all_day);
    assert_eq!(b.start_date, NaiveDate::from_ymd_opt(2026, 11, 23));
    assert_eq!(b.end_date, NaiveDate::from_ymd_opt(2026, 11, 28));
    assert_eq!(b.start_at, utc(2026, 11, 23, 6, 0));
    // floating time uses the household zone
    assert_eq!(
        of(&e, "float@school.example.com")[0].start_at,
        utc(2026, 11, 4, 21, 0)
    );
    // the cancelled override removes its occurrence
    let club = of(&e, "club@school.example.com");
    assert_eq!(club.len(), 2);
    assert!(
        !club
            .iter()
            .any(|o| o.instance_key == "2026-11-12T14:00:00Z")
    );
}

#[test]
fn duration_and_missing_end() {
    let e = wide("sports.ics", Tz::UTC);
    let m1 = &of(&e, "match-1@sports.example.com")[0].clone();
    assert_eq!(m1.end_at - m1.start_at, chrono::Duration::minutes(90));
    let m2 = &of(&e, "match-2@sports.example.com")[0].clone();
    assert_eq!(m2.end_at, m2.start_at);
    assert_eq!(m2.instance_key, "");
}

#[test]
fn unknown_tzid_falls_back_with_warning() {
    let chicago: Tz = "America/Chicago".parse().unwrap();
    let e = wide("unknown_tz.ics", chicago);
    assert_eq!(e.occurrences.len(), 1);
    assert_eq!(e.occurrences[0].start_at, utc(2026, 10, 26, 15, 0));
    assert_eq!(e.warnings.len(), 1);
    assert!(e.warnings[0].contains("Mars/Olympus_Mons"));
}

#[test]
fn dst_change_weekly_and_all_day() {
    let berlin: Tz = "Europe/Berlin".parse().unwrap();
    let e = wide("dst.ics", berlin);
    let w = of(&e, "dst-weekly@example.com");
    let utc_hours: Vec<_> = w
        .iter()
        .map(|o| o.start_at.format("%H").to_string())
        .collect();
    // 18:00 local before (CEST) and after (CET) the change of 2026-10-25
    assert_eq!(utc_hours, ["16", "17", "17"]);
    let a = &of(&e, "dst-allday@example.com")[0].clone();
    assert_eq!(a.start_at, utc(2026, 10, 23, 22, 0));
    assert_eq!(a.end_at, utc(2026, 10, 27, 23, 0));
    assert_eq!(a.end_date, NaiveDate::from_ymd_opt(2026, 10, 28));
}

#[test]
fn until_before_start_keeps_only_the_first_event_without_warning() {
    let e = wide("until_before_start.ics", Tz::UTC);
    assert_eq!(of(&e, "cut-off@example.com").len(), 1);
    assert!(e.warnings.is_empty());
}
