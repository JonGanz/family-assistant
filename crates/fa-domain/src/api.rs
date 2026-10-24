//! M1 request and response DTOs (plan/M1.md, HTTP API v1 additions).

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::dashboard::DashboardSettings;
use crate::entities::*;
use crate::ids::*;

// ---- household ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HouseholdView {
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub units: Units,
    pub calendar_past_days: i32,
    pub calendar_future_days: i32,
    pub weather_retention_days: i32,
    pub private_events_as_busy: bool,
    /// Read-only: true when the Google client is configured.
    pub google_oauth_enabled: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HouseholdPatch {
    pub name: Option<String>,
    pub tz: Option<String>,
    pub location: Option<Location>,
    pub units: Option<Units>,
    pub calendar_past_days: Option<i32>,
    pub calendar_future_days: Option<i32>,
    pub weather_retention_days: Option<i32>,
    pub private_events_as_busy: Option<bool>,
}

// ---- calendar sources ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct OAuthAccountView {
    pub id: OAuthAccountId,
    pub provider: String,
    pub owner_member: MemberId,
    pub email: String,
    pub scopes: Vec<String>,
    pub state: SourceState,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourceView {
    pub id: CalendarSourceId,
    pub kind: SourceKind,
    pub name: String,
    pub owner: Option<MemberId>,
    /// `https://host/…`; the full URL is a secret and never returned.
    pub masked_url: Option<String>,
    pub username: Option<String>,
    pub state: SourceState,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub poll_minutes: i32,
    pub version: i32,
}

/// New source. Debug output redacts passwords and URLs.
#[derive(Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NewSource {
    Google {
        oauth_account: OAuthAccountId,
        name: String,
        owner: Option<MemberId>,
    },
    Caldav {
        name: String,
        url: String,
        username: String,
        password: String,
        owner: Option<MemberId>,
    },
    Ics {
        name: String,
        url: String,
        owner: Option<MemberId>,
    },
}

impl fmt::Debug for NewSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Google { name, .. } => f.debug_struct("Google").field("name", name).finish(),
            Self::Caldav { name, username, .. } => f
                .debug_struct("Caldav")
                .field("name", name)
                .field("username", username)
                .finish_non_exhaustive(),
            Self::Ics { name, .. } => f
                .debug_struct("Ics")
                .field("name", name)
                .finish_non_exhaustive(),
        }
    }
}

/// Replacement credentials; any subset. Debug output is redacted.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourceCredentials {
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl fmt::Debug for SourceCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceCredentials").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourcePatch {
    pub name: Option<String>,
    pub poll_minutes: Option<i32>,
    pub credentials: Option<SourceCredentials>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CalendarView {
    pub id: CalendarId,
    pub source: CalendarSourceId,
    pub name: String,
    pub color: Option<String>,
    pub enabled: bool,
    pub members: Vec<MemberId>,
    pub version: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CalendarPatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub enabled: Option<bool>,
    pub members: Option<Vec<MemberId>>,
}

/// Source plus its discovered calendars (POST /calendar-sources).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourceCreated {
    pub source: SourceView,
    pub calendars: Vec<CalendarView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RemoteCalendar {
    pub remote_id: String,
    pub name: String,
    pub color: Option<String>,
}

/// POST /calendar-sources/test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourceTestResult {
    pub ok: bool,
    pub calendars: Vec<RemoteCalendar>,
    /// At most 3.
    pub sample_events: Vec<EventView>,
    pub error: Option<String>,
}

// ---- events ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventView {
    pub id: EventId,
    pub calendar: Option<CalendarId>,
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub members: Vec<MemberId>,
    pub color: Option<String>,
    pub visibility: Visibility,
    pub external_url: Option<String>,
}

