use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventTime {
    DateTime(DateTime<Utc>),
    /// All-day; for `end` this is exclusive.
    Date(NaiveDate),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteEvent {
    pub uid: String,
    /// `""` for plain events, else the original start of the instance (UTC ISO, or `YYYY-MM-DD` for all-day).
    pub instance_key: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: EventTime,
    pub end: EventTime,
    pub private: bool,
    pub cancelled: bool,
    pub external_url: Option<String>,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCalendar {
    pub remote_id: String,
    pub name: String,
    pub color: Option<String>,
    pub ctag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenSet {
    pub access: String,
    /// Absent on refresh responses (the existing refresh token stays valid).
    pub refresh: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
    pub id_token_email: Option<String>,
}

/// A time-range window for a full sync.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// A date/date-time value from an ICS property with its raw parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IcsTime {
    /// Raw value, e.g. `20261024T090000`, `20261024T090000Z`, or `20261024`.
    pub value: String,
    pub tzid: Option<String>,
    /// `VALUE=DATE`
    pub is_date: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedComponent {
    pub uid: Option<String>,
    pub recurrence_id: Option<IcsTime>,
    pub dtstart: Option<IcsTime>,
    pub dtend: Option<IcsTime>,
    pub duration: Option<String>,
    pub rrule: Option<String>,
    pub exdate: Vec<IcsTime>,
    pub rdate: Vec<IcsTime>,
    pub summary: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub class: Option<String>,
    pub status: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Forecast {
    pub current: serde_json::Value,
    pub hourly: serde_json::Value,
    pub daily: serde_json::Value,
    pub timezone: Option<String>,
    pub utc_offset_seconds: i32,
}

/// An event to create or patch on a remote calendar.
///
/// `start` and `end` are instants. For `all_day`, the dates are the local dates of `start` and
/// `end` in `tz` (`end` exclusive; forced to at least one day after `start`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventWrite {
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    /// IANA time zone name (household zone).
    pub tz: String,
}

/// Result of a Google Calendar write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Written {
    pub id: String,
    pub ical_uid: Option<String>,
    pub etag: Option<String>,
    pub html_link: Option<String>,
}
