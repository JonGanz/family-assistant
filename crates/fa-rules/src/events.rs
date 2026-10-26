//! Event extraction: ICS, schema.org (JSON-LD and microdata) and date phrases (Ingest.md 7).

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use fa_domain::tasks::local_to_utc;
use icalendar::parser::{self, Component};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;

use crate::dates::{PhraseKind, find_dates};
use crate::text::{sentence_around, truncate_chars};
use crate::types::{Context, MessageFacts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    Ics,
    SchemaOrg,
    DatePhrase,
}

/// All-day events: `start` and `end` are local midnights in the household zone, `end`
/// exclusive (at least one day).
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedEvent {
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub all_day: bool,
    pub location: Option<String>,
    pub uid: Option<String>,
    /// `METHOD:CANCEL`, `STATUS:CANCELLED` or schema.org `EventCancelled`.
    pub cancel: bool,
    pub confidence: f32,
    pub source: EventSource,
}

const ICS_CONF: f32 = 0.95;
const SCHEMA_CONF: f32 = 0.9;
const MAX_PHRASE_EVENTS: usize = 10;

fn midnight(date: NaiveDate, tz: Tz) -> DateTime<Utc> {
    local_to_utc(date.and_time(NaiveTime::MIN), tz)
}

fn clean_subject(s: &str) -> String {
    let mut s = s.trim();
    loop {
        let l = s.to_lowercase();
        let cut = ["re:", "fwd:", "fw:"]
            .iter()
            .find(|p| l.starts_with(**p))
            .map(|p| p.len());
        match cut {
            Some(n) => s = s[n..].trim_start(),
            None => break,
        }
    }
    s.to_owned()
}