// ---- weather ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WeatherView {
    pub fetched_at: DateTime<Utc>,
    /// Metric, Open-Meteo field names (arch/Feeds.md §4).
    pub current: serde_json::Value,
    /// 48 entries.
    pub hourly: serde_json::Value,
    /// 7 entries.
    pub daily: serde_json::Value,
    pub sunrise_today: Option<DateTime<Utc>>,
    pub sunset_today: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AdvisoryView {
    pub id: AdvisoryId,
    pub text: String,
    pub severity: Severity,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub source: AdvisorySource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WeatherRuleView {
    pub id: WeatherRuleId,
    pub name: String,
    pub condition: String,
    pub action: RuleAction,
    pub severity: Severity,
    pub message: String,
    pub status: RuleStatus,
    pub suggested_by: SuggestedBy,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NewWeatherRule {
    pub name: String,
    pub condition: String,
    pub action: RuleAction,
    pub severity: Severity,
    pub message: String,
    /// Defaults to `active`.
    pub status: Option<RuleStatus>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WeatherRulePatch {
    pub name: Option<String>,
    pub condition: Option<String>,
    pub action: Option<RuleAction>,
    pub severity: Option<Severity>,
    pub message: Option<String>,
    pub status: Option<RuleStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ValidateRuleRequest {
    pub condition: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RuleError {
    pub position: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RulePreview {
    pub matches: bool,
    pub value: Option<f64>,
    pub when: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ValidateRuleResponse {
    pub ok: bool,
    pub error: Option<RuleError>,
    pub preview: Option<RulePreview>,
}

// ---- devices & dashboard ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DeviceView {
    pub id: DeviceId,
    pub kind: DeviceKind,
    pub name: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub settings: DashboardSettings,
    pub version: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DevicePatch {
    pub name: Option<String>,
    pub settings: Option<DashboardSettings>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardHousehold {
    pub name: String,
    pub tz: String,
    pub units: Units,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardView {
    pub household: DashboardHousehold,
    pub members: Vec<MemberPublicDto>,
    pub events: Vec<EventView>,
    pub weather: Option<WeatherView>,
    pub advisories: Vec<AdvisoryView>,
    pub settings: DashboardSettings,
}

// ---- status ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SourceStatus {
    pub id: CalendarSourceId,
    pub name: String,
    pub kind: SourceKind,
    pub state: SourceState,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct WeatherStatus {
    pub fetched_at: Option<DateTime<Utc>>,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BackupStatus {
    pub last_at: Option<DateTime<Utc>>,
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QueueStatus {
    pub ready: i64,
    pub running: i64,
    pub dead: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StatusView {
    pub sources: Vec<SourceStatus>,
    pub weather: WeatherStatus,
    pub backup: BackupStatus,
    pub queue: QueueStatus,
}

/// Mask a feed URL for display: `https://host/…` (userinfo, path and query dropped).
pub fn mask_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return "…".into();
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    format!("{scheme}://{host}/…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_url_cases() {
        assert_eq!(
            mask_url("https://cal.example.com/feeds/abc123secret/basic.ics?x=1"),
            "https://cal.example.com/…"
        );
        assert_eq!(
            mask_url("https://user:pw@example.com:8443/a"),
            "https://example.com:8443/…"
        );
        assert_eq!(mask_url("https://example.com"), "https://example.com/…");
        assert_eq!(
            mask_url("https://example.com?token=x"),
            "https://example.com/…"
        );
        assert_eq!(mask_url("garbage"), "…");
    }

    #[test]
    fn new_source_round_trips() {
        let acct = OAuthAccountId::new();
        let cases = [
            serde_json::json!({"kind":"google","oauth_account":acct,"name":"G","owner":null}),
            serde_json::json!({"kind":"caldav","name":"C","url":"https://example.com/dav","username":"u","password":"p","owner":null}),
            serde_json::json!({"kind":"ics","name":"I","url":"https://example.com/a.ics","owner":null}),
        ];
        for j in cases {
            let s: NewSource = serde_json::from_value(j.clone()).unwrap();
            assert_eq!(serde_json::to_value(&s).unwrap(), j);
        }
        assert!(matches!(
            serde_json::from_str::<NewSource>(
                r#"{"kind":"ics","name":"I","url":"https://example.com/a.ics"}"#
            ),
            Ok(NewSource::Ics { owner: None, .. })
        ));
        assert!(serde_json::from_str::<NewSource>(r#"{"kind":"bogus","name":"x"}"#).is_err());
    }

    #[test]
    fn debug_redacts_secrets() {
        let s = NewSource::Caldav {
            name: "C".into(),
            url: "https://example.com/secret-token".into(),
            username: "u".into(),
            password: "hunter2".into(),
            owner: None,
        };
        let d = format!("{s:?}");
        assert!(!d.contains("hunter2") && !d.contains("secret-token"));
    }

    #[test]
    fn member_view_untagged() {
        let p = MemberPublicDto {
            id: MemberId::new(),
            display_name: "Sam".into(),
            role: Role::Child,
            avatar: None,
            avatar_url: None,
            color: None,
        };
        let v: MemberView = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        assert_eq!(v, MemberView::Public(p));
    }
}
