//! M3 mail domain: enums, entities, household settings and pure helpers
//! (plan/M3.md, Domain types; arch/Ingest.md).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::entities::{ParseEnumError, db_enum, text_enum};
use crate::ids::*;

// ---- enums (text values match the migration CHECKs) ----

db_enum!(OrgKind { School => "school", Daycare => "daycare", Club => "club", Biller => "biller", Other => "other" });
db_enum!(ContactRole { Teacher => "teacher", Coach => "coach", Admin => "admin", Office => "office", Other => "other" });
db_enum!(ContactMode { Suggest => "suggest", Auto => "auto", Manual => "manual" });
db_enum!(SuggestionStatus { Proposed => "proposed", Accepted => "accepted", Rejected => "rejected", Expired => "expired" });
db_enum!(MailProvider { Gmail => "gmail", Imap => "imap" });
db_enum!(MailVisibility { Owners => "owners", Household => "household" });
db_enum!(MailSourceState { Ok => "ok", NeedsReauth => "needs_reauth", Error => "error", Paused => "paused" });
db_enum!(ProcessingState { Pending => "pending", Processed => "processed", Expired => "expired" });
db_enum!(CategoryBehaviour { Ignore => "ignore", Keep => "keep", Extract => "extract" });
db_enum!(AutoAction { Label => "label", Archive => "archive", MarkRead => "mark_read" });
db_enum!(EmailTasksMode { Suggest => "suggest", Create => "create", Off => "off" });
db_enum!(BillStatus { Proposed => "proposed", Open => "open", Paid => "paid", Dismissed => "dismissed" });
db_enum!(MailActionKind { Archive => "archive", MarkRead => "mark_read", Label => "label" });
db_enum!(Gender { Female => "female", Male => "male", Other => "other" });
db_enum!(StageKind { Rules => "rules", Human => "human", Knn => "knn", SystemOne => "system_one", Llm => "llm" });
db_enum!(OnUnavailable { Skip => "skip", Wait => "wait", Queue => "queue" });
db_enum!(SubjectType { Message => "message", Event => "event" });
db_enum!(DecidedBy { Rule => "rule", User => "user", Default => "default" });
db_enum!(ContentPolicy { PurgeAfterProcessing => "purge_after_processing", KeepForLearning => "keep_for_learning", KeepAll => "keep_all" });
db_enum!(ApprovalMode { Immediate => "immediate", Batched => "batched", Digest => "digest", Off => "off" });
db_enum!(InboxKind { Event => "event", Bill => "bill", Task => "task", Contact => "contact" });
db_enum!(Relevance { Relevant => "relevant", General => "general", Other => "other" });
db_enum!(RuleField {
    FromAddr => "from_addr", FromDomain => "from_domain", To => "to", Subject => "subject",
    Body => "body", GmailLabel => "gmail_label", Header => "header",
    HasAttachmentType => "has_attachment_type", OrgKind => "org_kind", Org => "org", Source => "source",
});
db_enum!(RuleOp { Is => "is", In => "in", ContainsAny => "contains_any", Matches => "matches", Exists => "exists" });

macro_rules! default_variant {
    ($($ty:ident => $var:ident),+ $(,)?) => {$(
        #[allow(clippy::derivable_impls)]
        impl Default for $ty {
            fn default() -> Self {
                Self::$var
            }
        }
    )+};
}
default_variant!(
    ContactMode => Suggest,
    MailVisibility => Owners,
    ProcessingState => Pending,
    EmailTasksMode => Suggest,
    ContentPolicy => KeepForLearning,
    ApprovalMode => Batched,
    ContactRole => Other,
    OnUnavailable => Skip,
);

