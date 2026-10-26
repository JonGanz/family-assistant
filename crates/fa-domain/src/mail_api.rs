//! M3 request and response DTOs (plan/M3.md, Domain types and HTTP API v1 additions).
//!
//! Wire formats: instants RFC 3339 (`DateTime<Utc>`), dates `YYYY-MM-DD`, money as
//! `amount_minor` (minor currency units) plus an ISO 4217 `currency`.

use std::fmt;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::entities::*;
use crate::ids::*;
use crate::mail::*;
use crate::tasks_api::TaskView;
use crate::timefmt;

macro_rules! dto {
    ($(#[$m:meta])* $v:vis struct $name:ident { $($body:tt)* }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        $v struct $name { $($body)* }
    };
}

// ---- mail sources ----

dto!(
    pub struct MailSourceView {
        pub id: MailSourceId,
        pub provider: MailProvider,
        pub name: String,
        pub address: String,
        pub owners: Vec<MemberId>,
        pub visibility: MailVisibility,
        pub read_only: bool,
        pub backfill_days: i32,
        #[ts(optional = nullable)]
        pub query: Option<String>,
        pub poll_minutes: i32,
        pub folders: Vec<String>,
        #[ts(optional = nullable)]
        pub archive_folder: Option<String>,
        pub gmail_ext: bool,
        #[ts(optional = nullable)]
        pub imap_host: Option<String>,
        #[ts(optional = nullable)]
        pub imap_port: Option<i32>,
        #[ts(optional = nullable)]
        pub imap_username: Option<String>,
        #[ts(as = "Option<OAuthAccountId>", optional = nullable)]
        pub oauth_account: Option<OAuthAccountId>,
        pub state: MailSourceState,
        /// RFC 3339.
        #[ts(optional = nullable)]
        pub last_synced_at: Option<DateTime<Utc>>,
        #[ts(optional = nullable)]
        pub last_error: Option<String>,
        pub message_count: i64,
        /// The viewer may edit, pause or remove the source.
        pub can_manage: bool,
        pub version: i32,
    }
);

/// IMAP connection settings. Debug output omits the password.
#[derive(Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct ImapSettings {
    pub host: String,
    pub port: i32,
    pub username: String,
    /// Write-only; stored as a secret. Absent on a patch = keep.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub folders: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub archive_folder: Option<String>,
}

impl fmt::Debug for ImapSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImapSettings")
            .field("host", &self.host)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct NewMailSource {
    pub provider: MailProvider,
    pub name: String,
    /// Defaults to the OAuth account's email (Gmail) or the IMAP username.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub address: Option<String>,
    #[serde(default)]
    pub owners: Vec<MemberId>,
    /// Default `owners`.
    #[serde(default)]
    #[ts(as = "Option<MailVisibility>", optional = nullable)]
    pub visibility: Option<MailVisibility>,
    /// Default true.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub read_only: Option<bool>,
    /// 1..=365, default 30.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub backfill_days: Option<i32>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub query: Option<String>,
    /// 1..=1440, default 5.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub poll_minutes: Option<i32>,
    #[serde(default)]
    #[ts(as = "Option<OAuthAccountId>", optional = nullable)]
    pub oauth_account: Option<OAuthAccountId>,
    #[serde(default)]
    #[ts(as = "Option<ImapSettings>", optional = nullable)]
    pub imap: Option<ImapSettings>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MailSourcePatch {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub owners: Option<Vec<MemberId>>,
    #[serde(default)]
    #[ts(as = "Option<MailVisibility>", optional = nullable)]
    pub visibility: Option<MailVisibility>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub read_only: Option<bool>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub backfill_days: Option<i32>,
    /// Absent = keep, `null` = clear.
    #[serde(
        default,
        with = "timefmt::dopt",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(type = "string | null", optional)]
    #[schema(value_type = Option<String>)]
    pub query: Option<Option<String>>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub poll_minutes: Option<i32>,
    #[serde(default)]
    #[ts(as = "Option<ImapSettings>", optional = nullable)]
    pub imap: Option<ImapSettings>,
    /// Only `paused` and `ok` are accepted: pause or resume.
    #[serde(default)]
    #[ts(as = "Option<MailSourceState>", optional = nullable)]
    pub state: Option<MailSourceState>,
}