pub fn extract_events(f: &MessageFacts<'_>, ctx: &Context) -> Vec<ExtractedEvent> {
    let mut out: Vec<ExtractedEvent> = Vec::new();
    for a in f.attachments {
        if let Some(ics) = &a.ics {
            out.extend(ics_events(ics, f.subject, ctx.tz));
        }
    }
    if let Some(html) = f.body_html {
        out.extend(schema_events(html, f.subject, ctx.tz));
    }
    // The same event often arrives as ICS and as markup.
    let mut seen: Vec<(String, DateTime<Utc>, bool)> = Vec::new();
    out.retain(|e| {
        let key = (
            e.uid.clone().unwrap_or_else(|| e.title.to_lowercase()),
            e.start,
            e.cancel,
        );
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    if out.is_empty() {
        out = phrase_events(f, ctx);
    }
    out
}

// ---- ICS ----

fn prop<'a>(c: &'a Component<'_>, name: &str) -> Option<&'a icalendar::parser::Property<'a>> {
    c.properties
        .iter()
        .find(|p| p.name.as_ref().eq_ignore_ascii_case(name))
}

fn prop_text(c: &Component<'_>, name: &str) -> Option<String> {
    prop(c, name)
        .map(|p| p.val.clone().unescape_text().as_ref().trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn param(p: &icalendar::parser::Property<'_>, key: &str) -> Option<String> {
    p.params
        .iter()
        .find(|q| q.key.as_ref().eq_ignore_ascii_case(key))
        .and_then(|q| q.val.as_ref())
        .map(|v| v.as_ref().trim_matches('"').to_owned())
}

fn tz_from_name(name: &str) -> Option<Tz> {
    let n = name.trim().trim_matches('"');
    n.parse::<Tz>().ok().or_else(|| {
        // The common Windows zone names.
        let mapped = match n.to_lowercase().as_str() {
            "eastern standard time" | "us eastern standard time" => "America/New_York",
            "central standard time" => "America/Chicago",
            "mountain standard time" => "America/Denver",
            "pacific standard time" => "America/Los_Angeles",
            "gmt standard time" => "Europe/London",
            "w. europe standard time" | "central europe standard time" => "Europe/Berlin",
            _ => return None,
        };
        mapped.parse().ok()
    })
}

/// `(instant, all_day)`
fn ics_time(p: &icalendar::parser::Property<'_>, default_tz: Tz) -> Option<(DateTime<Utc>, bool)> {
    let v = p.val.as_ref().trim();
    let is_date = param(p, "VALUE").is_some_and(|x| x.eq_ignore_ascii_case("DATE"))
        || (v.len() == 8 && v.bytes().all(|b| b.is_ascii_digit()));
    if is_date {
        let d = NaiveDate::parse_from_str(v, "%Y%m%d").ok()?;
        return Some((midnight(d, default_tz), true));
    }
    if let Some(z) = v.strip_suffix(['Z', 'z']) {
        let n = NaiveDateTime::parse_from_str(z, "%Y%m%dT%H%M%S").ok()?;
        return Some((n.and_utc(), false));
    }
    let n = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    let tz = param(p, "TZID")
        .and_then(|t| tz_from_name(&t))
        .unwrap_or(default_tz);
    Some((local_to_utc(n, tz), false))
}

fn ics_duration(s: &str) -> Option<Duration> {
    let s = s.trim().trim_start_matches('+');
    let rest = s.strip_prefix('P')?;
    let (date_part, time_part) = rest.split_once('T').unwrap_or((rest, ""));
    let mut total = Duration::zero();
    let mut num = String::new();
    for ch in date_part.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else {
            let n: i64 = num.parse().ok()?;
            num.clear();
            total += match ch {
                'W' => Duration::weeks(n),
                'D' => Duration::days(n),
                _ => return None,
            };
        }
    }
    for ch in time_part.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else {
            let n: i64 = num.parse().ok()?;
            num.clear();
            total += match ch {
                'H' => Duration::hours(n),
                'M' => Duration::minutes(n),
                'S' => Duration::seconds(n),
                _ => return None,
            };
        }
    }
    Some(total)
}

fn ics_events(body: &str, subject: &str, tz: Tz) -> Vec<ExtractedEvent> {
    let unfolded = parser::unfold(body);
    let Ok(comps) = parser::read_components(&unfolded) else {
        return vec![];
    };
    let mut out = Vec::new();
    for root in &comps {
        let method_cancel =
            prop_text(root, "METHOD").is_some_and(|m| m.eq_ignore_ascii_case("CANCEL"));
        let events: Vec<&Component<'_>> = if root.name.as_ref().eq_ignore_ascii_case("VEVENT") {
            vec![root]
        } else {
            root.components
                .iter()
                .filter(|c| c.name.as_ref().eq_ignore_ascii_case("VEVENT"))
                .collect()
        };
        for c in events {
            let Some((start, all_day)) = prop(c, "DTSTART").and_then(|p| ics_time(p, tz)) else {
                continue;
            };
            let end = match prop(c, "DTEND").and_then(|p| ics_time(p, tz)) {
                Some((e, _)) if e > start => Some(e),
                _ => prop_text(c, "DURATION")
                    .and_then(|d| ics_duration(&d))
                    .map(|d| start + d),
            };
            let end = if all_day {
                let min_end = start.with_timezone(&tz).date_naive() + Duration::days(1);
                Some(end.unwrap_or_else(|| midnight(min_end, tz)))
            } else {
                end
            };
            let status_cancel =
                prop_text(c, "STATUS").is_some_and(|s| s.eq_ignore_ascii_case("CANCELLED"));
            out.push(ExtractedEvent {
                title: prop_text(c, "SUMMARY").unwrap_or_else(|| clean_subject(subject)),
                start,
                end,
                all_day,
                location: prop_text(c, "LOCATION"),
                uid: prop_text(c, "UID"),
                cancel: method_cancel || status_cancel,
                confidence: ICS_CONF,
                source: EventSource::Ics,
            });
        }
    }
    out
}

// ---- schema.org ----

#[derive(Default, Debug)]
struct Raw {
    name: Option<String>,
    start: Option<String>,
    end: Option<String>,
    location: Option<String>,
    cancelled: bool,
    inclusive_end: bool,
}

const RESERVATIONS: [&str; 4] = [
    "EventReservation",
    "FoodEstablishmentReservation",
    "LodgingReservation",
    "FlightReservation",
];

fn is_event_type(t: &str) -> bool {
    t == "Event" || t.ends_with("Event")
}

fn is_wanted(t: &str) -> bool {
    is_event_type(t) || RESERVATIONS.contains(&t)
}