/// The five decision points. `as_str` and the wire format are the dotted key.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    sqlx::Type,
    TS,
    utoipa::ToSchema,
)]
#[sqlx(type_name = "text")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum DecisionPointKey {
    #[serde(rename = "email.category")]
    #[sqlx(rename = "email.category")]
    #[ts(rename = "email.category")]
    EmailCategory,
    #[serde(rename = "email.time_sensitive")]
    #[sqlx(rename = "email.time_sensitive")]
    #[ts(rename = "email.time_sensitive")]
    EmailTimeSensitive,
    #[serde(rename = "email.member_link")]
    #[sqlx(rename = "email.member_link")]
    #[ts(rename = "email.member_link")]
    EmailMemberLink,
    #[serde(rename = "email.is_bill")]
    #[sqlx(rename = "email.is_bill")]
    #[ts(rename = "email.is_bill")]
    EmailIsBill,
    #[serde(rename = "event.calendar_route")]
    #[sqlx(rename = "event.calendar_route")]
    #[ts(rename = "event.calendar_route")]
    EventCalendarRoute,
}

impl DecisionPointKey {
    pub const ALL: &'static [DecisionPointKey] = &[
        Self::EmailCategory,
        Self::EmailTimeSensitive,
        Self::EmailMemberLink,
        Self::EmailIsBill,
        Self::EventCalendarRoute,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EmailCategory => "email.category",
            Self::EmailTimeSensitive => "email.time_sensitive",
            Self::EmailMemberLink => "email.member_link",
            Self::EmailIsBill => "email.is_bill",
            Self::EventCalendarRoute => "event.calendar_route",
        }
    }

    /// The seeded default value (plan/M3.md, Seeding).
    pub fn default_value(&self) -> serde_json::Value {
        match self {
            Self::EmailCategory => serde_json::json!("other"),
            Self::EmailTimeSensitive | Self::EmailIsBill => serde_json::json!(false),
            Self::EmailMemberLink => serde_json::json!("owners"),
            Self::EventCalendarRoute => serde_json::json!("propose"),
        }
    }
}

impl std::str::FromStr for DecisionPointKey {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| ParseEnumError {
                ty: "DecisionPointKey",
                value: s.to_string(),
            })
    }
}

// ---- household settings ----

/// Words in mail that point at a gender-specific group ("girls' soccer").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct GenderTerms {
    pub female: Vec<String>,
    pub male: Vec<String>,
}