/// Debug output omits the password.
#[derive(Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct ImapTestRequest {
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
}

impl fmt::Debug for ImapTestRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImapTestRequest")
            .field("host", &self.host)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

dto!(
    pub struct ImapTestResult {
        pub ok: bool,
        pub gmail_ext: bool,
        pub folders: Vec<String>,
        #[ts(optional = nullable)]
        pub error: Option<String>,
    }
);

// ---- messages ----

dto!(
    pub struct MessageSummary {
        pub id: MessageId,
        pub source: MailSourceId,
        pub from_addr: String,
        #[ts(optional = nullable)]
        pub from_name: Option<String>,
        pub subject: String,
        #[ts(optional = nullable)]
        pub snippet: Option<String>,
        /// RFC 3339.
        pub received_at: DateTime<Utc>,
        /// A `category.key`.
        #[ts(optional = nullable)]
        pub category: Option<String>,
        #[ts(optional = nullable)]
        pub time_sensitive: Option<bool>,
        #[ts(optional = nullable)]
        pub is_bill: Option<bool>,
        pub members: Vec<MemberId>,
        pub processing_state: ProcessingState,
        pub has_attachments: bool,
        pub in_eval_sample: bool,
        /// Computed link back to the original message (Gmail only).
        #[ts(optional = nullable)]
        pub external_url: Option<String>,
    }
);

dto!(
    pub struct MessageQuery {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub category: Option<String>,
        #[serde(default)]
        #[ts(as = "Option<MemberId>", optional = nullable)]
        pub member: Option<MemberId>,
        #[serde(default)]
        #[ts(as = "Option<MailSourceId>", optional = nullable)]
        pub source: Option<MailSourceId>,
        #[serde(default)]
        #[ts(as = "Option<ProcessingState>", optional = nullable)]
        pub state: Option<ProcessingState>,
        /// Free-text search over sender and subject.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub q: Option<String>,
        /// Include categories whose behaviour is `ignore` (default false).
        #[serde(default)]
        #[ts(optional = nullable)]
        pub include_ignored: Option<bool>,
        /// Cursor: the `received_at` (RFC 3339) of the last row of the previous page.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub before: Option<DateTime<Utc>>,
        /// At most 100, default 50.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub limit: Option<u32>,
    }
);

dto!(
    pub struct MessagePage {
        pub items: Vec<MessageSummary>,
        /// RFC 3339; pass as `before` for the next page.
        #[ts(optional = nullable)]
        pub next_before: Option<DateTime<Utc>>,
    }
);

dto!(
    pub struct SectionView {
        #[ts(optional = nullable)]
        pub title: Option<String>,
        pub text: String,
        /// Filled when the request carries `?member=`.
        #[ts(as = "Option<Relevance>", optional = nullable)]
        pub relevance: Option<Relevance>,
    }
);

dto!(
    pub struct DecisionView {
        pub decision_point: DecisionPointKey,
        #[ts(type = "unknown")]
        pub value: serde_json::Value,
        pub confidence: f32,
        pub stage: StageKind,
        pub is_final: bool,
        #[ts(optional = nullable)]
        pub source: Option<String>,
        #[ts(as = "Option<MemberId>", optional = nullable)]
        pub decided_by: Option<MemberId>,
        /// RFC 3339.
        pub at: DateTime<Utc>,
    }
);

dto!(
    pub struct DerivedEvent {
        pub id: EventId,
        pub title: String,
        /// RFC 3339.
        pub start_at: DateTime<Utc>,
        pub status: EventStatus,
    }
);

dto!(
    pub struct DerivedTask {
        pub id: TaskId,
        pub title: String,
        pub status: TaskStatus,
    }
);

