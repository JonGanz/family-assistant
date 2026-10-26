//! Date and time phrase finder, resolved in the household time zone against `received_at`.

use std::ops::Range;
use std::sync::LazyLock;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhraseKind {
    /// Month name and day ("Friday, Nov 7", "7 November").
    Explicit,
    /// "11/7", "2026-11-07".
    Numeric,
    /// "today", "tomorrow", "tonight".
    Relative,
    /// A weekday on its own ("next Friday").
    Weekday,
}

#[derive(Debug, Clone)]
pub(crate) struct DatePhrase {
    pub span: Range<usize>,
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub kind: PhraseKind,
    /// The written weekday does not fall on the resolved date.
    pub weekday_mismatch: bool,
    pub had_weekday: bool,
}

const WD: &str = r"(?:monday|mon|tuesday|tues|tue|wednesday|wed|thursday|thurs|thur|thu|friday|fri|saturday|sat|sunday|sun)\.?";
const MON: &str = r"(?:january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)\.?";

fn re(p: &str) -> Regex {
    RegexBuilder::new(p)
        .case_insensitive(true)
        .size_limit(1 << 22)
        .build()
        .expect("static regex")
}

static MONTH_DAY: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b(?:(?P<wd>{WD}),?\s+)?(?P<mon>{MON})\s+(?P<day>\d{{1,2}})(?:st|nd|rd|th)?\b(?:,?\s*(?P<year>\d{{4}})\b)?"
    ))
});
static DAY_MONTH: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b(?:(?P<wd>{WD}),?\s+)?(?P<day>\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?(?P<mon>{MON})(?:,?\s*(?P<year>\d{{4}})\b)?(?:\s|[,.;:!?)]|$)"
    ))
});
static NUMERIC: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"(?:\b(?P<wd>{WD}),?\s+)?(?:\b(?P<iso_y>\d{{4}})-(?P<iso_m>\d{{2}})-(?P<iso_d>\d{{2}})\b|\b(?P<a>\d{{1,2}})/(?P<b>\d{{1,2}})(?:/(?P<y>\d{{4}}|\d{{2}}))?\b)"
    ))
});
static RELATIVE: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?P<rel>today|tomorrow|tonight)\b"));
static WEEKDAY: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b(?:(?P<next>next|this|coming)\s+)?(?P<wd>{WD})"
    ))
});
static TIME: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^\s*(?:(?P<conn>,|-|–|—|at|@|from|starting|beginning)\s*)?(?P<h1>\d{1,2})(?::(?P<m1>\d{2}))?\s*(?:(?P<ap1>[ap]\.?m)\b\.?)?(?:\s*(?:-|–|—|to|until|till)\s*(?P<h2>\d{1,2})(?::(?P<m2>\d{2}))?\s*(?:(?P<ap2>[ap]\.?m)\b\.?)?)?",
    )
});

