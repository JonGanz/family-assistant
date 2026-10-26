use icalendar::parser::{self, Component, Property};
use reqwest::header;

use crate::{ConnectorError, Http, IcsTime, ParsedComponent, error::Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub body: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IcsFetch {
    Fetched(Fetched),
    NotModified,
}

pub fn normalize_url(url: &str) -> String {
    match url.get(..9) {
        Some(p) if p.eq_ignore_ascii_case("webcal://") => format!("https://{}", &url[9..]),
        _ => url.to_owned(),
    }
}

pub async fn fetch(
    http: &Http,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<IcsFetch> {
    let url = http.parse_url(&normalize_url(url))?;
    let mut rb = http.get(&url)?;
    if let Some(e) = etag {
        rb = rb.header(header::IF_NONE_MATCH, e);
    }
    if let Some(l) = last_modified {
        rb = rb.header(header::IF_MODIFIED_SINCE, l);
    }
    let resp = http.send(rb).await?;
    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(IcsFetch::NotModified);
    }
    Http::check_status(resp.status())?;
    let hv = |name| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let (etag, last_modified) = (hv(header::ETAG), hv(header::LAST_MODIFIED));
    let bytes = http.body(resp).await?;
    Ok(IcsFetch::Fetched(Fetched {
        body: String::from_utf8_lossy(&bytes).into_owned(),
        etag,
        last_modified,
    }))
}

fn time_of(p: &Property<'_>, value: &str) -> IcsTime {
    let param = |k: &str| {
        p.params
            .iter()
            .find(|q| q.key.as_ref().eq_ignore_ascii_case(k))
            .and_then(|q| q.val.as_ref())
            .map(|v| v.as_ref().to_owned())
    };
    IcsTime {
        value: value.trim().to_owned(),
        tzid: param("TZID"),
        is_date: param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE")),
    }
}

fn times(c: &Component<'_>, name: &str) -> Vec<IcsTime> {
    c.properties
        .iter()
        .filter(|p| p.name.as_ref().eq_ignore_ascii_case(name))
        .flat_map(|p| {
            p.val
                .as_ref()
                .split(',')
                .filter(|v| !v.trim().is_empty())
                .map(|v| time_of(p, v))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn text(c: &Component<'_>, name: &str) -> Option<String> {
    c.properties
        .iter()
        .find(|p| p.name.as_ref().eq_ignore_ascii_case(name))
        .map(|p| p.val.clone().unescape_text().as_ref().to_owned())
}

fn raw(c: &Component<'_>, name: &str) -> Option<String> {
    c.properties
        .iter()
        .find(|p| p.name.as_ref().eq_ignore_ascii_case(name))
        .map(|p| p.val.as_ref().trim().to_owned())
}

fn first_time(c: &Component<'_>, name: &str) -> Option<IcsTime> {
    times(c, name).into_iter().next()
}

/// Parses VEVENT components of one or more VCALENDAR blobs. Rules and TZIDs are kept raw.
pub fn parse(body: &str) -> Result<Vec<ParsedComponent>> {
    let unfolded = parser::unfold(body);
    let comps = parser::read_components(&unfolded).map_err(ConnectorError::Parse)?;
    let mut out = Vec::new();
    for root in &comps {
        let events: Vec<&Component<'_>> = if root.name.as_ref().eq_ignore_ascii_case("VEVENT") {
            vec![root]
        } else {
            root.components
                .iter()
                .filter(|c| c.name.as_ref().eq_ignore_ascii_case("VEVENT"))
                .collect()
        };
        for c in events {
            out.push(ParsedComponent {
                uid: raw(c, "UID"),
                recurrence_id: first_time(c, "RECURRENCE-ID"),
                dtstart: first_time(c, "DTSTART"),
                dtend: first_time(c, "DTEND"),
                duration: raw(c, "DURATION"),
                rrule: raw(c, "RRULE"),
                exdate: times(c, "EXDATE"),
                rdate: times(c, "RDATE"),
                summary: text(c, "SUMMARY"),
                location: text(c, "LOCATION"),
                description: text(c, "DESCRIPTION"),
                class: raw(c, "CLASS"),
                status: raw(c, "STATUS"),
                url: raw(c, "URL"),
            });
        }
    }
    Ok(out)
}

/// Builds a VCALENDAR with one VEVENT for a CalDAV `PUT`. Timed events are written in UTC;
/// all-day events use the local dates of `ev.tz` (`DTEND` exclusive, at least one day).
pub fn build_vevent(uid: &str, ev: &crate::EventWrite) -> Result<String> {
    use icalendar::{Calendar, Component, Event, EventLike};
    let mut e = Event::new();
    e.uid(uid)
        .summary(&ev.summary)
        .timestamp(chrono::Utc::now());
    if let Some(d) = &ev.description {
        e.description(d);
    }
    if let Some(l) = &ev.location {
        e.location(l);
    }
    if ev.all_day {
        let tz: chrono_tz::Tz = ev
            .tz
            .parse()
            .map_err(|_| ConnectorError::Parse("unknown time zone".into()))?;
        let s = ev.start.with_timezone(&tz).date_naive();
        let end = ev
            .end
            .with_timezone(&tz)
            .date_naive()
            .max(s + chrono::Days::new(1));
        e.starts(s).ends(end);
    } else {
        e.starts(ev.start).ends(ev.end);
    }
    let mut cal = Calendar::new();
    cal.push(e.done());
    Ok(cal.done().to_string())
}