dto!(
    pub struct MailActionView {
        pub id: MailActionId,
        pub action: MailActionKind,
        #[ts(optional = nullable)]
        pub label: Option<String>,
        pub automatic: bool,
        #[ts(as = "Option<MemberId>", optional = nullable)]
        pub done_by: Option<MemberId>,
        /// RFC 3339.
        pub done_at: DateTime<Utc>,
        #[ts(optional = nullable)]
        pub undone_at: Option<DateTime<Utc>>,
        /// RFC 3339; undo is refused after this instant (30 days, D-058).
        pub undo_until: DateTime<Utc>,
        #[ts(optional = nullable)]
        pub error: Option<String>,
    }
);

dto!(
    pub struct MessageDetail {
        pub summary: MessageSummary,
        #[ts(optional = nullable)]
        pub body_text: Option<String>,
        /// The body was purged by the retention policy.
        pub content_purged: bool,
        pub sections: Vec<SectionView>,
        pub decisions: Vec<DecisionView>,
        pub events: Vec<DerivedEvent>,
        pub tasks: Vec<DerivedTask>,
        pub bills: Vec<BillView>,
        pub actions: Vec<MailActionView>,
        /// The viewer may apply mail actions (writable source, `GmailAction`).
        pub can_act: bool,
    }
);

dto!(
    pub struct LabelInput {
        pub decision_point: DecisionPointKey,
        /// `email.member_link`: an array of member ids.
        #[ts(type = "unknown")]
        pub value: serde_json::Value,
    }
);

dto!(
    pub struct EvalSampleInput {
        pub in_sample: bool,
    }
);

dto!(
    pub struct MailActionInput {
        pub action: MailActionKind,
        /// Required for `label`.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub label: Option<String>,
    }
);

dto!(
    pub struct MakeTaskInput {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub title: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub assignees: Option<Vec<MemberId>>,
        /// RFC 3339.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub due_at: Option<DateTime<Utc>>,
        #[serde(default)]
        #[ts(as = "Option<TaskListId>", optional = nullable)]
        pub list: Option<TaskListId>,
    }
);

dto!(
    pub struct EvalProgress {
        pub in_sample: i64,
        pub target: i64,
        pub labelled: i64,
    }
);

dto!(
    pub struct LabelQueueItem {
        pub message: MessageDetail,
        pub progress: EvalProgress,
    }
);

// ---- orgs, contacts, enrollments ----

dto!(
    /// Per-organisation bill regexes (capture group 1 is the value).
    pub struct BillTemplate {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub amount: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub due: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub account: Option<String>,
    }
);

dto!(
    pub struct OrgContactView {
        pub id: OrgContactId,
        pub org: OrgId,
        pub name: String,
        pub email: String,
        pub role: ContactRole,
        pub version: i32,
    }
);

dto!(
    pub struct OrgView {
        pub id: OrgId,
        pub kind: OrgKind,
        pub name: String,
        pub domains: Vec<String>,
        pub senders: Vec<String>,
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub ics_calendar: Option<CalendarId>,
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub default_calendar: Option<CalendarId>,
        pub contact_mode: ContactMode,
        #[ts(as = "Option<BillTemplate>", optional = nullable)]
        pub bill_template: Option<BillTemplate>,
        pub contacts: Vec<OrgContactView>,
        pub version: i32,
    }
);

dto!(
    pub struct OrgInput {
        pub kind: OrgKind,
        pub name: String,
        #[serde(default)]
        pub domains: Vec<String>,
        #[serde(default)]
        pub senders: Vec<String>,
        #[serde(default)]
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub ics_calendar: Option<CalendarId>,
        #[serde(default)]
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub default_calendar: Option<CalendarId>,
        /// Default `suggest`.
        #[serde(default)]
        #[ts(as = "Option<ContactMode>", optional = nullable)]
        pub contact_mode: Option<ContactMode>,
        #[serde(default)]
        #[ts(as = "Option<BillTemplate>", optional = nullable)]
        pub bill_template: Option<BillTemplate>,
    }
);

