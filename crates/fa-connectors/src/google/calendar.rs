use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::Deserialize;

use super::{api_url, ok_body, parse_json};
use crate::{
    ConnectorError, EventTime, EventWrite, Http, RemoteCalendar, RemoteEvent, Window, Written,
    error::Result,
};

#[derive(Debug, Clone)]
pub enum EventQuery {
    /// Full sync bounded by a window.
    Window(Window),
    /// Incremental sync.
    SyncToken(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventPage {
    pub items: Vec<RemoteEvent>,
    pub next_sync_token: Option<String>,
}

#[derive(Deserialize)]
struct CalendarList {
    #[serde(default)]
    items: Vec<CalendarListItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct CalendarListItem {
    id: String,
    summary: Option<String>,
    #[serde(rename = "summaryOverride")]
    summary_override: Option<String>,
    #[serde(rename = "backgroundColor")]
    background_color: Option<String>,
    #[serde(default)]
    deleted: bool,
}

#[derive(Deserialize)]
struct EventList {
    #[serde(default)]
    items: Vec<GEvent>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "nextSyncToken")]
    next_sync_token: Option<String>,
}

#[derive(Deserialize)]
struct GTime {
    date: Option<NaiveDate>,
    #[serde(rename = "dateTime")]
    date_time: Option<DateTime<chrono::FixedOffset>>,
}

#[derive(Deserialize)]
struct GEvent {
    id: String,
    status: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    visibility: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    etag: Option<String>,
    #[serde(rename = "recurringEventId")]
    recurring_event_id: Option<String>,
    #[serde(rename = "originalStartTime")]
    original_start_time: Option<GTime>,
    start: Option<GTime>,
    end: Option<GTime>,
}

fn to_event_time(t: &GTime) -> Option<EventTime> {
    match (&t.date_time, &t.date) {
        (Some(dt), _) => Some(EventTime::DateTime(dt.with_timezone(&Utc))),
        (None, Some(d)) => Some(EventTime::Date(*d)),
        _ => None,
    }
}

fn instance_key(t: &GTime) -> Option<String> {
    match to_event_time(t)? {
        EventTime::DateTime(dt) => Some(dt.to_rfc3339_opts(SecondsFormat::Secs, true)),
        EventTime::Date(d) => Some(d.to_string()),
    }
}

impl GEvent {
    fn into_remote(self) -> RemoteEvent {
        let cancelled = self.status.as_deref() == Some("cancelled");
        // Cancelled stubs in incremental syncs carry no start/end.
        let epoch = EventTime::DateTime(DateTime::UNIX_EPOCH);
        let start = self.start.as_ref().and_then(to_event_time);
        let end = self.end.as_ref().and_then(to_event_time);
        let (start, end) = match (start, end) {
            (Some(s), Some(e)) => (s, e),
            (Some(s), None) => (s, s),
            (None, Some(e)) => (e, e),
            (None, None) => (epoch, epoch),
        };
        let (uid, key) = match &self.recurring_event_id {
            Some(base) => (
                base.clone(),
                self.original_start_time
                    .as_ref()
                    .and_then(instance_key)
                    .unwrap_or_else(|| self.id.clone()),
            ),
            None => (self.id.clone(), String::new()),
        };
        RemoteEvent {
            uid,
            instance_key: key,
            title: self.summary.unwrap_or_default(),
            description: self.description,
            location: self.location,
            start,
            end,
            private: matches!(self.visibility.as_deref(), Some("private" | "confidential")),
            cancelled,
            external_url: self.html_link,
            etag: self.etag,
        }
    }
}

pub async fn list_calendars(http: &Http, base: &str, access: &str) -> Result<Vec<RemoteCalendar>> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = api_url(base, &["calendar", "v3", "users", "me", "calendarList"])?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("maxResults", "250");
            if let Some(p) = &page {
                q.append_pair("pageToken", p);
            }
        }
        let json = http.get_json(http.get(&url)?.bearer_auth(access)).await?;
        let list: CalendarList =
            serde_json::from_value(json).map_err(|e| ConnectorError::Parse(e.to_string()))?;
        out.extend(list.items.into_iter().filter(|i| !i.deleted).map(|i| {
            RemoteCalendar {
                name: i
                    .summary_override
                    .or(i.summary)
                    .unwrap_or_else(|| i.id.clone()),
                remote_id: i.id,
                color: i.background_color,
                ctag: None,
            }
        }));
        match list.next_page_token {
            Some(p) => page = Some(p),
            None => return Ok(out),
        }
    }
}

