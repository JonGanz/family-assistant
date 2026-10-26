use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::*;
use crate::tasks::NowWeights;

macro_rules! text_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, TS, utoipa::ToSchema)]
        #[serde(rename_all = "snake_case")]
        #[sqlx(type_name = "text", rename_all = "snake_case")]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        pub enum $name { $($var),+ }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {ty}: {value:?}")]
pub struct ParseEnumError {
    pub ty: &'static str,
    pub value: String,
}

/// A `text_enum!` plus `as_str()` and `FromStr` using the DB text values.
macro_rules! db_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident => $text:literal),+ $(,)? }) => {
        text_enum!($(#[$m])* $name { $($var),+ });

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$var),+];

            pub fn as_str(&self) -> &'static str {
                match self { $($name::$var => $text),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = ParseEnumError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok($name::$var),)+
                    _ => Err(ParseEnumError { ty: stringify!($name), value: s.to_string() }),
                }
            }
        }
    };
}

pub(crate) use db_enum;
pub(crate) use text_enum;

text_enum!(Role { Adult, Child });
text_enum!(DeviceKind { Dashboard });
text_enum!(SecretKind { OauthToken, Other });
text_enum!(Units { Metric, Imperial });
text_enum!(SourceKind {
    Google,
    Caldav,
    Ics
});
text_enum!(SourceState {
    Ok,
    NeedsReauth,
    Error
});
text_enum!(
    /// D-036: `Busy` hides title, location and description.
    Visibility { Full, Busy }
);
text_enum!(EventOrigin {
    Synced,
    Extracted,
    Manual
});
text_enum!(EventStatus {
    Proposed,
    Approved,
    Written,
    Rejected,
    Extracted,
    Expired
});
text_enum!(RuleAction { Alert, Task });
text_enum!(Severity {
    Info,
    Warning,
    Severe
});
text_enum!(RuleStatus {
    Active,
    Suggested,
    Disabled
});
text_enum!(SuggestedBy { Default, S2, User });
text_enum!(AdvisorySource { Rule, S2 });

db_enum!(TaskListKind { Chores => "chores", Homework => "homework", General => "general", Email => "email" });
db_enum!(ChildCanAdd { Own => "own", None => "none" });
db_enum!(CalendarDisplay { Hidden => "hidden", UntilDone => "until_done", Always => "always" });
db_enum!(ScheduleKind { AfterDone => "after_done", Rrule => "rrule" });
db_enum!(OnMissed { RollOver => "roll_over", AutoSkip => "auto_skip", PileUp => "pile_up" });
db_enum!(Strategy { Fixed => "fixed", Rotation => "rotation", Pool => "pool" });
db_enum!(AdvanceOn { AnyCompletion => "any_completion", AssigneeCompletion => "assignee_completion" });
db_enum!(PriorityFlag { Normal => "normal", High => "high" });
db_enum!(TaskStatus { Open => "open", Done => "done", Skipped => "skipped", Suggested => "suggested" });
db_enum!(TaskOrigin { Manual => "manual", System => "system", Weather => "weather", Email => "email" });
db_enum!(DayFilter { Daily => "daily", Weekdays => "weekdays", Weekends => "weekends", SchoolDays => "school_days" });
db_enum!(RunStatus { Open => "open", Done => "done", Skipped => "skipped" });
db_enum!(NotifyTrigger {
    Assigned => "assigned",
    Overdue => "overdue",
    Digest => "digest",
    DueSoon => "due_soon",
    RoutineNudge => "routine_nudge",
    SourceReauth => "source_reauth",
    ApprovalNeeded => "approval_needed",
    MailTimeSensitive => "mail_time_sensitive",
});
db_enum!(NotificationStatus { Pending => "pending", Sent => "sent", Dropped => "dropped", Failed => "failed" });
db_enum!(TaskMode { Create => "create", Suggest => "suggest" });

