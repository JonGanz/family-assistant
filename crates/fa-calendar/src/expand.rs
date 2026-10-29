//! Turns parsed ICS components (CalDAV and ICS feeds) into one `NewOccurrence` per occurrence.
//!
//! Recurrences are expanded on the *wall clock* of the event's own zone (as RFC 5545 requires): the
//! rule runs on naive times, and each result is mapped back to an instant afterwards. A weekly 17:00
//! event therefore stays at 17:00 local across a DST change while its UTC time shifts.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{
    DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc,
};
use chrono_tz::Tz;
use fa_connectors::{IcsTime, ParsedComponent};
use fa_store::NewOccurrence;

use crate::tz_windows::windows_to_iana;

/// Hard cap per component, so a hostile `FREQ=SECONDLY` rule can't flood the database.
const MAX_PER_COMPONENT: usize = 5000;

#[derive(Debug, Clone)]
pub struct ExpandOptions {
    pub household_tz: Tz,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct Expansion {
    pub occurrences: Vec<NewOccurrence>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy)]
struct Wall {
    naive: NaiveDateTime,
    tz: Tz,
}

enum Parsed {
    Date(NaiveDate),
    Wall(Wall),
}

struct Ctx<'a> {
    opts: &'a ExpandOptions,
    warnings: BTreeSet<String>,
}

pub fn local_to_utc(naive: NaiveDateTime, tz: Tz) -> DateTime<Utc> {
    use chrono::LocalResult;
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(d) => d.with_timezone(&Utc),
        LocalResult::Ambiguous(a, _) => a.with_timezone(&Utc),
        LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| Utc.from_utc_datetime(&naive)),
    }
}

fn midnight(d: NaiveDate) -> NaiveDateTime {
    d.and_time(NaiveTime::MIN)
}

fn key_of_instant(i: DateTime<Utc>) -> String {
    i.to_rfc3339_opts(SecondsFormat::Secs, true)
}

impl Ctx<'_> {
    fn zone(&mut self, tzid: Option<&str>) -> Tz {
        let Some(raw) = tzid else {
            return self.opts.household_tz;
        };
        let t = raw.trim().trim_matches('"');
        if let Ok(z) = t.parse::<Tz>() {
            return z;
        }
        if let Some(z) = windows_to_iana(t).and_then(|n| n.parse::<Tz>().ok()) {
            return z;
        }
        // Path-style ids such as "/mozilla.org/20050126_1/Europe/Berlin".
        let segs: Vec<&str> = t.split('/').filter(|s| !s.is_empty()).collect();
        for n in [2usize, 1] {
            if segs.len() >= n
                && let Ok(z) = segs[segs.len() - n..].join("/").parse::<Tz>()
            {
                return z;
            }
        }
        self.warnings.insert(format!(
            "unknown time zone \"{}\"; used the household time zone",
            t.chars().take(60).collect::<String>()
        ));
        self.opts.household_tz
    }

    fn parse(&mut self, t: &IcsTime) -> Option<Parsed> {
        let v = t.value.trim();
        let date_part = v.get(..8)?;
        let date = NaiveDate::parse_from_str(date_part, "%Y%m%d").ok()?;
        if t.is_date || v.len() == 8 {
            return Some(Parsed::Date(date));
        }
        let utc = v.ends_with(['Z', 'z']);
        let body = v.trim_end_matches(['Z', 'z']);
        let naive = NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M%S")
            .or_else(|_| NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M"))
            .ok()?;
        let tz = if utc {
            Tz::UTC
        } else {
            self.zone(t.tzid.as_deref())
        };
        Some(Parsed::Wall(Wall { naive, tz }))
    }

    /// Wall time of `p` in `tz` (an instant is converted; a date becomes local midnight).
    fn naive_in(&self, p: &Parsed, tz: Tz) -> NaiveDateTime {
        match p {
            Parsed::Date(d) => midnight(*d),
            Parsed::Wall(w) if w.tz == tz => w.naive,
            Parsed::Wall(w) => local_to_utc(w.naive, w.tz).with_timezone(&tz).naive_local(),
        }
    }

    fn instant(&self, p: &Parsed) -> DateTime<Utc> {
        match p {
            Parsed::Date(d) => local_to_utc(midnight(*d), self.opts.household_tz),
            Parsed::Wall(w) => local_to_utc(w.naive, w.tz),
        }
    }
}