pub async fn list_events(
    http: &Http,
    base: &str,
    access: &str,
    cal_id: &str,
    query: &EventQuery,
) -> Result<EventPage> {
    let mut items = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = api_url(base, &["calendar", "v3", "calendars", cal_id, "events"])?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("singleEvents", "true");
            q.append_pair("maxResults", "2500");
            match query {
                EventQuery::Window(w) => {
                    q.append_pair(
                        "timeMin",
                        &w.from.to_rfc3339_opts(SecondsFormat::Secs, true),
                    );
                    q.append_pair("timeMax", &w.to.to_rfc3339_opts(SecondsFormat::Secs, true));
                }
                EventQuery::SyncToken(t) => {
                    q.append_pair("syncToken", t);
                }
            }
            if let Some(p) = &page {
                q.append_pair("pageToken", p);
            }
        }
        let json = http.get_json(http.get(&url)?.bearer_auth(access)).await?;
        let list: EventList =
            serde_json::from_value(json).map_err(|e| ConnectorError::Parse(e.to_string()))?;
        items.extend(list.items.into_iter().map(GEvent::into_remote));
        match list.next_page_token {
            Some(p) => page = Some(p),
            None => {
                return Ok(EventPage {
                    items,
                    next_sync_token: list.next_sync_token,
                });
            }
        }
    }
}

#[derive(Deserialize)]
struct WrittenResp {
    id: String,
    #[serde(rename = "iCalUID")]
    ical_uid: Option<String>,
    etag: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
}

fn local_date(t: DateTime<Utc>, tz: &str) -> Result<NaiveDate> {
    let tz: chrono_tz::Tz = tz
        .parse()
        .map_err(|_| ConnectorError::Parse("unknown time zone".into()))?;
    Ok(t.with_timezone(&tz).date_naive())
}

fn event_body(ev: &EventWrite) -> Result<serde_json::Value> {
    use serde_json::{Map, Value, json};
    let (start, end) = if ev.all_day {
        let s = local_date(ev.start, &ev.tz)?;
        let e = local_date(ev.end, &ev.tz)?.max(s + chrono::Days::new(1));
        (
            json!({ "date": s.to_string() }),
            json!({ "date": e.to_string() }),
        )
    } else {
        let f = |t: DateTime<Utc>| json!({ "dateTime": t.to_rfc3339_opts(SecondsFormat::Secs, true), "timeZone": ev.tz });
        (f(ev.start), f(ev.end))
    };
    let mut m = Map::new();
    m.insert("summary".into(), Value::String(ev.summary.clone()));
    if let Some(d) = &ev.description {
        m.insert("description".into(), Value::String(d.clone()));
    }
    if let Some(l) = &ev.location {
        m.insert("location".into(), Value::String(l.clone()));
    }
    m.insert("start".into(), start);
    m.insert("end".into(), end);
    Ok(Value::Object(m))
}

fn written(bytes: &[u8]) -> Result<Written> {
    let w: WrittenResp = parse_json(bytes)?;
    Ok(Written {
        id: w.id,
        ical_uid: w.ical_uid,
        etag: w.etag,
        html_link: w.html_link,
    })
}

/// `events.insert` (scope `calendar.events`). The returned `iCalUID` is the remote UID.
pub async fn insert_event(
    http: &Http,
    base: &str,
    access: &str,
    cal_id: &str,
    ev: &EventWrite,
) -> Result<Written> {
    let url = api_url(base, &["calendar", "v3", "calendars", cal_id, "events"])?;
    let rb = http
        .request(reqwest::Method::POST, &url)?
        .bearer_auth(access)
        .json(&event_body(ev)?);
    let resp = http.send(rb).await?;
    written(&ok_body(http, resp).await?)
}

/// `events.patch`: only the fields of `ev` are sent.
pub async fn patch_event(
    http: &Http,
    base: &str,
    access: &str,
    cal_id: &str,
    event_id: &str,
    ev: &EventWrite,
) -> Result<Written> {
    let url = api_url(
        base,
        &["calendar", "v3", "calendars", cal_id, "events", event_id],
    )?;
    let rb = http
        .request(reqwest::Method::PATCH, &url)?
        .bearer_auth(access)
        .json(&event_body(ev)?);
    let resp = http.send(rb).await?;
    written(&ok_body(http, resp).await?)
}

/// `events.delete`; an already-deleted event (404 or 410) counts as done.
pub async fn delete_event(
    http: &Http,
    base: &str,
    access: &str,
    cal_id: &str,
    event_id: &str,
) -> Result<()> {
    let url = api_url(
        base,
        &["calendar", "v3", "calendars", cal_id, "events", event_id],
    )?;
    let rb = http
        .request(reqwest::Method::DELETE, &url)?
        .bearer_auth(access);
    let resp = http.send(rb).await?;
    if matches!(resp.status().as_u16(), 404 | 410) {
        return Ok(());
    }
    ok_body(http, resp).await.map(|_| ())
}