fn jstr(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn jplace(v: &Value) -> Option<String> {
    match v {
        Value::String(_) => jstr(v),
        Value::Array(a) => a.iter().find_map(jplace),
        Value::Object(o) => {
            let name = o.get("name").and_then(jstr);
            let street = |ao: &serde_json::Map<String, Value>| {
                let parts: Vec<String> = ["streetAddress", "addressLocality", "addressRegion"]
                    .iter()
                    .filter_map(|k| ao.get(*k).and_then(jstr))
                    .collect();
                (!parts.is_empty()).then(|| parts.join(", "))
            };
            let addr = match o.get("address") {
                Some(Value::Object(ao)) => street(ao),
                Some(other) => jstr(other),
                // The object is itself a PostalAddress.
                None => street(o),
            };
            match (name, addr) {
                (Some(n), Some(a)) => Some(format!("{n}, {a}")),
                (n, a) => n.or(a),
            }
        }
        _ => None,
    }
}

fn from_json(v: &Value, ty: &str) -> Option<Raw> {
    let get = |o: &Value, k: &str| o.get(k).and_then(jstr);
    let inner = v.get("reservationFor").filter(|r| r.is_object());
    let r = match ty {
        "FlightReservation" => {
            let f = inner?;
            let code = f
                .get("airline")
                .and_then(|a| a.get("iataCode").or_else(|| a.get("name")))
                .and_then(jstr)
                .unwrap_or_default();
            let num = get(f, "flightNumber").unwrap_or_default();
            let air = |k: &str| {
                f.get(k)
                    .and_then(|a| a.get("iataCode").or_else(|| a.get("name")))
                    .and_then(jstr)
            };
            let route = match (air("departureAirport"), air("arrivalAirport")) {
                (Some(a), Some(b)) => format!(" {a} to {b}"),
                _ => String::new(),
            };
            Raw {
                name: Some(format!("Flight {code}{num}{route}").replace("  ", " ")),
                start: get(f, "departureTime"),
                end: get(f, "arrivalTime"),
                location: f.get("departureAirport").and_then(jplace),
                ..Raw::default()
            }
        }
        "LodgingReservation" => Raw {
            name: inner
                .and_then(|i| get(i, "name"))
                .or_else(|| get(v, "name")),
            start: get(v, "checkinTime").or_else(|| get(v, "checkinDate")),
            end: get(v, "checkoutTime").or_else(|| get(v, "checkoutDate")),
            location: inner.and_then(|i| i.get("address").and_then(jplace)),
            ..Raw::default()
        },
        t if RESERVATIONS.contains(&t) => Raw {
            name: inner
                .and_then(|i| get(i, "name"))
                .or_else(|| get(v, "name")),
            start: get(v, "startTime")
                .or_else(|| inner.and_then(|i| get(i, "startDate")))
                .or_else(|| get(v, "startDate")),
            end: get(v, "endTime").or_else(|| inner.and_then(|i| get(i, "endDate"))),
            location: inner.and_then(|i| {
                i.get("location")
                    .or_else(|| i.get("address"))
                    .and_then(jplace)
            }),
            ..Raw::default()
        },
        _ => Raw {
            name: get(v, "name"),
            start: get(v, "startDate"),
            end: get(v, "endDate"),
            location: v.get("location").and_then(jplace),
            cancelled: v
                .get("eventStatus")
                .and_then(jstr)
                .is_some_and(|s| s.to_lowercase().contains("cancel")),
            inclusive_end: true,
        },
    };
    Some(r)
}

fn types_of(v: &Value) -> Vec<String> {
    match v.get("@type") {
        Some(Value::String(s)) => vec![s.rsplit('/').next().unwrap_or(s).to_owned()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|t| t.as_str())
            .map(|s| s.rsplit('/').next().unwrap_or(s).to_owned())
            .collect(),
        _ => vec![],
    }
}

fn visit_json(v: &Value, out: &mut Vec<Raw>, depth: usize) {
    if depth > 6 {
        return;
    }
    match v {
        Value::Array(a) => a.iter().for_each(|x| visit_json(x, out, depth + 1)),
        Value::Object(o) => {
            if let Some(g) = o.get("@graph") {
                visit_json(g, out, depth + 1);
            }
            if let Some(t) = types_of(v).into_iter().find(|t| is_wanted(t))
                && let Some(r) = from_json(v, &t)
            {
                out.push(r);
            }
        }
        _ => {}
    }
}

fn selector(s: &str) -> Option<Selector> {
    Selector::parse(s).ok()
}

fn el_text(el: ElementRef<'_>) -> String {
    crate::text::collapse_ws(&el.text().collect::<Vec<_>>().join(" "))
}

/// Value of a microdata property element.
fn item_value(el: ElementRef<'_>) -> String {
    let a = el.value();
    for k in ["content", "datetime"] {
        if let Some(v) = a.attr(k) {
            return v.trim().to_owned();
        }
    }
    if (a.name() == "a" || a.name() == "link")
        && let Some(v) = a.attr("href")
    {
        return v.trim().to_owned();
    }
    if a.attr("itemscope").is_some()
        && let Some(sel) = selector("[itemprop=name]")
        && let Some(n) = el.select(&sel).next()
    {
        return item_value(n);
    }
    el_text(el)
}

/// Properties whose nearest `itemscope` ancestor is `scope`.
fn props_of<'a>(scope: ElementRef<'a>) -> HashMap<String, ElementRef<'a>> {
    let mut map = HashMap::new();
    let Some(sel) = selector("[itemprop]") else {
        return map;
    };
    for p in scope.select(&sel) {
        let owner = p
            .ancestors()
            .filter_map(ElementRef::wrap)
            .find(|a| a.value().attr("itemscope").is_some());
        if owner.is_some_and(|o| o.id() == scope.id()) {
            for name in p
                .value()
                .attr("itemprop")
                .unwrap_or_default()
                .split_whitespace()
            {
                map.entry(name.to_owned()).or_insert(p);
            }
        }
    }
    map
}