/// `P1D`, `PT1H30M`, `P2W`, `-PT15M`.
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let s = s.strip_prefix('P')?;
    let mut total = Duration::zero();
    let mut num = String::new();
    let mut in_time = false;
    for c in s.chars() {
        match c {
            'T' => in_time = true,
            '0'..='9' => num.push(c),
            _ => {
                let n: i64 = num.parse().ok()?;
                num.clear();
                total += match (c, in_time) {
                    ('W', false) => Duration::weeks(n),
                    ('D', false) => Duration::days(n),
                    ('H', true) => Duration::hours(n),
                    ('M', true) => Duration::minutes(n),
                    ('S', true) => Duration::seconds(n),
                    _ => return None,
                };
            }
        }
    }
    if !num.is_empty() {
        return None;
    }
    Some(if neg { -total } else { total })
}

/// Rewrites `UNTIL` as a naive time in the event's wall clock (suffixed `Z`, since the rule runs on
/// naive-as-UTC times).
fn fix_rule(rule: &str, tz: Tz, ctx: &mut Ctx<'_>) -> String {
    let rule = rule.trim().trim_start_matches("RRULE:");
    rule.split(';')
        .filter(|p| !p.is_empty())
        .map(|part| {
            let Some(v) = part
                .strip_prefix("UNTIL=")
                .or_else(|| part.strip_prefix("until="))
            else {
                return part.to_owned();
            };
            let t = IcsTime {
                value: v.to_owned(),
                tzid: None,
                is_date: v.len() == 8,
            };
            let naive = match ctx.parse(&t) {
                Some(Parsed::Date(d)) => d.and_hms_opt(23, 59, 59).unwrap_or_default(),
                Some(p @ Parsed::Wall(_)) => {
                    // A `Z` value is an instant, a floating one is already wall time.
                    if v.ends_with(['Z', 'z']) {
                        ctx.naive_in(&p, tz)
                    } else if let Parsed::Wall(w) = p {
                        w.naive
                    } else {
                        NaiveDateTime::default()
                    }
                }
                None => return part.to_owned(),
            };
            format!("UNTIL={}Z", naive.format("%Y%m%dT%H%M%S"))
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn rule_starts(
    rule: &str,
    base: NaiveDateTime,
    lo: NaiveDateTime,
    hi: NaiveDateTime,
    ctx: &mut Ctx<'_>,
) -> Vec<NaiveDateTime> {
    use rrule::{RRuleSet, Tz as RTz};
    // A series cut off before its own start (providers do this on "this and following" edits) has no
    // occurrences; the rrule crate rejects it as invalid, so don't hand it over.
    let until = rule.split(';').find_map(|p| p.strip_prefix("UNTIL="));
    if let Some(u) = until.and_then(|u| NaiveDateTime::parse_from_str(u, "%Y%m%dT%H%M%SZ").ok())
        && u < base
    {
        return Vec::new();
    }
    let text = format!("DTSTART:{}Z\nRRULE:{rule}", base.format("%Y%m%dT%H%M%S"));
    let set: RRuleSet = match text.parse() {
        Ok(s) => s,
        Err(e) => {
            ctx.warnings
                .insert(format!("unsupported recurrence rule ignored: {e}"));
            return Vec::new();
        }
    };
    let res = set
        .after(RTz::UTC.from_utc_datetime(&lo))
        .before(RTz::UTC.from_utc_datetime(&hi))
        .all(u16::MAX);
    res.dates.into_iter().map(|d| d.naive_local()).collect()
}

struct Timing {
    all_day: bool,
    tz: Tz,
    base: NaiveDateTime,
    /// All-day: whole days; timed: the absolute length.
    span_days: i64,
    dur: Duration,
}

fn timing(c: &ParsedComponent, ctx: &mut Ctx<'_>) -> Option<Timing> {
    let start = ctx.parse(c.dtstart.as_ref()?)?;
    let dtend = c.dtend.as_ref().and_then(|t| ctx.parse(t));
    let dur_prop = c.duration.as_deref().and_then(parse_duration);
    Some(match start {
        Parsed::Date(d) => {
            let days = match (&dtend, dur_prop) {
                (Some(e), _) => {
                    let end = match e {
                        Parsed::Date(x) => *x,
                        Parsed::Wall(w) => w.naive.date(),
                    };
                    (end - d).num_days()
                }
                (None, Some(p)) => p.num_days(),
                (None, None) => 1,
            }
            .max(1);
            Timing {
                all_day: true,
                tz: Tz::UTC,
                base: midnight(d),
                span_days: days,
                dur: Duration::zero(),
            }
        }
        Parsed::Wall(w) => {
            let s = local_to_utc(w.naive, w.tz);
            let dur = match (&dtend, dur_prop) {
                (Some(e), _) => ctx.instant(e) - s,
                (None, Some(p)) => p,
                (None, None) => Duration::zero(),
            }
            .max(Duration::zero());
            Timing {
                all_day: false,
                tz: w.tz,
                base: w.naive,
                span_days: 0,
                dur,
            }
        }
    })
}

fn recurrence_key(rid: &IcsTime, ctx: &mut Ctx<'_>) -> Option<String> {
    Some(match ctx.parse(rid)? {
        Parsed::Date(d) => d.to_string(),
        Parsed::Wall(w) => key_of_instant(local_to_utc(w.naive, w.tz)),
    })
}

struct Built<'a> {
    c: &'a ParsedComponent,
    uid: &'a str,
}

impl Built<'_> {
    fn occurrence(
        &self,
        t: &Timing,
        start: NaiveDateTime,
        key: String,
        opts: &ExpandOptions,
    ) -> NewOccurrence {
        let (start_at, end_at, start_date, end_date) = if t.all_day {
            let sd = start.date();
            let ed = sd + Duration::days(t.span_days);
            (
                local_to_utc(midnight(sd), opts.household_tz),
                local_to_utc(midnight(ed), opts.household_tz),
                Some(sd),
                Some(ed),
            )
        } else {
            let s = local_to_utc(start, t.tz);
            (s, s + t.dur, None, None)
        };
        NewOccurrence {
            remote_uid: self.uid.to_owned(),
            instance_key: key,
            title: self.c.summary.clone().unwrap_or_default(),
            description: self.c.description.clone().filter(|s| !s.is_empty()),
            location: self.c.location.clone().filter(|s| !s.is_empty()),
            start_at,
            end_at,
            all_day: t.all_day,
            start_date,
            end_date,
            source_tz: (!t.all_day && t.tz != Tz::UTC).then(|| t.tz.name().to_owned()),
            is_private: self.c.class.as_deref().is_some_and(|k| {
                k.eq_ignore_ascii_case("PRIVATE") || k.eq_ignore_ascii_case("CONFIDENTIAL")
            }),
            cancelled: false,
            remote_etag: None,
            external_url: None,
        }
    }
}

fn overlaps(o: &NewOccurrence, opts: &ExpandOptions) -> bool {
    o.start_at < opts.to && (o.end_at > opts.from || o.start_at >= opts.from)
}

fn is_cancelled(c: &ParsedComponent) -> bool {
    c.status
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("CANCELLED"))
}