#[allow(clippy::derivable_impls)]
impl Default for CalendarDisplay {
    fn default() -> Self {
        Self::Hidden
    }
}
#[allow(clippy::derivable_impls)]
impl Default for AdvanceOn {
    fn default() -> Self {
        Self::AnyCompletion
    }
}
#[allow(clippy::derivable_impls)]
impl Default for PriorityFlag {
    fn default() -> Self {
        Self::Normal
    }
}
#[allow(clippy::derivable_impls)]
impl Default for TaskMode {
    fn default() -> Self {
        Self::Create
    }
}
#[allow(clippy::derivable_impls)]
impl Default for DayFilter {
    fn default() -> Self {
        Self::Weekdays
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}

// ---- Entities (internal, may carry hashes; never serialised to clients) ----

#[derive(Debug, Clone)]
pub struct Household {
    pub id: HouseholdId,
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub meal_plan_horizon_weeks: i32,
    pub dashboard_completion_override: bool,
    pub units: Units,
    pub calendar_past_days: i32,
    pub calendar_future_days: i32,
    pub weather_retention_days: i32,
    pub private_events_as_busy: bool,
    pub now_list_size: i32,
    pub now_weights: NowWeights,
    pub quiet_from: NaiveTime,
    pub quiet_until: NaiveTime,
    /// ISO 4217.
    pub currency: String,
    pub gender_terms: crate::mail::GenderTerms,
    /// Built-in signal keys switched off.
    pub disabled_signals: Vec<String>,
    pub events: crate::mail::EventsSettings,
    pub inbox: crate::mail::InboxSettings,
    pub retention_mail: crate::mail::MailRetention,
    pub bill_lead_days: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub id: MemberId,
    pub household_id: HouseholdId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    pub color: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    pub interests: Vec<String>,
    pub hobbies: Vec<String>,
    pub responsibilities: Vec<String>,
    /// Extra names the member goes by in mail (nicknames).
    pub aliases: Vec<String>,
    pub gender: Option<crate::mail::Gender>,
    pub relevance_tags: Vec<String>,
    /// Argon2id hash, adults only.
    pub pin_hash: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserAccount {
    pub id: UserAccountId,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub username: String,
    pub password_hash: String,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub household_id: HouseholdId,
    pub kind: DeviceKind,
    pub name: String,
    pub token_hash: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeviceElevation {
    pub device_id: DeviceId,
    pub member_id: MemberId,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: AuditEntryId,
    pub household_id: HouseholdId,
    pub actor_member: Option<MemberId>,
    pub device: Option<DeviceId>,
    pub action: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub at: DateTime<Utc>,
}

/// Provenance of a derived entity (shown as a "from" link in the UI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct SourceRef {
    pub kind: RefKind,
    pub message_id: Option<String>,
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum RefKind {
    Email,
    Ics,
    /// A synced calendar event with a link back to its provider (e.g. Google `htmlLink`).
    Calendar,
    Weather,
    Manual,
    System,
}

// ---- API DTOs (exported to TypeScript) ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct HouseholdDto {
    pub id: HouseholdId,
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub meal_plan_horizon_weeks: i32,
    pub dashboard_completion_override: bool,
}

impl From<&Household> for HouseholdDto {
    fn from(h: &Household) -> Self {
        Self {
            id: h.id,
            name: h.name.clone(),
            tz: h.tz.clone(),
            location: h.location.clone(),
            meal_plan_horizon_weeks: h.meal_plan_horizon_weeks,
            dashboard_completion_override: h.dashboard_completion_override,
        }
    }
}

/// Public fields, visible to children and devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MemberPublicDto {
    pub id: MemberId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    /// `/api/v1/members/{id}/avatar?v=<sha256 prefix>`; set by the API layer.
    pub avatar_url: Option<String>,
    pub color: Option<String>,
}

/// Full member view for adults. Never includes the PIN hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MemberDto {
    pub id: MemberId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    pub avatar_url: Option<String>,
    pub color: Option<String>,
    #[ts(type = "string | null")]
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    pub interests: Vec<String>,
    pub hobbies: Vec<String>,
    pub responsibilities: Vec<String>,
    pub has_pin: bool,
    pub has_account: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    #[ts(as = "Option<crate::mail::Gender>", optional = nullable)]
    #[schema(value_type = Option<crate::mail::Gender>)]
    pub gender: Option<crate::mail::Gender>,
    #[serde(default)]
    pub relevance_tags: Vec<String>,
    /// Optimistic-concurrency version for `If-Match` (0 when not loaded).
    #[serde(default)]
    pub version: i32,
}

