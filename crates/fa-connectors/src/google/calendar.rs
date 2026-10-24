use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use serde::Deserialize;
use url::Url;

use crate::{ConnectorError, EventTime, Http, RemoteCalendar, RemoteEvent, Window, error::Result};

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

fn api_url(base: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(base).map_err(|e| ConnectorError::Parse(e.to_string()))?;
    url.path_segments_mut()
        .map_err(|_| ConnectorError::Parse("bad base url".into()))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
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