pub fn expand(components: &[ParsedComponent], opts: &ExpandOptions) -> Expansion {
    let mut ctx = Ctx {
        opts,
        warnings: BTreeSet::new(),
    };
    let mut out: BTreeMap<(String, String), NewOccurrence> = BTreeMap::new();

    // Override keys per UID; they replace (or, when cancelled, remove) the master's occurrence.
    let mut overridden: HashMap<&str, HashSet<String>> = HashMap::new();
    let mut overrides: Vec<(&ParsedComponent, String)> = Vec::new();
    for c in components {
        let (Some(uid), Some(rid)) = (c.uid.as_deref(), c.recurrence_id.as_ref()) else {
            continue;
        };
        if let Some(k) = recurrence_key(rid, &mut ctx) {
            overridden.entry(uid).or_default().insert(k.clone());
            overrides.push((c, k));
        }
    }

    for (c, key) in overrides {
        if is_cancelled(c) {
            continue;
        }
        let Some(uid) = c.uid.as_deref() else {
            continue;
        };
        let Some(t) = timing(c, &mut ctx) else {
            continue;
        };
        let o = Built { c, uid }.occurrence(&t, t.base, key.clone(), opts);
        if overlaps(&o, opts) {
            out.insert((uid.to_owned(), key), o);
        }
    }

    for c in components.iter().filter(|c| c.recurrence_id.is_none()) {
        let Some(uid) = c.uid.as_deref() else {
            continue;
        };
        if is_cancelled(c) {
            continue;
        }
        let Some(t) = timing(c, &mut ctx) else {
            continue;
        };
        let recurring = c.rrule.is_some() || !c.rdate.is_empty();
        let mut starts: BTreeSet<NaiveDateTime> = BTreeSet::from([t.base]);
        if recurring {
            let (lo, hi) = {
                let f = if t.all_day {
                    opts.from.naive_utc()
                } else {
                    opts.from.with_timezone(&t.tz).naive_local()
                };
                let e = if t.all_day {
                    opts.to.naive_utc()
                } else {
                    opts.to.with_timezone(&t.tz).naive_local()
                };
                let back = t.dur + Duration::days(t.span_days + 2);
                (f - back, e + Duration::days(2))
            };
            if let Some(rule) = &c.rrule {
                let rule = fix_rule(rule, t.tz, &mut ctx);
                starts.extend(rule_starts(&rule, t.base, lo, hi, &mut ctx));
            }
            for r in &c.rdate {
                // Periods ("start/end") are reduced to their start.
                let r = IcsTime {
                    value: r.value.split('/').next().unwrap_or_default().to_owned(),
                    ..r.clone()
                };
                if let Some(p) = ctx.parse(&r) {
                    starts.insert(ctx.naive_in(&p, t.tz));
                }
            }
            let mut ex_naive = BTreeSet::new();
            let mut ex_days = BTreeSet::new();
            for e in &c.exdate {
                match ctx.parse(e) {
                    Some(Parsed::Date(d)) => {
                        ex_days.insert(d);
                    }
                    Some(p) => {
                        ex_naive.insert(ctx.naive_in(&p, t.tz));
                    }
                    None => {}
                }
            }
            starts.retain(|s| !ex_naive.contains(s) && !ex_days.contains(&s.date()));
        }
        let skip = overridden.get(uid);
        let b = Built { c, uid };
        for s in starts.into_iter().take(MAX_PER_COMPONENT) {
            let key = if !recurring {
                String::new()
            } else if t.all_day {
                s.date().to_string()
            } else {
                key_of_instant(local_to_utc(s, t.tz))
            };
            if skip.is_some_and(|k| k.contains(&key)) {
                continue;
            }
            let o = b.occurrence(&t, s, key.clone(), opts);
            if overlaps(&o, opts) {
                out.insert((uid.to_owned(), key), o);
            }
        }
    }

    Expansion {
        occurrences: out.into_values().collect(),
        warnings: ctx.warnings.into_iter().collect(),
    }
}