impl From<&Member> for MemberPublicDto {
    fn from(m: &Member) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name.clone(),
            role: m.role,
            avatar: m.avatar.clone(),
            avatar_url: None,
            color: m.color.clone(),
        }
    }
}

impl MemberDto {
    pub fn from_member(m: &Member, has_account: bool) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name.clone(),
            role: m.role,
            avatar: m.avatar.clone(),
            avatar_url: None,
            color: m.color.clone(),
            birthdate: m.birthdate,
            grade: m.grade.clone(),
            interests: m.interests.clone(),
            hobbies: m.hobbies.clone(),
            responsibilities: m.responsibilities.clone(),
            has_pin: m.pin_hash.is_some(),
            has_account,
            aliases: m.aliases.clone(),
            gender: m.gender,
            relevance_tags: m.relevance_tags.clone(),
            version: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct UserAccountDto {
    pub id: UserAccountId,
    pub member_id: MemberId,
    pub username: String,
    pub disabled: bool,
}

impl From<&UserAccount> for UserAccountDto {
    fn from(a: &UserAccount) -> Self {
        Self {
            id: a.id,
            member_id: a.member_id,
            username: a.username.clone(),
            disabled: a.disabled_at.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct DeviceDto {
    pub id: DeviceId,
    pub kind: DeviceKind,
    pub name: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl From<&Device> for DeviceDto {
    fn from(d: &Device) -> Self {
        Self {
            id: d.id,
            kind: d.kind,
            name: d.name.clone(),
            last_seen: d.last_seen,
            revoked: d.revoked_at.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct DeviceElevationDto {
    pub device_id: DeviceId,
    pub member_id: MemberId,
    pub expires_at: DateTime<Utc>,
}

impl From<&DeviceElevation> for DeviceElevationDto {
    fn from(e: &DeviceElevation) -> Self {
        Self {
            device_id: e.device_id,
            member_id: e.member_id,
            expires_at: e.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AuditEntryDto {
    pub id: AuditEntryId,
    pub actor_member: Option<MemberId>,
    pub device: Option<DeviceId>,
    pub action: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub at: DateTime<Utc>,
}

impl From<&AuditEntry> for AuditEntryDto {
    fn from(a: &AuditEntry) -> Self {
        Self {
            id: a.id,
            actor_member: a.actor_member,
            device: a.device,
            action: a.action.clone(),
            subject_type: a.subject_type.clone(),
            subject_id: a.subject_id.clone(),
            at: a.at,
        }
    }
}

/// Either member view, as returned by the members API (untagged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum MemberView {
    Full(MemberFull),
    Public(MemberPublic),
}

pub type MemberPublic = MemberPublicDto;
pub type MemberFull = MemberDto;

// ---- M2 entities (tasks, routines, notifications) ----

#[derive(Debug, Clone)]
pub struct TaskList {
    pub id: TaskListId,
    pub household_id: HouseholdId,
    pub name: String,
    pub kind: TaskListKind,
    pub completion_policy: crate::authz::ListCompletionPolicy,
    pub child_can_add: ChildCanAdd,
    pub dashboard_add: bool,
    pub calendar_default: CalendarDisplay,
    /// `None` = end of the local day.
    pub claim_ttl_minutes: Option<i32>,
    pub visible_on_dashboard: bool,
    pub color: Option<String>,
    pub sort: i32,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum Schedule {
    AfterDone { interval_days: i32 },
    Rrule { rrule: String },
}

impl Schedule {
    pub fn kind(&self) -> ScheduleKind {
        match self {
            Self::AfterDone { .. } => ScheduleKind::AfterDone,
            Self::Rrule { .. } => ScheduleKind::Rrule,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub id: TaskDefinitionId,
    pub household_id: HouseholdId,
    pub list_id: TaskListId,
    pub title: String,
    pub notes: Option<String>,
    pub steps_template: Vec<String>,
    pub schedule: Schedule,
    pub start_date: NaiveDate,
    pub due_time: Option<NaiveTime>,
    pub lead_days: i32,
    pub on_missed: OnMissed,
    pub strategy: Strategy,
    pub advance_on: AdvanceOn,
    pub skip_advances: bool,
    pub rotation_cursor: i32,
    pub priority_flag: PriorityFlag,
    /// `(from, until)` as MMDD ints (401 = April 1st); both or neither.
    pub season: Option<(i32, i32)>,
    pub season_condition: Option<String>,
    pub condition_ok: Option<bool>,
    pub condition_checked_at: Option<DateTime<Utc>>,
    pub last_done_at: Option<DateTime<Utc>>,
    pub next_due_at: Option<DateTime<Utc>>,
    pub paused: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub household_id: HouseholdId,
    pub list_id: TaskListId,
    pub definition_id: Option<TaskDefinitionId>,
    pub weather_rule_id: Option<WeatherRuleId>,
    pub title: String,
    pub notes: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub visible_from: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub origin: TaskOrigin,
    pub source_ref: Option<serde_json::Value>,
    pub priority_flag: PriorityFlag,
    pub calendar_display: CalendarDisplay,
    pub created_by: Option<MemberId>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<MemberId>,
    pub completed_via_device: Option<DeviceId>,
    /// `None` on a skipped task = skipped by the system.
    pub skipped_by: Option<MemberId>,
    pub claimed_by: Option<MemberId>,
    pub claimed_until: Option<DateTime<Utc>>,
    /// The definition's `{rotation_cursor, last_done_at, next_due_at}` before the completion.
    pub completion_undo: Option<serde_json::Value>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TaskStep {
    pub id: TaskStepId,
    pub household_id: HouseholdId,
    pub task_id: TaskId,
    pub ord: i32,
    pub text: String,
    pub done_at: Option<DateTime<Utc>>,
    pub done_by: Option<MemberId>,
}

#[derive(Debug, Clone)]
pub struct Routine {
    pub id: RoutineId,
    pub household_id: HouseholdId,
    pub name: String,
    pub starts_at: NaiveTime,
    pub done_by: NaiveTime,
    pub day_filter: DayFilter,
    pub school_calendar_id: Option<CalendarId>,
    pub no_school_match: Vec<String>,
    pub show_on_dashboard: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RoutineStep {
    pub id: RoutineStepId,
    pub household_id: HouseholdId,
    pub routine_id: RoutineId,
    pub ord: i32,
    pub text: String,
    pub target_time: Option<NaiveTime>,
}

#[derive(Debug, Clone)]
pub struct RoutineRun {
    pub id: RoutineRunId,
    pub household_id: HouseholdId,
    pub routine_id: RoutineId,
    pub member_id: MemberId,
    pub run_date: NaiveDate,
    pub status: RunStatus,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct RoutineRunStep {
    pub household_id: HouseholdId,
    pub run_id: RoutineRunId,
    pub step_id: RoutineStepId,
    pub done_at: Option<DateTime<Utc>>,
    pub done_by: Option<MemberId>,
    pub done_via_device: Option<DeviceId>,
}

#[derive(Debug, Clone)]
pub struct MemberAway {
    pub id: MemberAwayId,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub from_date: NaiveDate,
    pub until_date: NaiveDate,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: NotificationId,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub trigger: NotifyTrigger,
    pub subject_type: String,
    pub subject_id: Option<uuid::Uuid>,
    pub title: String,
    pub body: String,
    pub click_path: Option<String>,
    pub priority: i32,
    pub dedupe_key: String,
    pub status: NotificationStatus,
    pub not_before: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}