dto!(
    pub struct OrgPatch {
        #[serde(default)]
        #[ts(as = "Option<OrgKind>", optional = nullable)]
        pub kind: Option<OrgKind>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub name: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub domains: Option<Vec<String>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub senders: Option<Vec<String>>,
        /// Absent = keep, `null` = clear.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<CalendarId>)]
        pub ics_calendar: Option<Option<CalendarId>>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<CalendarId>)]
        pub default_calendar: Option<Option<CalendarId>>,
        #[serde(default)]
        #[ts(as = "Option<ContactMode>", optional = nullable)]
        pub contact_mode: Option<ContactMode>,
        /// Absent = keep, `null` = clear.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(as = "Option<BillTemplate>", optional = nullable)]
        #[schema(value_type = Option<BillTemplate>)]
        pub bill_template: Option<Option<BillTemplate>>,
    }
);

dto!(
    pub struct OrgContactInput {
        pub name: String,
        pub email: String,
        /// Default `other`.
        #[serde(default)]
        #[ts(as = "Option<ContactRole>", optional = nullable)]
        pub role: Option<ContactRole>,
    }
);

dto!(
    pub struct OrgContactPatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub name: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub email: Option<String>,
        #[serde(default)]
        #[ts(as = "Option<ContactRole>", optional = nullable)]
        pub role: Option<ContactRole>,
    }
);

dto!(
    pub struct EnrollmentView {
        pub id: EnrollmentId,
        pub member: MemberId,
        pub org: OrgId,
        #[ts(optional = nullable)]
        pub grade: Option<String>,
        #[ts(optional = nullable)]
        pub class_name: Option<String>,
        /// `YYYY-MM-DD`.
        #[serde(rename = "from")]
        #[ts(rename = "from", as = "Option<String>", optional = nullable)]
        pub from_date: Option<NaiveDate>,
        /// `YYYY-MM-DD`.
        #[serde(rename = "until")]
        #[ts(rename = "until", as = "Option<String>", optional = nullable)]
        pub until_date: Option<NaiveDate>,
        pub contacts: Vec<OrgContactId>,
        pub version: i32,
    }
);

dto!(
    pub struct EnrollmentInput {
        pub org: OrgId,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub grade: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub class_name: Option<String>,
        /// `YYYY-MM-DD`.
        #[serde(default, rename = "from")]
        #[ts(rename = "from", as = "Option<String>", optional = nullable)]
        pub from_date: Option<NaiveDate>,
        /// `YYYY-MM-DD`.
        #[serde(default, rename = "until")]
        #[ts(rename = "until", as = "Option<String>", optional = nullable)]
        pub until_date: Option<NaiveDate>,
        #[serde(default)]
        pub contacts: Vec<OrgContactId>,
    }
);