fn microdata_events(doc: &Html, out: &mut Vec<Raw>) {
    let Some(sel) = selector("[itemtype]") else {
        return;
    };
    for el in doc.select(&sel) {
        let ty = el
            .value()
            .attr("itemtype")
            .unwrap_or_default()
            .split_whitespace()
            .find_map(|t| {
                let t = t.trim_end_matches('/').rsplit('/').next().unwrap_or(t);
                is_wanted(t).then(|| t.to_owned())
            });
        let Some(ty) = ty else { continue };
        let props = props_of(el);
        let val = |m: &HashMap<String, ElementRef<'_>>, k: &str| {
            m.get(k).map(|e| item_value(*e)).filter(|s| !s.is_empty())
        };
        let inner_el = props.get("reservationFor").copied();
        let inner = inner_el.map(props_of).unwrap_or_default();
        let raw = if is_event_type(&ty) {
            Raw {
                name: val(&props, "name"),
                start: val(&props, "startDate"),
                end: val(&props, "endDate"),
                location: val(&props, "location"),
                cancelled: val(&props, "eventStatus")
                    .is_some_and(|s| s.to_lowercase().contains("cancel")),
                inclusive_end: true,
            }
        } else if ty == "LodgingReservation" {
            Raw {
                name: val(&inner, "name").or_else(|| val(&props, "name")),
                start: val(&props, "checkinTime").or_else(|| val(&props, "checkinDate")),
                end: val(&props, "checkoutTime").or_else(|| val(&props, "checkoutDate")),
                location: val(&inner, "address"),
                ..Raw::default()
            }
        } else if ty == "FlightReservation" {
            Raw {
                name: val(&inner, "flightNumber").map(|n| format!("Flight {n}")),
                start: val(&inner, "departureTime"),
                end: val(&inner, "arrivalTime"),
                ..Raw::default()
            }
        } else {
            Raw {
                name: val(&inner, "name").or_else(|| val(&props, "name")),
                start: val(&props, "startTime")
                    .or_else(|| val(&inner, "startDate"))
                    .or_else(|| val(&props, "startDate")),
                end: val(&props, "endTime").or_else(|| val(&inner, "endDate")),
                location: val(&inner, "location").or_else(|| val(&inner, "address")),
                ..Raw::default()
            }
        };
        out.push(raw);
    }
}

fn schema_time(s: &str, tz: Tz) -> Option<(DateTime<Utc>, bool)> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some((d.with_timezone(&Utc), false));
    }
    if let Ok(d) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z") {
        return Some((d.with_timezone(&Utc), false));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some((local_to_utc(n, tz), false));
        }
    }
    let d = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some((midnight(d, tz), true))
}