pub(crate) fn month_num(s: &str) -> Option<u32> {
    let s = s.trim_end_matches('.').to_lowercase();
    let m = match s.get(..3)? {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    Some(m)
}

pub(crate) fn weekday_of(s: &str) -> Option<Weekday> {
    let s = s.trim_end_matches('.').to_lowercase();
    Some(match s.get(..3)? {
        "mon" => Weekday::Mon,
        "tue" => Weekday::Tue,
        "wed" => Weekday::Wed,
        "thu" => Weekday::Thu,
        "fri" => Weekday::Fri,
        "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

fn is_pm(s: &str) -> bool {
    s.to_lowercase().starts_with('p')
}

fn hour24(h: u32, pm: Option<bool>) -> Option<u32> {
    if h > 23 {
        return None;
    }
    Some(match pm {
        Some(_) if h > 12 => h,
        Some(true) => {
            if h == 12 {
                12
            } else {
                h + 12
            }
        }
        Some(false) => {
            if h == 12 {
                0
            } else {
                h
            }
        }
        None => match h {
            0 | 13..=23 => h,
            1..=6 => h + 12,
            _ => h,
        },
    })
}

/// A time (or range) directly after a date. Returns (bytes consumed, start, end).
fn time_after(rest: &str) -> Option<(usize, NaiveTime, Option<NaiveTime>)> {
    let c = TIME.captures(rest)?;
    let whole = c.get(0)?;
    let h1: u32 = c.name("h1")?.as_str().parse().ok()?;
    let m1: u32 = c.name("m1").map_or(Some(0), |m| m.as_str().parse().ok())?;
    let ap1 = c.name("ap1").map(|m| is_pm(m.as_str()));
    let at_conn = c
        .name("conn")
        .is_some_and(|m| matches!(m.as_str().to_lowercase().as_str(), "at" | "@"));
    let ap2 = c.name("ap2").map(|m| is_pm(m.as_str()));
    let end = match c.name("h2") {
        Some(h2) => {
            let h2: u32 = h2.as_str().parse().ok()?;
            let m2: u32 = c.name("m2").map_or(Some(0), |m| m.as_str().parse().ok())?;
            let explicit2 = c.name("m2").is_some() || ap2.is_some();
            explicit2.then_some((h2, m2))
        }
        None => None,
    };
    let explicit1 = c.name("m1").is_some() || ap1.is_some() || at_conn || end.is_some();
    if !explicit1 || m1 > 59 {
        return None;
    }
    let (end_t, consumed) = match end {
        Some((h2, m2)) if m2 <= 59 => {
            let e = NaiveTime::from_hms_opt(hour24(h2, ap2)?, m2, 0)?;
            (Some((e, ap2)), whole.end())
        }
        _ => {
            // Range part dropped: consume only the first time.
            let first_end = c
                .name("ap1")
                .map(|m| m.end())
                .or_else(|| c.name("m1").map(|m| m.end()))
                .unwrap_or_else(|| c.name("h1").map_or(0, |m| m.end()));
            (None, first_end)
        }
    };
    let mut start_h = match (ap1, end_t) {
        (None, Some((_, Some(pm2)))) => hour24(h1, Some(pm2))?,
        _ => hour24(h1, ap1)?,
    };
    let start = NaiveTime::from_hms_opt(start_h, m1, 0)?;
    if let Some((e, _)) = end_t
        && ap1.is_none()
        && start > e
        && start_h >= 12
        && h1 <= 12
    {
        // "11-1pm": 11pm would be after 1pm, so the start is in the morning.
        start_h = if h1 == 12 { 0 } else { h1 };
        let start = NaiveTime::from_hms_opt(start_h, m1, 0)?;
        return Some((consumed, start, Some(e)));
    }
    Some((consumed, start, end_t.map(|(e, _)| e)))
}

/// Month and day to a date. Without a year: this year, or next year when received within
/// two months of the year end; otherwise past dates are dropped (unless `allow_past`).
fn resolve_md(
    m: u32,
    d: u32,
    year: Option<i32>,
    received: NaiveDate,
    allow_past: bool,
) -> Option<NaiveDate> {
    match year {
        Some(y) => {
            let date = NaiveDate::from_ymd_opt(y, m, d)?;
            (allow_past || date >= received).then_some(date)
        }
        None => {
            let date = NaiveDate::from_ymd_opt(received.year(), m, d);
            match date {
                Some(date) if date >= received || allow_past => Some(date),
                _ if received.month() >= 11 => NaiveDate::from_ymd_opt(received.year() + 1, m, d),
                _ => None,
            }
        }
    }
}

fn overlaps(found: &[DatePhrase], r: &Range<usize>) -> bool {
    found
        .iter()
        .any(|p| p.span.start < r.end && r.start < p.span.end)
}

fn with_time(text: &str, mut p: DatePhrase) -> DatePhrase {
    if let Some((n, t, e)) = time_after(&text[p.span.end..]) {
        p.span.end += n;
        p.time = Some(t);
        p.end_time = e;
    }
    p
}

/// Every date phrase in `text`, in order. `allow_past` keeps dates before `received_at`
/// (bills).
pub(crate) fn find_dates(
    text: &str,
    received_at: DateTime<Utc>,
    tz: Tz,
    allow_past: bool,
) -> Vec<DatePhrase> {
    // Quoted reply lines are not part of this message's own content.
    let scrub: String = text
        .split_inclusive('\n')
        .map(|l| {
            if l.trim_start().starts_with('>') {
                " ".repeat(l.len())
            } else {
                l.to_owned()
            }
        })
        .collect();
    let text = scrub.as_str();
    let received = received_at.with_timezone(&tz).date_naive();
    let mut found: Vec<DatePhrase> = Vec::new();

    for (re, kind_dm) in [(&*MONTH_DAY, false), (&*DAY_MONTH, true)] {
        for c in re.captures_iter(text) {
            let Some(whole) = c.get(0) else { continue };
            let mut end = whole.end();
            if kind_dm {
                // The pattern consumed one trailing delimiter; give it back.
                end = c.name("mon").map_or(end, |m| m.end());
                if let Some(y) = c.name("year") {
                    end = y.end();
                }
            }
            let range = whole.start()..end;
            if overlaps(&found, &range) {
                continue;
            }
            let (Some(mon), Some(day)) = (c.name("mon"), c.name("day")) else {
                continue;
            };
            let (Some(m), Ok(d)) = (month_num(mon.as_str()), day.as_str().parse::<u32>()) else {
                continue;
            };
            let year = c.name("year").and_then(|y| y.as_str().parse::<i32>().ok());
            let Some(date) = resolve_md(m, d, year, received, allow_past) else {
                continue;
            };
            let wd = c.name("wd").and_then(|w| weekday_of(w.as_str()));
            found.push(with_time(
                text,
                DatePhrase {
                    span: range,
                    date,
                    time: None,
                    end_time: None,
                    kind: PhraseKind::Explicit,
                    weekday_mismatch: wd.is_some_and(|w| w != date.weekday()),
                    had_weekday: wd.is_some(),
                },
            ));
        }
    }

    for c in NUMERIC.captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        let start = c
            .name("wd")
            .or_else(|| c.name("iso_y"))
            .or_else(|| c.name("a"))
            .map_or(whole.start(), |m| m.start());
        let range = start..whole.end();
        if overlaps(&found, &range) {
            continue;
        }
        if c.name("a").is_some() && text[..start].ends_with(['/', '.']) {
            continue;
        }
        let (date, has_year) = if let Some(y) = c.name("iso_y") {
            let (Ok(y), Ok(m), Ok(d)) = (
                y.as_str().parse::<i32>(),
                c.name("iso_m").map_or("", |m| m.as_str()).parse::<u32>(),
                c.name("iso_d").map_or("", |m| m.as_str()).parse::<u32>(),
            ) else {
                continue;
            };
            let Some(date) = resolve_md(m, d, Some(y), received, allow_past) else {
                continue;
            };
            (date, true)
        } else {
            let (Some(a), Some(b)) = (c.name("a"), c.name("b")) else {
                continue;
            };
            let (Ok(a), Ok(b)) = (a.as_str().parse::<u32>(), b.as_str().parse::<u32>()) else {
                continue;
            };
            // US M/D, and D/M only when the first number cannot be a month.
            let (m, d) = if a > 12 { (b, a) } else { (a, b) };
            if m > 12 || d > 31 {
                continue;
            }
            let year = c.name("y").and_then(|y| {
                let n: i32 = y.as_str().parse().ok()?;
                Some(if y.as_str().len() == 2 { 2000 + n } else { n })
            });
            let Some(date) = resolve_md(m, d, year, received, allow_past) else {
                continue;
            };
            (date, year.is_some())
        };
        let wd = c.name("wd").and_then(|w| weekday_of(w.as_str()));
        let p = with_time(
            text,
            DatePhrase {
                span: range,
                date,
                time: None,
                end_time: None,
                kind: PhraseKind::Numeric,
                weekday_mismatch: wd.is_some_and(|w| w != date.weekday()),
                had_weekday: wd.is_some(),
            },
        );
        // A bare "1/2" is too likely to be a fraction.
        if p.had_weekday || p.time.is_some() || has_year {
            found.push(p);
        }
    }

    for c in RELATIVE.captures_iter(text) {
        let Some(m) = c.get(0) else { continue };
        if overlaps(&found, &m.range()) {
            continue;
        }
        let date = match c["rel"].to_lowercase().as_str() {
            "tomorrow" => received + Duration::days(1),
            _ => received,
        };
        found.push(with_time(
            text,
            DatePhrase {
                span: m.range(),
                date,
                time: None,
                end_time: None,
                kind: PhraseKind::Relative,
                weekday_mismatch: false,
                had_weekday: false,
            },
        ));
    }

    for c in WEEKDAY.captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        if overlaps(&found, &whole.range()) {
            continue;
        }
        let Some(wd) = c.name("wd").and_then(|w| weekday_of(w.as_str())) else {
            continue;
        };
        let ahead = (i64::from(wd.num_days_from_monday())
            - i64::from(received.weekday().num_days_from_monday()))
        .rem_euclid(7);
        let is_next = c
            .name("next")
            .is_some_and(|n| n.as_str().eq_ignore_ascii_case("next"));
        let date = if is_next {
            let d = received + Duration::days(if ahead == 0 { 7 } else { ahead });
            if d.iso_week() == received.iso_week() {
                d + Duration::days(7)
            } else {
                d
            }
        } else {
            received + Duration::days(ahead)
        };
        found.push(with_time(
            text,
            DatePhrase {
                span: whole.range(),
                date,
                time: None,
                end_time: None,
                kind: PhraseKind::Weekday,
                weekday_mismatch: false,
                had_weekday: true,
            },
        ));
    }

    found.sort_by_key(|p| p.span.start);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, TimeZone};

    fn ny() -> Tz {
        chrono_tz::America::New_York
    }

    fn at(s: &str, tz: Tz) -> DateTime<Utc> {
        let n = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        tz.from_local_datetime(&n).unwrap().with_timezone(&Utc)
    }

    fn one(text: &str, recv: &str) -> DatePhrase {
        let v = find_dates(text, at(recv, ny()), ny(), false);
        assert_eq!(v.len(), 1, "{text}: {v:?}");
        v.into_iter().next().unwrap()
    }

    fn d(y: i32, m: u32, dd: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, dd).unwrap()
    }
    fn t(h: u32, m: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(h, m, 0)
    }

    #[test]
    fn phrase_table() {
        // 2026-10-28 is a Wednesday.
        let r = "2026-10-28 09:00";
        let cases: &[(&str, NaiveDate, Option<NaiveTime>, Option<NaiveTime>)] = &[
            ("Friday, Nov 7 at 6pm", d(2026, 11, 7), t(18, 0), None),
            (
                "Fri Nov 7, 6:00-7:30 pm",
                d(2026, 11, 7),
                t(18, 0),
                t(19, 30),
            ),
            ("see you November 7th", d(2026, 11, 7), None, None),
            ("7 November at 6:30pm", d(2026, 11, 7), t(18, 30), None),
            ("11/7 6:30–8pm", d(2026, 11, 7), t(18, 30), t(20, 0)),
            ("on 11/7/2026 at 9am", d(2026, 11, 7), t(9, 0), None),
            ("Friday 11/7", d(2026, 11, 7), None, None),
            ("17/11 at 18:00", d(2026, 11, 17), t(18, 0), None),
            ("tomorrow at 3", d(2026, 10, 29), t(15, 0), None),
            ("today at 8:15", d(2026, 10, 28), t(8, 15), None),
            ("tonight at 7pm", d(2026, 10, 28), t(19, 0), None),
            ("next Friday", d(2026, 11, 6), None, None),
            ("this Friday at 5", d(2026, 10, 30), t(17, 0), None),
            ("Nov 7 11-1pm", d(2026, 11, 7), t(11, 0), t(13, 0)),
            ("Nov 7 at noonish 9", d(2026, 11, 7), None, None),
            ("2026-11-07 at 4pm", d(2026, 11, 7), t(16, 0), None),
        ];
        for (text, date, time, end) in cases {
            let p = one(text, r);
            assert_eq!((p.date, p.time, p.end_time), (*date, *time, *end), "{text}");
        }
    }

    #[test]
    fn rejects() {
        let r = at("2026-10-28 09:00", ny());
        // Bare fraction, a past date mid-year, and a number after a date that is not a time.
        assert!(find_dates("add 1/2 cup", r, ny(), false).is_empty());
        assert!(find_dates("on March 3", r, ny(), false).is_empty());
        assert!(find_dates("30/30", r, ny(), false).is_empty());
        let p = one("Nov 7 3 children", "2026-10-28 09:00");
        assert!(p.time.is_none());
    }

    #[test]
    fn year_rollover() {
        // Received in December: "Jan 9" is next January.
        let p = one("Jan 9 at 6pm", "2026-12-20 09:00");
        assert_eq!(p.date, d(2027, 1, 9));
        // Received in March: February is past and dropped.
        let r = at("2026-03-20 09:00", ny());
        assert!(find_dates("Feb 9", r, ny(), false).is_empty());
        assert_eq!(find_dates("Feb 9", r, ny(), true).len(), 1);
    }

    #[test]
    fn weekday_mismatch_flag() {
        // Nov 7 2026 is a Saturday.
        let p = one("Friday, Nov 7", "2026-10-28 09:00");
        assert!(p.weekday_mismatch && p.had_weekday);
        let p = one("Saturday, Nov 7", "2026-10-28 09:00");
        assert!(!p.weekday_mismatch);
    }

    #[test]
    fn quoted_lines_skipped() {
        let r = at("2026-10-28 09:00", ny());
        assert!(find_dates("> Friday Nov 7 at 6pm\nthanks", r, ny(), false).is_empty());
    }

    #[test]
    fn relative_uses_local_date() {
        // 2026-10-29 02:00 UTC is still Oct 28 in New York.
        let r = Utc.with_ymd_and_hms(2026, 10, 29, 2, 0, 0).unwrap();
        let v = find_dates("tomorrow at 3pm", r, ny(), false);
        assert_eq!(v[0].date, d(2026, 10, 29));
    }
}