dto!(
    pub struct EnrollmentPatch {
        #[serde(default)]
        #[ts(as = "Option<OrgId>", optional = nullable)]
        pub org: Option<OrgId>,
        /// Absent = keep, `null` = clear.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub grade: Option<Option<String>>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub class_name: Option<Option<String>>,
        /// `YYYY-MM-DD`; absent = keep, `null` = clear.
        #[serde(
            default,
            rename = "from",
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(rename = "from", type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub from_date: Option<Option<NaiveDate>>,
        #[serde(
            default,
            rename = "until",
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(rename = "until", type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub until_date: Option<Option<NaiveDate>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub contacts: Option<Vec<OrgContactId>>,
    }
);

// ---- rules, signals, categories ----

dto!(
    pub struct MailRuleView {
        pub id: MailRuleId,
        pub decision_point: DecisionPointKey,
        pub name: String,
        pub priority: i32,
        pub conditions: Vec<RuleCondition>,
        #[ts(type = "unknown")]
        pub value: serde_json::Value,
        pub confidence: f32,
        pub enabled: bool,
        pub version: i32,
    }
);

dto!(
    pub struct MailRuleInput {
        pub decision_point: DecisionPointKey,
        pub name: String,
        /// Lower runs first; default 100.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub priority: Option<i32>,
        pub conditions: Vec<RuleCondition>,
        #[ts(type = "unknown")]
        pub value: serde_json::Value,
        /// 0..=1, default 0.9.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub confidence: Option<f32>,
        /// Default true.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub enabled: Option<bool>,
    }
);

dto!(
    pub struct MailRulePatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub name: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub priority: Option<i32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub conditions: Option<Vec<RuleCondition>>,
        #[serde(default)]
        #[ts(type = "unknown", optional)]
        pub value: Option<serde_json::Value>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub confidence: Option<f32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub enabled: Option<bool>,
    }
);

dto!(
    pub struct RuleTestRequest {
        pub rule: MailRuleInput,
        /// Messages to scan, newest first.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub limit: Option<u32>,
    }
);

dto!(
    pub struct RuleTestMatch {
        pub message: MessageId,
        pub subject: String,
        pub from_addr: String,
        /// RFC 3339.
        pub received_at: DateTime<Utc>,
    }
);

dto!(
    pub struct RuleTestResult {
        pub matched: Vec<RuleTestMatch>,
        pub scanned: i64,
    }
);

dto!(
    /// A built-in signal (Ingest.md §6.2).
    pub struct SignalView {
        pub key: String,
        pub decision_point: DecisionPointKey,
        pub description: String,
        pub confidence: f32,
        pub enabled: bool,
    }
);

dto!(
    pub struct SignalPatch {
        pub enabled: bool,
    }
);

dto!(
    pub struct CategoryView {
        pub id: CategoryId,
        pub key: String,
        pub label: String,
        pub behaviour: CategoryBehaviour,
        pub notify_time_sensitive: bool,
        pub auto_actions: Vec<AutoAction>,
        pub email_tasks: EmailTasksMode,
        #[ts(optional = nullable)]
        pub color: Option<String>,
        pub ord: i32,
        pub archived: bool,
        pub version: i32,
    }
);

dto!(
    pub struct CategoryInput {
        /// `^[a-z][a-z0-9_]{0,31}$`, unique per household.
        pub key: String,
        pub label: String,
        pub behaviour: CategoryBehaviour,
        #[serde(default)]
        pub notify_time_sensitive: bool,
        /// Default `[label]`.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub auto_actions: Option<Vec<AutoAction>>,
        /// Default `suggest`.
        #[serde(default)]
        #[ts(as = "Option<EmailTasksMode>", optional = nullable)]
        pub email_tasks: Option<EmailTasksMode>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub color: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ord: Option<i32>,
    }
);

dto!(
    /// The key is immutable. Archive with `archived: true`.
    pub struct CategoryPatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub label: Option<String>,
        #[serde(default)]
        #[ts(as = "Option<CategoryBehaviour>", optional = nullable)]
        pub behaviour: Option<CategoryBehaviour>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub notify_time_sensitive: Option<bool>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub auto_actions: Option<Vec<AutoAction>>,
        #[serde(default)]
        #[ts(as = "Option<EmailTasksMode>", optional = nullable)]
        pub email_tasks: Option<EmailTasksMode>,
        /// Absent = keep, `null` = clear.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub color: Option<Option<String>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ord: Option<i32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub archived: Option<bool>,
    }
);

// ---- bills ----

dto!(
    pub struct BillView {
        pub id: BillId,
        pub payee: String,
        /// Minor currency units (cents).
        pub amount_minor: i64,
        /// ISO 4217.
        pub currency: String,
        /// `YYYY-MM-DD`.
        #[ts(as = "Option<String>", optional = nullable)]
        pub due_on: Option<NaiveDate>,
        #[ts(optional = nullable)]
        pub account_hint: Option<String>,
        pub status: BillStatus,
        pub confidence: f32,
        #[ts(as = "Option<TaskId>", optional = nullable)]
        pub task: Option<TaskId>,
        /// The pay task is done.
        pub task_done: bool,
        #[ts(as = "Option<MessageId>", optional = nullable)]
        pub message: Option<MessageId>,
        #[ts(optional = nullable)]
        pub external_url: Option<String>,
        pub version: i32,
    }
);

