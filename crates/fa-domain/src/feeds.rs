//! Entities for the M1 feed tables (arch/Feeds.md §2). Secrets are references only.

use chrono::{DateTime, NaiveDate, Utc};

use crate::entities::*;
use crate::ids::*;

#[derive(Debug, Clone)]
pub struct OAuthAccount {
    pub id: OAuthAccountId,
    pub household_id: HouseholdId,
    /// Always `google` in M1.
    pub provider: String,
    pub owner_member: MemberId,
    pub email: String,
    pub scopes: Vec<String>,
    pub secret_id: Option<SecretId>,
    pub state: SourceState,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CalendarSource {
    pub id: CalendarSourceId,
    pub household_id: HouseholdId,
    pub kind: SourceKind,
    pub name: String,
    /// `None` is a household source.
    pub owner_member: Option<MemberId>,
    pub oauth_account: Option<OAuthAccountId>,
    /// CalDAV base URL.
    pub url: Option<String>,
    pub username: Option<String>,
    /// CalDAV password, or the whole ICS URL.
    pub secret_id: Option<SecretId>,
    pub writable: bool,
    pub poll_minutes: i32,
    pub state: SourceState,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Calendar {
    pub id: CalendarId,
    pub household_id: HouseholdId,
    pub source: CalendarSourceId,
    pub remote_id: String,
    pub name: String,
    pub color: Option<String>,
    pub enabled: bool,
    pub is_default_target: bool,
    pub sync_state: Option<String>,
    pub last_full_sync_at: Option<DateTime<Utc>>,
}

/// One row per occurrence.
#[derive(Debug, Clone)]
pub struct Event {
    pub id: EventId,
    pub household_id: HouseholdId,
    pub calendar: Option<CalendarId>,
    pub remote_uid: Option<String>,
    pub instance_key: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub source_tz: Option<String>,
    pub is_private: bool,
    pub cancelled: bool,
    pub origin: EventOrigin,
    pub status: Option<EventStatus>,
    pub confidence: Option<f32>,
    pub source_ref: Option<serde_json::Value>,
    pub dedupe_key: String,
    pub remote_etag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WeatherSnapshot {
    pub id: uuid::Uuid,
    pub household_id: HouseholdId,
    pub fetched_at: DateTime<Utc>,
    pub provider: String,
    pub current: serde_json::Value,
    pub hourly: serde_json::Value,
    pub daily: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct WeatherRule {
    pub id: WeatherRuleId,
    pub household_id: HouseholdId,
    pub name: String,
    pub condition: String,
    pub action: RuleAction,
    pub severity: Severity,
    pub message: String,
    pub task_definition: Option<serde_json::Value>,
    pub task_mode: TaskMode,
    pub status: RuleStatus,
    pub suggested_by: SuggestedBy,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Advisory {
    pub id: AdvisoryId,
    pub household_id: HouseholdId,
    pub rule: Option<WeatherRuleId>,
    pub text: String,
    pub severity: Severity,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub source: AdvisorySource,
    pub dedupe_key: String,
    pub dismissed_at: Option<DateTime<Utc>>,
    pub dismissed_by: Option<MemberId>,
}

/// Avatar metadata (the bytes are fetched separately).
#[derive(Debug, Clone)]
pub struct MemberAvatar {
    pub member_id: MemberId,
    pub household_id: HouseholdId,
    pub mime: String,
    pub byte_len: i32,
    pub sha256: String,
}