fn schema_events(html: &str, subject: &str, tz: Tz) -> Vec<ExtractedEvent> {
    let doc = Html::parse_document(html);
    let mut raws: Vec<Raw> = Vec::new();
    if let Some(sel) = selector(r#"script[type="application/ld+json" i]"#) {
        for s in doc.select(&sel) {
            let text: String = s.text().collect();
            if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
                visit_json(&v, &mut raws, 0);
            }
        }
    }
    microdata_events(&doc, &mut raws);

    raws.into_iter()
        .filter_map(|r| {
            let (start, all_day) = r.start.as_deref().and_then(|s| schema_time(s, tz))?;
            let end = r.end.as_deref().and_then(|s| schema_time(s, tz));
            let end = if all_day {
                let sd = start.with_timezone(&tz).date_naive();
                let ed = end.map(|(e, _)| e.with_timezone(&tz).date_naive());
                let excl = match ed {
                    Some(d) if r.inclusive_end && d >= sd => d + Duration::days(1),
                    Some(d) if d > sd => d,
                    _ => sd + Duration::days(1),
                };
                Some(midnight(excl, tz))
            } else {
                end.map(|(e, _)| e).filter(|e| *e > start)
            };
            Some(ExtractedEvent {
                title: r.name.unwrap_or_else(|| clean_subject(subject)),
                start,
                end,
                all_day,
                location: r.location,
                uid: None,
                cancel: r.cancelled,
                confidence: SCHEMA_CONF,
                source: EventSource::SchemaOrg,
            })
        })
        .collect()
}

// ---- date phrases ----

fn phrase_events(f: &MessageFacts<'_>, ctx: &Context) -> Vec<ExtractedEvent> {
    let text = f.text();
    let tz = ctx.tz;
    let mut found: Vec<(crate::dates::DatePhrase, f32)> = Vec::new();
    for p in find_dates(&text, f.received_at, tz, false) {
        let needs_time = matches!(p.kind, PhraseKind::Relative | PhraseKind::Weekday);
        if needs_time && p.time.is_none() {
            continue;
        }
        let base = match (p.kind, p.time.is_some()) {
            (PhraseKind::Explicit, true) => 0.7,
            (PhraseKind::Explicit, false) if p.had_weekday => 0.6,
            (PhraseKind::Explicit, false) => 0.5,
            (PhraseKind::Numeric, true) => 0.6,
            (PhraseKind::Numeric, false) => 0.5,
            (PhraseKind::Relative, _) => 0.6,
            (PhraseKind::Weekday, _) => 0.55,
        };
        let conf = if p.weekday_mismatch {
            (base - 0.1f32).max(0.5)
        } else {
            base
        };
        if found
            .iter()
            .any(|(q, _)| q.date == p.date && q.time == p.time)
        {
            continue;
        }
        found.push((p, conf));
        if found.len() >= MAX_PHRASE_EVENTS {
            break;
        }
    }
    let single = found.len() == 1;
    let subject = clean_subject(f.subject);
    found
        .into_iter()
        .map(|(p, confidence)| {
            let title = if single && !subject.is_empty() {
                truncate_chars(&subject, 80)
            } else {
                let s = sentence_around(&text, p.span.start, p.span.end);
                truncate_chars(if s.is_empty() { &subject } else { s }, 80)
            };
            let (start, end, all_day) = match p.time {
                Some(t) => {
                    let start = local_to_utc(p.date.and_time(t), tz);
                    let end = p.end_time.map(|e| {
                        let mut end = local_to_utc(p.date.and_time(e), tz);
                        if end <= start {
                            end = local_to_utc((p.date + Duration::days(1)).and_time(e), tz);
                        }
                        end
                    });
                    (start, end, false)
                }
                None => (
                    midnight(p.date, tz),
                    Some(midnight(p.date + Duration::days(1), tz)),
                    true,
                ),
            };
            ExtractedEvent {
                title,
                start,
                end,
                all_day,
                location: None,
                uid: None,
                cancel: false,
                confidence,
                source: EventSource::DatePhrase,
            }
        })
        .collect()
}