text_enum!(
    /// The statuses a user may set on a bill.
    BillPatchStatus { Open, Dismissed }
);

dto!(
    pub struct BillPatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub payee: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub amount_minor: Option<i64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub currency: Option<String>,
        /// `YYYY-MM-DD`; absent = keep, `null` = clear.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub due_on: Option<Option<NaiveDate>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub account_hint: Option<String>,
        #[serde(default)]
        #[ts(as = "Option<BillPatchStatus>", optional = nullable)]
        pub status: Option<BillPatchStatus>,
    }
);

// ---- inbox ----

dto!(
    pub struct EventProposalView {
        pub id: EventId,
        pub title: String,
        /// RFC 3339.
        pub start_at: DateTime<Utc>,
        /// RFC 3339.
        pub end_at: DateTime<Utc>,
        pub all_day: bool,
        #[ts(optional = nullable)]
        pub location: Option<String>,
        pub confidence: f32,
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub suggested_calendar: Option<CalendarId>,
        #[ts(optional = nullable)]
        pub route_confidence: Option<f32>,
        /// The proposal is to delete an event we wrote (a cancellation arrived).
        pub pending_cancel: bool,
        pub members: Vec<MemberId>,
        #[ts(as = "Option<MessageId>", optional = nullable)]
        pub message: Option<MessageId>,
        /// Sender display text.
        #[ts(optional = nullable)]
        pub from_label: Option<String>,
        #[ts(optional = nullable)]
        pub external_url: Option<String>,
        #[ts(optional = nullable)]
        pub write_error: Option<String>,
        pub version: i32,
    }
);