impl Default for GenderTerms {
    fn default() -> Self {
        let v = |a: [&str; 3]| a.iter().map(|s| s.to_string()).collect();
        Self {
            female: v(["girls", "girls'", "girl's"]),
            male: v(["boys", "boys'", "boy's"]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct EventsSettings {
    /// Write approved-by-rule events without asking (default off, D-057).
    pub auto_write: bool,
    /// Extraction confidence needed for auto write, 0..1.
    pub extract_threshold: f32,
    /// Routing confidence needed for auto write, 0..1.
    pub route_threshold: f32,
}

impl Default for EventsSettings {
    fn default() -> Self {
        Self {
            auto_write: false,
            extract_threshold: 0.9,
            route_threshold: 0.9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct InboxSettings {
    /// 1..=365.
    pub task_expiry_days: i32,
    /// 1..=365.
    pub contact_expiry_days: i32,
}

impl Default for InboxSettings {
    fn default() -> Self {
        Self {
            task_expiry_days: 14,
            contact_expiry_days: 14,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MailRetention {
    pub content_policy: ContentPolicy,
    pub examples_per_category: i32,
    pub max_pending_days: i32,
}

impl Default for MailRetention {
    fn default() -> Self {
        Self {
            content_policy: ContentPolicy::KeepForLearning,
            examples_per_category: 200,
            max_pending_days: 14,
        }
    }
}

pub const DEFAULT_CURRENCY: &str = "USD";
pub const DEFAULT_BILL_LEAD_DAYS: i32 = 3;

// ---- entities ----

#[derive(Debug, Clone)]
pub struct Org {
    pub id: OrgId,
    pub household_id: HouseholdId,
    pub kind: OrgKind,
    pub name: String,
    pub domains: Vec<String>,
    /// Lower-cased addresses.
    pub senders: Vec<String>,
    pub ics_calendar_id: Option<CalendarId>,
    pub default_calendar_id: Option<CalendarId>,
    pub contact_mode: ContactMode,
    pub bill_amount_regex: Option<String>,
    pub bill_due_regex: Option<String>,
    pub bill_account_regex: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrgContact {
    pub id: OrgContactId,
    pub household_id: HouseholdId,
    pub org_id: OrgId,
    pub name: String,
    /// Lower-cased.
    pub email: String,
    pub role: ContactRole,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Enrollment {
    pub id: EnrollmentId,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub org_id: OrgId,
    pub grade: Option<String>,
    pub class_name: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub until_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ContactSuggestion {
    pub id: ContactSuggestionId,
    pub household_id: HouseholdId,
    pub org_id: OrgId,
    /// Lower-cased.
    pub email: String,
    pub name: Option<String>,
    pub member_id: Option<MemberId>,
    pub message_id: Option<MessageId>,
    pub status: SuggestionStatus,
    pub decided_by: Option<MemberId>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A mailbox. Owners live in `mail_source_owner`; the secret holds the IMAP password.
#[derive(Debug, Clone)]
pub struct MailSource {
    pub id: MailSourceId,
    pub household_id: HouseholdId,
    pub provider: MailProvider,
    pub name: String,
    /// Lower-cased.
    pub address: String,
    pub oauth_account_id: Option<OAuthAccountId>,
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub imap_username: Option<String>,
    pub secret_id: Option<SecretId>,
    pub folders: Vec<String>,
    pub archive_folder: Option<String>,
    /// The IMAP server speaks the Gmail extensions (`X-GM-MSGID`, labels).
    pub gmail_ext: bool,
    pub read_only: bool,
    pub visibility: MailVisibility,
    pub backfill_days: i32,
    pub query: Option<String>,
    pub poll_minutes: i32,
    pub history_id: Option<String>,
    pub state: MailSourceState,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One attachment's metadata, the element type of `Message::attachments` (jsonb).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AttachmentMeta {
    pub part: String,
    pub filename: Option<String>,
    pub mime: String,
    pub size: i64,
    /// Calendar text of a `text/calendar` part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ics: Option<String>,
    /// Extracted PDF text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub household_id: HouseholdId,
    pub source_id: MailSourceId,
    pub remote_id: String,
    pub folder: Option<String>,
    pub thread_id: Option<String>,
    pub rfc822_id: Option<String>,
    pub received_at: DateTime<Utc>,
    /// Lower-cased.
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub subject: String,
    pub gmail_labels: Vec<String>,
    pub has_attachments: bool,
    /// A `category.key`.
    pub category: Option<String>,
    pub time_sensitive: Option<bool>,
    pub is_bill: Option<bool>,
    pub processing_state: ProcessingState,
    pub processed_at: Option<DateTime<Utc>>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub snippet: Option<String>,
    pub headers: Option<serde_json::Value>,
    /// `[AttachmentMeta]` as jsonb.
    pub attachments: Option<serde_json::Value>,
    pub is_learning_example: bool,
    pub in_eval_sample: bool,
    pub content_purged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: CategoryId,
    pub household_id: HouseholdId,
    pub key: String,
    pub label: String,
    pub behaviour: CategoryBehaviour,
    pub notify_time_sensitive: bool,
    pub auto_actions: Vec<AutoAction>,
    pub email_tasks: EmailTasksMode,
    pub color: Option<String>,
    pub ord: i32,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// `{field, op, header?, values}`; `header` names the header when `field = header`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct RuleCondition {
    pub field: RuleField,
    pub op: RuleOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub header: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MailRule {
    pub id: MailRuleId,
    pub household_id: HouseholdId,
    pub decision_point: DecisionPointKey,
    pub name: String,
    pub priority: i32,
    pub conditions: Vec<RuleCondition>,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub enabled: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Bill {
    pub id: BillId,
    pub household_id: HouseholdId,
    pub message_id: Option<MessageId>,
    pub org_id: Option<OrgId>,
    pub payee: String,
    pub amount_minor: i64,
    pub currency: String,
    pub due_on: Option<NaiveDate>,
    pub account_hint: Option<String>,
    pub statement_period: Option<String>,
    pub status: BillStatus,
    pub confidence: f32,
    pub task_id: Option<TaskId>,
    pub source_ref: serde_json::Value,
    pub dedupe_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MailAction {
    pub id: MailActionId,
    pub household_id: HouseholdId,
    pub message_id: MessageId,
    pub action: MailActionKind,
    pub label: Option<String>,
    pub automatic: bool,
    pub done_by: Option<MemberId>,
    pub done_at: DateTime<Utc>,
    pub undone_at: Option<DateTime<Utc>>,
    pub undone_by: Option<MemberId>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DecisionPoint {
    pub id: DecisionPointId,
    pub household_id: HouseholdId,
    pub key: DecisionPointKey,
    pub default_value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PipelineVersion {
    pub id: PipelineVersionId,
    pub household_id: HouseholdId,
    pub decision_point_id: DecisionPointId,
    pub version: i32,
    pub active: bool,
    pub name: String,
    pub created_by: Option<MemberId>,
}

#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub id: PipelineStageId,
    pub household_id: HouseholdId,
    pub pipeline_version_id: PipelineVersionId,
    pub ord: i32,
    pub kind: StageKind,
    pub enabled: bool,
    pub accept_threshold: f32,
    pub on_unavailable: OnUnavailable,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub id: DecisionId,
    pub household_id: HouseholdId,
    pub decision_point_id: DecisionPointId,
    pub subject_type: SubjectType,
    pub subject_id: Uuid,
    pub pipeline_version_id: Option<PipelineVersionId>,
    pub stage_kind: StageKind,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub is_final: bool,
    /// Signal key or rule id; `"default"` for the human-stage default.
    pub source: Option<String>,
    pub decided_by: Option<MemberId>,
    pub created_at: DateTime<Utc>,
}

// ---- pure helpers ----

/// Characters left as they are in the `authuser` query value.
fn query_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'@' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Opens a message in Gmail's web UI for the given account.
pub fn gmail_deep_link(address: &str, id_hex: &str) -> String {
    format!(
        "https://mail.google.com/mail/u/?authuser={}#all/{}",
        query_encode(address),
        id_hex
    )
}

/// Link back to the original message. Gmail API ids and Gmail-IMAP hex `X-GM-MSGID`s
/// open in Gmail; other IMAP servers have no stable URL. Computed, never stored.
pub fn message_external_url(
    provider: MailProvider,
    gmail_ext: bool,
    address: &str,
    remote_id: &str,
) -> Option<String> {
    let hex = !remote_id.is_empty() && remote_id.bytes().all(|b| b.is_ascii_hexdigit());
    match provider {
        MailProvider::Gmail => Some(gmail_deep_link(address, remote_id)),
        MailProvider::Imap if gmail_ext && hex => Some(gmail_deep_link(address, remote_id)),
        MailProvider::Imap => None,
    }
}

/// Lower-case, trim, and drop surrounding angle brackets.
pub fn normalise_address(a: &str) -> String {
    a.trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_lowercase()
}

/// The part after the last `@`, if any.
pub fn domain_of(addr: &str) -> Option<&str> {
    let (_, d) = addr.rsplit_once('@')?;
    let d = d.trim().trim_end_matches('>');
    (!d.is_empty()).then_some(d)
}

/// Equal to `org_domain` or a subdomain of it (`mail.example.com` matches `example.com`,
/// `evilexample.com` does not). Case-insensitive; a leading `@` or `.` on the org domain is ignored.
pub fn domain_matches(domain: &str, org_domain: &str) -> bool {
    let d = domain.trim().trim_end_matches('.').to_lowercase();
    let o = org_domain
        .trim()
        .trim_start_matches(['@', '.'])
        .trim_end_matches('.')
        .to_lowercase();
    if o.is_empty() || d.is_empty() {
        return false;
    }
    d == o || d.strip_suffix(&o).is_some_and(|rest| rest.ends_with('.'))
}

/// Stable identity of a bill: normalised payee, amount and due date.
pub fn bill_dedupe_key(payee: &str, amount_minor: i64, due_on: Option<NaiveDate>) -> String {
    let payee: String = payee
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let due = due_on.map_or_else(|| "none".to_string(), |d| d.format("%Y-%m-%d").to_string());
    format!("{payee}|{amount_minor}|{due}")
}

/// `^[a-z][a-z0-9_]{0,31}$`
pub fn valid_category_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some('a'..='z'))
        && key.len() <= 32
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_'))
}

/// A seeded category (D-054).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySeed {
    pub key: &'static str,
    pub label: &'static str,
    pub behaviour: CategoryBehaviour,
    pub notify_time_sensitive: bool,
}

/// The D-054 list, in display order.
pub fn default_categories() -> Vec<CategorySeed> {
    use CategoryBehaviour::*;
    let c = |key, label, behaviour, notify_time_sensitive| CategorySeed {
        key,
        label,
        behaviour,
        notify_time_sensitive,
    };
    vec![
        c("school", "School", Extract, true),
        c("daycare", "Daycare", Extract, true),
        c("activities", "Activities", Extract, true),
        c("appointments", "Appointments", Extract, true),
        c("bills", "Bills", Extract, false),
        c("household", "Household", Extract, false),
        c("personal", "Personal", Keep, false),
        c("newsletters", "Newsletters", Keep, false),
        c("promotions", "Promotions", Ignore, false),
        c("social", "Social", Ignore, false),
        c("junk", "Junk", Ignore, false),
        c("other", "Other", Keep, false),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enums_round_trip() {
        macro_rules! rt {
            ($($t:ident),+) => {$(
                for v in $t::ALL {
                    assert_eq!(v.as_str().parse::<$t>().unwrap(), *v, stringify!($t));
                    assert_eq!(serde_json::to_value(v).unwrap(), json!(v.as_str()), stringify!($t));
                }
                assert!("nope".parse::<$t>().is_err());
            )+};
        }
        rt!(
            OrgKind,
            ContactRole,
            ContactMode,
            SuggestionStatus,
            MailProvider,
            MailVisibility,
            MailSourceState,
            ProcessingState,
            CategoryBehaviour,
            AutoAction,
            EmailTasksMode,
            BillStatus,
            MailActionKind,
            Gender,
            StageKind,
            OnUnavailable,
            SubjectType,
            DecidedBy,
            ContentPolicy,
            ApprovalMode,
            InboxKind,
            Relevance,
            RuleField,
            RuleOp,
            DecisionPointKey
        );
    }

    #[test]
    fn decision_point_keys_are_dotted() {
        assert_eq!(DecisionPointKey::EmailCategory.as_str(), "email.category");
        assert_eq!(
            serde_json::to_value(DecisionPointKey::EventCalendarRoute).unwrap(),
            json!("event.calendar_route")
        );
        assert_eq!(DecisionPointKey::ALL.len(), 5);
        assert_eq!(StageKind::SystemOne.as_str(), "system_one");
        assert_eq!(AutoAction::MarkRead.as_str(), "mark_read");
        assert_eq!(MailSourceState::NeedsReauth.as_str(), "needs_reauth");
    }

    #[test]
    fn new_status_values() {
        use crate::entities::*;
        assert_eq!(
            serde_json::to_value(EventStatus::Extracted).unwrap(),
            json!("extracted")
        );
        assert_eq!(
            serde_json::to_value(EventStatus::Expired).unwrap(),
            json!("expired")
        );
        assert_eq!(TaskListKind::Email.as_str(), "email");
        assert_eq!(TaskOrigin::Email.as_str(), "email");
        assert_eq!(NotifyTrigger::ApprovalNeeded.as_str(), "approval_needed");
        assert_eq!(
            "mail_time_sensitive".parse::<NotifyTrigger>().unwrap(),
            NotifyTrigger::MailTimeSensitive
        );
    }

    #[test]
    fn deep_links() {
        assert_eq!(
            gmail_deep_link("a@example.com", "18f3c0de"),
            "https://mail.google.com/mail/u/?authuser=a@example.com#all/18f3c0de"
        );
        assert!(gmail_deep_link("a+b@example.com", "1").contains("authuser=a%2Bb@example.com"));
        let g = |p, ext, id| message_external_url(p, ext, "a@example.com", id);
        assert!(g(MailProvider::Gmail, false, "18f3c0de").is_some());
        assert!(g(MailProvider::Imap, true, "18f3c0de").is_some());
        assert_eq!(g(MailProvider::Imap, false, "18f3c0de"), None);
        assert_eq!(g(MailProvider::Imap, true, "not-hex"), None);
        assert_eq!(g(MailProvider::Imap, true, ""), None);
    }

    #[test]
    fn addresses() {
        assert_eq!(
            normalise_address("  Teacher@Example.EDU "),
            "teacher@example.edu"
        );
        assert_eq!(normalise_address("<A@B.com>"), "a@b.com");
        assert_eq!(domain_of("a@mail.example.com"), Some("mail.example.com"));
        assert_eq!(domain_of("nodomain"), None);
        assert_eq!(domain_of("a@"), None);
    }

    #[test]
    fn domains() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(domain_matches("Mail.Example.com", "example.com"));
        assert!(domain_matches("a.b.example.com", "@example.com"));
        assert!(!domain_matches("evilexample.com", "example.com"));
        assert!(!domain_matches("example.com.evil.org", "example.com"));
        assert!(!domain_matches("example.com", "mail.example.com"));
        assert!(!domain_matches("example.com", ""));
    }

    #[test]
    fn bill_keys() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 3);
        assert_eq!(
            bill_dedupe_key("  Example  Power, Co.", 12_345, d),
            "example power co|12345|2026-11-03"
        );
        assert_eq!(
            bill_dedupe_key("example power co", 12_345, d),
            bill_dedupe_key("EXAMPLE POWER CO", 12_345, d)
        );
        assert_eq!(bill_dedupe_key("x", 1, None), "x|1|none");
        assert_ne!(bill_dedupe_key("x", 1, None), bill_dedupe_key("x", 2, None));
    }

    #[test]
    fn category_defaults_and_keys() {
        use CategoryBehaviour::*;
        let got: Vec<_> = default_categories()
            .iter()
            .map(|c| (c.key, c.behaviour, c.notify_time_sensitive))
            .collect();
        assert_eq!(
            got,
            vec![
                ("school", Extract, true),
                ("daycare", Extract, true),
                ("activities", Extract, true),
                ("appointments", Extract, true),
                ("bills", Extract, false),
                ("household", Extract, false),
                ("personal", Keep, false),
                ("newsletters", Keep, false),
                ("promotions", Ignore, false),
                ("social", Ignore, false),
                ("junk", Ignore, false),
                ("other", Keep, false),
            ]
        );
        assert!(
            default_categories()
                .iter()
                .all(|c| valid_category_key(c.key))
        );
        assert!(valid_category_key("a"));
        assert!(valid_category_key(&format!("a{}", "b".repeat(31))));
        assert!(!valid_category_key(&format!("a{}", "b".repeat(32))));
        assert!(!valid_category_key(""));
        assert!(!valid_category_key("1abc"));
        assert!(!valid_category_key("Abc"));
        assert!(!valid_category_key("a-b"));
        assert!(!valid_category_key("é"));
    }

    #[test]
    fn settings_defaults_match_migration() {
        assert_eq!(GenderTerms::default().female, ["girls", "girls'", "girl's"]);
        assert_eq!(InboxSettings::default().task_expiry_days, 14);
        assert_eq!(
            MailRetention::default().content_policy,
            ContentPolicy::KeepForLearning
        );
        let e = EventsSettings::default();
        assert!(!e.auto_write && e.extract_threshold == 0.9 && e.route_threshold == 0.9);
    }

    #[test]
    fn rule_condition_wire() {
        let c: RuleCondition = serde_json::from_value(
            json!({"field":"from_domain","op":"in","values":["example.edu"]}),
        )
        .unwrap();
        assert_eq!(c.field, RuleField::FromDomain);
        assert_eq!(c.op, RuleOp::In);
        assert_eq!(c.header, None);
    }
}