dto!(
    pub struct ContactSuggestionView {
        pub id: ContactSuggestionId,
        pub org: OrgId,
        pub org_name: String,
        pub email: String,
        #[ts(optional = nullable)]
        pub name: Option<String>,
        #[ts(as = "Option<MemberId>", optional = nullable)]
        pub member: Option<MemberId>,
        #[ts(as = "Option<MessageId>", optional = nullable)]
        pub message: Option<MessageId>,
        /// RFC 3339.
        pub created_at: DateTime<Utc>,
    }
);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum InboxItem {
    Event { event: EventProposalView },
    Bill { bill: BillView },
    Task { task: TaskView },
    Contact { contact: ContactSuggestionView },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct InboxCounts {
    pub event: i64,
    pub bill: i64,
    pub task: i64,
    pub contact: i64,
    pub total: i64,
}

impl InboxCounts {
    pub fn new(event: i64, bill: i64, task: i64, contact: i64) -> Self {
        Self {
            event,
            bill,
            task,
            contact,
            total: event + bill + task + contact,
        }
    }
}

dto!(
    /// Devices without elevation get `items: []` and the counts only.
    pub struct InboxView {
        pub items: Vec<InboxItem>,
        pub counts: InboxCounts,
    }
);

dto!(
    pub struct EventApproval {
        #[serde(default)]
        #[ts(as = "Option<CalendarId>", optional = nullable)]
        pub calendar: Option<CalendarId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub title: Option<String>,
        /// RFC 3339.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub start_at: Option<DateTime<Utc>>,
        /// RFC 3339.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub end_at: Option<DateTime<Utc>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub all_day: Option<bool>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub members: Option<Vec<MemberId>>,
    }
);

dto!(
    pub struct TaskApproval {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub title: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub assignees: Option<Vec<MemberId>>,
        /// RFC 3339.
        #[serde(default)]
        #[ts(optional = nullable)]
        pub due_at: Option<DateTime<Utc>>,
    }
);

dto!(
    pub struct ContactApproval {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub name: Option<String>,
        #[serde(default)]
        #[ts(as = "Option<ContactRole>", optional = nullable)]
        pub role: Option<ContactRole>,
        #[serde(default)]
        #[ts(as = "Option<MemberId>", optional = nullable)]
        pub member: Option<MemberId>,
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inbox_item_is_tagged() {
        let c = ContactSuggestionView {
            id: ContactSuggestionId::new(),
            org: OrgId::new(),
            org_name: "School".into(),
            email: "t@example.edu".into(),
            name: None,
            member: None,
            message: None,
            created_at: Utc::now(),
        };
        let v = serde_json::to_value(InboxItem::Contact { contact: c.clone() }).unwrap();
        assert_eq!(v["kind"], json!("contact"));
        assert_eq!(v["contact"]["email"], json!("t@example.edu"));
        let back: InboxItem = serde_json::from_value(v).unwrap();
        assert_eq!(back, InboxItem::Contact { contact: c });
    }

    #[test]
    fn counts_total() {
        let c = InboxCounts::new(1, 2, 3, 4);
        assert_eq!(c.total, 10);
        assert_eq!(InboxCounts::default().total, 0);
    }

    #[test]
    fn imap_password_not_in_debug() {
        let s = ImapSettings {
            host: "imap.example.com".into(),
            port: 993,
            username: "u".into(),
            password: Some("hunter2".into()),
            folders: None,
            archive_folder: None,
        };
        let n = NewMailSource {
            provider: MailProvider::Imap,
            name: "m".into(),
            address: None,
            owners: vec![],
            visibility: None,
            read_only: None,
            backfill_days: None,
            query: None,
            poll_minutes: None,
            oauth_account: None,
            imap: Some(s.clone()),
        };
        assert!(!format!("{s:?}").contains("hunter2"));
        assert!(!format!("{n:?}").contains("hunter2"));
        let t = ImapTestRequest {
            host: "h".into(),
            port: 1,
            username: "u".into(),
            password: "hunter2".into(),
        };
        assert!(!format!("{t:?}").contains("hunter2"));
    }

    #[test]
    fn patches_distinguish_absent_and_null() {
        let p: OrgPatch = serde_json::from_value(json!({})).unwrap();
        assert_eq!(p.ics_calendar, None);
        let p: OrgPatch = serde_json::from_value(json!({"ics_calendar": null})).unwrap();
        assert_eq!(p.ics_calendar, Some(None));
        let id = CalendarId::new();
        let p: OrgPatch = serde_json::from_value(json!({"default_calendar": id})).unwrap();
        assert_eq!(p.default_calendar, Some(Some(id)));
        let p: EnrollmentPatch = serde_json::from_value(json!({"from": "2026-09-01"})).unwrap();
        assert_eq!(p.from_date, Some(NaiveDate::from_ymd_opt(2026, 9, 1)));
        let p: BillPatch =
            serde_json::from_value(json!({"due_on": null, "status": "open"})).unwrap();
        assert_eq!(p.due_on, Some(None));
        assert_eq!(p.status, Some(BillPatchStatus::Open));
        assert!(serde_json::from_value::<BillPatch>(json!({"status": "paid"})).is_err());
    }

    #[test]
    fn inputs_default() {
        let c: CategoryInput =
            serde_json::from_value(json!({"key": "pets", "label": "Pets", "behaviour": "keep"}))
                .unwrap();
        assert!(!c.notify_time_sensitive && c.auto_actions.is_none());
        let e: EnrollmentInput = serde_json::from_value(json!({"org": OrgId::new()})).unwrap();
        assert!(e.contacts.is_empty());
        let o: OrgInput = serde_json::from_value(json!({"kind": "school", "name": "S"})).unwrap();
        assert!(o.domains.is_empty() && o.contact_mode.is_none());
        let l: LabelInput =
            serde_json::from_value(json!({"decision_point": "email.category", "value": "school"}))
                .unwrap();
        assert_eq!(l.decision_point, DecisionPointKey::EmailCategory);
        let m: MessageQuery = serde_json::from_value(json!({})).unwrap();
        assert_eq!(m.limit, None);
    }
}
