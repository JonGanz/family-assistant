//! M3 household TOML sections: orgs, enrollments, mail sources, categories and mail rules, plus
//! the mail-related household settings. Secrets never appear here: a mail source imported from a
//! file starts as `needs_reauth`, and the export carries no passwords, tokens or message data.

use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, Utc};
use fa_domain::{
    AutoAction, Category, CategoryBehaviour, CategoryId, ContactMode, ContactRole, ContentPolicy,
    DecisionPointKey, EmailTasksMode, Enrollment, EnrollmentId, HouseholdId, MailProvider,
    MailRule, MailRuleId, MailSource, MailSourceId, MailSourceState, MailVisibility, MemberId, Org,
    OrgContact, OrgContactId, OrgId, OrgKind, RuleCondition, valid_category_key,
};
use fa_store::{
    CalendarRepo, CalendarSourceRepo, CategoryRepo, EnrollmentRepo, MailRuleRepo, MailSourceRepo,
    OrgContactRepo, OrgRepo, mail_defaults,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::household::{HouseholdConfig, HouseholdTomlError, ImportError, ImportReport};

fn is_false(b: &bool) -> bool {
    !*b
}

fn default_true() -> bool {
    true
}

// ---- household-level settings ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenderTermsConfig {
    #[serde(default)]
    pub female: Vec<String>,
    #[serde(default)]
    pub male: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_write: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract_threshold: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_threshold: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_expiry_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_expiry_days: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mail: Option<MailRetentionConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailRetentionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_policy: Option<ContentPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples_per_category: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pending_days: Option<i32>,
}

// ---- members ----

/// A child's (or adult's) place at an org. `org` is the org name; `contacts` are emails of that
/// org's contacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentConfig {
    pub org: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
    #[serde(default, rename = "class", skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<NaiveDate>,
    /// `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<String>,
}

// ---- orgs ----

/// Regexes for the biller extractor (each needs one capture group, see Ingest §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BillTemplateConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

impl BillTemplateConfig {
    fn is_empty(&self) -> bool {
        self.amount.is_none() && self.due.is_none() && self.account.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgContactConfig {
    pub name: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<ContactRole>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgConfig {
    pub name: String,
    pub kind: OrgKind,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub senders: Vec<String>,
    /// `"<calendar source key>/<calendar remote_id>"` of a calendar source in this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ics_calendar: Option<String>,
    /// Same format as `ics_calendar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_calendar: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_mode: Option<ContactMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bill_template: Option<BillTemplateConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contacts: Vec<OrgContactConfig>,
}

// ---- mail sources ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImapSection {
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub username: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_folder: Option<String>,
}

/// A mailbox. Never carries a password or token: Gmail sources need an OAuth connection made in
/// the UI (so only existing ones are updated, new ones are skipped), IMAP sources are created
/// as `needs_reauth` until a password is entered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailSourceConfig {
    pub name: String,
    pub provider: MailProvider,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Member keys.
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<MailVisibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backfill_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imap: Option<ImapSection>,
}

// ---- categories and rules ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryConfig {
    pub key: String,
    pub label: String,
    /// Default `keep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behaviour: Option<CategoryBehaviour>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify_time_sensitive: Option<bool>,
    /// Default `["label"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_actions: Option<Vec<AutoAction>>,
    /// Default `suggest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_tasks: Option<EmailTasksMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub archived: bool,
}

/// A user mail rule. For `email.member_link` the value lists member keys, for
/// `event.calendar_route` it is a `"source/calendar"` reference; both are translated to ids on
/// import and back on export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailRuleConfig {
    pub name: String,
    pub decision_point: DecisionPointKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub conditions: Vec<RuleCondition>,
    pub value: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---- validation ----

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

fn plausible_email(s: &str) -> bool {
    let s = s.trim();
    s.len() > 2 && !s.contains(char::is_whitespace) && s.matches('@').count() == 1 && {
        let (l, d) = s.split_once('@').unwrap_or_default();
        !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.')
    }
}

fn plausible_domain(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && !s.contains(['@', '/', ' ']) && s.contains('.')
}

fn regex_error(re: &str) -> Option<String> {
    let cond = RuleCondition {
        field: fa_domain::RuleField::Body,
        op: fa_domain::RuleOp::Matches,
        header: None,
        values: vec![re.to_string()],
    };
    fa_rules::validate_rule(&[cond])
        .err()
        .map(|e| e.to_string())
}

fn split_cal_ref(r: &str) -> Option<(&str, &str)> {
    r.split_once('/')
        .filter(|(k, c)| !k.is_empty() && !c.is_empty())
}

pub(crate) fn validate_mail(cfg: &HouseholdConfig, errs: &mut Vec<String>) {
    let h = &cfg.household;
    if let Some(c) = &h.currency
        && !(c.len() == 3 && c.chars().all(|c| c.is_ascii_uppercase()))
    {
        errs.push(format!(
            "household.currency '{c}' must be a 3-letter code such as USD"
        ));
    }
    if let Some(n) = h.bill_lead_days
        && !(0..=60).contains(&n)
    {
        errs.push("household.bill_lead_days must be 0-60".to_string());
    }
    if let Some(g) = &h.gender_terms {
        for t in g.female.iter().chain(&g.male) {
            if t.trim().is_empty() {
                errs.push("household.gender_terms must not contain empty terms".to_string());
                break;
            }
        }
    }
    if let Some(sig) = &h.disabled_signals {
        let known: HashSet<&str> = fa_rules::signals().iter().map(|s| s.key).collect();
        for k in sig {
            if !known.contains(k.as_str()) {
                errs.push(format!("household.disabled_signals: unknown signal '{k}'"));
            }
        }
    }
    if let Some(e) = &h.events {
        for (n, v) in [
            ("extract_threshold", e.extract_threshold),
            ("route_threshold", e.route_threshold),
        ] {
            if let Some(v) = v
                && !(0.0..=1.0).contains(&v)
            {
                errs.push(format!("household.events.{n} must be 0-1"));
            }
        }
    }
    if let Some(i) = &h.inbox {
        for (n, v) in [
            ("task_expiry_days", i.task_expiry_days),
            ("contact_expiry_days", i.contact_expiry_days),
        ] {
            if let Some(v) = v
                && !(1..=365).contains(&v)
            {
                errs.push(format!("household.inbox.{n} must be 1-365"));
            }
        }
    }
    if let Some(m) = cfg.retention.as_ref().and_then(|r| r.mail.as_ref()) {
        if m.examples_per_category.is_some_and(|v| v < 0) {
            errs.push("retention.mail.examples_per_category must be >= 0".to_string());
        }
        if m.max_pending_days.is_some_and(|v| v < 1) {
            errs.push("retention.mail.max_pending_days must be >= 1".to_string());
        }
    }

    let member_keys: HashSet<&str> = cfg.members.iter().map(|m| m.key.as_str()).collect();
    let mut cal_refs: HashSet<String> = HashSet::new();
    for s in &cfg.calendar_sources {
        for c in &s.calendars {
            cal_refs.insert(format!("{}/{}", s.key, c.remote_id));
        }
    }

    // orgs
    let mut org_names: HashMap<String, usize> = HashMap::new();
    for (i, o) in cfg.orgs.iter().enumerate() {
        if o.name.trim().is_empty() {
            errs.push(format!("org #{}: name must not be empty", i + 1));
            continue;
        }
        if org_names.insert(lc(&o.name), i).is_some() {
            errs.push(format!("duplicate org name '{}'", o.name));
        }
        for d in &o.domains {
            if !plausible_domain(d) {
                errs.push(format!("org '{}': '{d}' is not a domain name", o.name));
            }
        }
        for s in &o.senders {
            if !plausible_email(s) {
                errs.push(format!("org '{}': '{s}' is not an email address", o.name));
            }
        }
        for (field, r) in [
            ("ics_calendar", &o.ics_calendar),
            ("default_calendar", &o.default_calendar),
        ] {
            if let Some(r) = r
                && (split_cal_ref(r).is_none() || !cal_refs.contains(r))
            {
                errs.push(format!(
                    "org '{}': {field} '{r}' must be \"<calendar source key>/<calendar remote_id>\" of a calendar listed in this file",
                    o.name
                ));
            }
        }
        if let Some(t) = &o.bill_template {
            for (field, v) in [
                ("amount", &t.amount),
                ("due", &t.due),
                ("account", &t.account),
            ] {
                if let Some(v) = v
                    && let Some(e) = regex_error(v)
                {
                    errs.push(format!("org '{}': bill_template.{field}: {e}", o.name));
                }
            }
        }
        let mut emails = HashSet::new();
        for c in &o.contacts {
            if c.name.trim().is_empty() {
                errs.push(format!("org '{}': a contact has an empty name", o.name));
            }
            if !plausible_email(&c.email) {
                errs.push(format!(
                    "org '{}': contact email '{}' is not an email address",
                    o.name, c.email
                ));
            } else if !emails.insert(lc(&c.email)) {
                errs.push(format!(
                    "org '{}': duplicate contact email '{}'",
                    o.name, c.email
                ));
            }
        }
    }

    // member extras
    for m in &cfg.members {
        if m.aliases.iter().any(|a| a.trim().is_empty())
            || m.relevance_tags.iter().any(|a| a.trim().is_empty())
        {
            errs.push(format!(
                "member '{}': aliases and relevance_tags must not be empty strings",
                m.key
            ));
        }
        for e in &m.enrollments {
            if e.org.trim().is_empty() {
                errs.push(format!("member '{}': an enrollment has no org", m.key));
                continue;
            }
            if let (Some(f), Some(u)) = (e.from, e.until)
                && f > u
            {
                errs.push(format!(
                    "member '{}', enrollment '{}': from is after until",
                    m.key, e.org
                ));
            }
            if let Some(&i) = org_names.get(&lc(&e.org)) {
                let have: HashSet<String> =
                    cfg.orgs[i].contacts.iter().map(|c| lc(&c.email)).collect();
                for c in &e.contacts {
                    if !have.contains(&lc(c)) {
                        errs.push(format!(
                            "member '{}', enrollment '{}': contact '{c}' is not a contact of that org in this file",
                            m.key, e.org
                        ));
                    }
                }
            }
        }
    }

    // mail sources
    let mut names = HashSet::new();
    for s in &cfg.mail_sources {
        if s.name.trim().is_empty() {
            errs.push("mail source: name must not be empty".to_string());
            continue;
        }
        if !names.insert(lc(&s.name)) {
            errs.push(format!("duplicate mail source name '{}'", s.name));
        }
        for o in &s.owners {
            if !member_keys.contains(o.as_str()) {
                errs.push(format!(
                    "mail source '{}': unknown member key '{o}' in owners",
                    s.name
                ));
            }
        }
        if let Some(a) = &s.address
            && !plausible_email(a)
        {
            errs.push(format!(
                "mail source '{}': address is not an email address",
                s.name
            ));
        }
        if s.backfill_days.is_some_and(|v| !(1..=365).contains(&v)) {
            errs.push(format!(
                "mail source '{}': backfill_days must be 1-365",
                s.name
            ));
        }
        if s.poll_minutes.is_some_and(|v| !(1..=1440).contains(&v)) {
            errs.push(format!(
                "mail source '{}': poll_minutes must be 1-1440",
                s.name
            ));
        }
        match (s.provider, &s.imap) {
            (MailProvider::Imap, None) => errs.push(format!(
                "mail source '{}': provider imap needs an [mail_sources.imap] section",
                s.name
            )),
            (MailProvider::Imap, Some(i)) => {
                if i.host.trim().is_empty() || i.username.trim().is_empty() {
                    errs.push(format!(
                        "mail source '{}': imap host and username must not be empty",
                        s.name
                    ));
                }
                if i.port == Some(0) {
                    errs.push(format!(
                        "mail source '{}': imap port must be 1-65535",
                        s.name
                    ));
                }
            }
            (MailProvider::Gmail, Some(_)) => errs.push(format!(
                "mail source '{}': a gmail source has no imap section",
                s.name
            )),
            _ => {}
        }
    }

    // categories
    let mut keys = HashSet::new();
    for c in &cfg.categories {
        if !valid_category_key(&c.key) {
            errs.push(format!(
                "category key '{}' must be a-z, 0-9 or '_', start with a letter, max 32",
                c.key
            ));
        }
        if !keys.insert(c.key.as_str()) {
            errs.push(format!("duplicate category key '{}'", c.key));
        }
        if c.label.trim().is_empty() {
            errs.push(format!("category '{}': label must not be empty", c.key));
        }
    }

    // rules
    let mut rule_names = HashSet::new();
    for r in &cfg.mail_rules {
        if r.name.trim().is_empty() {
            errs.push("mail rule: name must not be empty".to_string());
            continue;
        }
        if !rule_names.insert(lc(&r.name)) {
            errs.push(format!("duplicate mail rule name '{}'", r.name));
        }
        if let Err(e) = fa_rules::validate_rule(&r.conditions) {
            errs.push(format!("mail rule '{}': {e}", r.name));
        }
        if r.confidence.is_some_and(|c| !(0.0..=1.0).contains(&c)) {
            errs.push(format!("mail rule '{}': confidence must be 0-1", r.name));
        }
        match r.decision_point {
            DecisionPointKey::EmailTimeSensitive | DecisionPointKey::EmailIsBill
                if !r.value.is_boolean() =>
            {
                errs.push(format!(
                    "mail rule '{}': value must be true or false",
                    r.name
                ));
            }
            DecisionPointKey::EmailCategory => match r.value.as_str() {
                Some(k) if valid_category_key(k) => {}
                _ => errs.push(format!(
                    "mail rule '{}': value must be a category key",
                    r.name
                )),
            },
            DecisionPointKey::EmailMemberLink => {
                for v in member_link_items(&r.value) {
                    match v {
                        Some(s) if member_keys.contains(s) || is_member_token(s) => {}
                        _ => errs.push(format!(
                            "mail rule '{}': member_link value must be member keys from this file",
                            r.name
                        )),
                    }
                }
            }
            _ => {}
        }
    }
}

fn is_member_token(s: &str) -> bool {
    matches!(s, "owners" | "household") || uuid::Uuid::parse_str(s).is_ok()
}

fn member_link_items(v: &serde_json::Value) -> Vec<Option<&str>> {
    match v {
        serde_json::Value::Array(a) => a.iter().map(|x| x.as_str()).collect(),
        other => vec![other.as_str()],
    }
}

// ---- value translation (member keys / calendar refs <-> ids) ----

struct Refs<'a> {
    members: &'a HashMap<&'a str, MemberId>,
    cal_by_ref: &'a HashMap<String, String>,
}

fn to_store_value(dp: DecisionPointKey, v: &serde_json::Value, refs: &Refs) -> serde_json::Value {
    use serde_json::Value;
    let member = |s: &str| {
        Value::String(
            refs.members
                .get(s)
                .map(|m| m.to_string())
                .unwrap_or_else(|| s.to_string()),
        )
    };
    match (dp, v) {
        (DecisionPointKey::EmailMemberLink, Value::Array(a)) => Value::Array(
            a.iter()
                .map(|x| x.as_str().map(member).unwrap_or_else(|| x.clone()))
                .collect(),
        ),
        (DecisionPointKey::EmailMemberLink, Value::String(s)) => member(s),
        (DecisionPointKey::EventCalendarRoute, Value::String(s)) => {
            Value::String(refs.cal_by_ref.get(s).cloned().unwrap_or_else(|| s.clone()))
        }
        _ => v.clone(),
    }
}

fn to_file_value(
    dp: DecisionPointKey,
    v: &serde_json::Value,
    key_of: &HashMap<String, String>,
    cal_ref_of: &HashMap<String, String>,
) -> serde_json::Value {
    use serde_json::Value;
    let member = |s: &str| Value::String(key_of.get(s).cloned().unwrap_or_else(|| s.to_string()));
    match (dp, v) {
        (DecisionPointKey::EmailMemberLink, Value::Array(a)) => Value::Array(
            a.iter()
                .map(|x| x.as_str().map(member).unwrap_or_else(|| x.clone()))
                .collect(),
        ),
        (DecisionPointKey::EmailMemberLink, Value::String(s)) => member(s),
        (DecisionPointKey::EventCalendarRoute, Value::String(s)) => {
            Value::String(cal_ref_of.get(s).cloned().unwrap_or_else(|| s.clone()))
        }
        _ => v.clone(),
    }
}

// ---- import ----

fn pick<'a>(
    created: bool,
    changed: bool,
    c: &'a mut Vec<String>,
    u: &'a mut Vec<String>,
    n: &'a mut Vec<String>,
) -> &'a mut Vec<String> {
    if created {
        c
    } else if changed {
        u
    } else {
        n
    }
}

fn norm_list(v: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        let s = lc(s);
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

fn blank_none(s: &Option<String>) -> Option<String> {
    s.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Resolves `"<source key>/<remote_id>"` references to calendar ids.
async fn calendar_refs(
    conn: &mut PgConnection,
    cfg: &HouseholdConfig,
) -> Result<HashMap<String, fa_domain::CalendarId>, ImportError> {
    let sources = CalendarSourceRepo::list(conn, false).await?;
    let mut out = HashMap::new();
    for sc in &cfg.calendar_sources {
        let Some(src) = sources
            .iter()
            .find(|v| v.value.kind == sc.kind && v.value.name == sc.name)
        else {
            continue;
        };
        for c in CalendarRepo::list(conn, Some(src.value.id)).await? {
            out.insert(format!("{}/{}", sc.key, c.value.remote_id), c.value.id);
        }
    }
    Ok(out)
}

pub(crate) async fn import_mail(
    conn: &mut PgConnection,
    hid: HouseholdId,
    cfg: &HouseholdConfig,
    member_ids: &HashMap<&str, MemberId>,
    report: &mut ImportReport,
) -> Result<(), ImportError> {
    let cal_ids = calendar_refs(conn, cfg).await?;
    let cal_by_ref: HashMap<String, String> = cal_ids
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    // Resolve everything the file references that lives only in the database, and collect every
    // problem before writing anything.
    let mut errs = Vec::new();
    let db_orgs = OrgRepo::list(conn, false).await?;
    let mut db_org_contacts: HashMap<String, HashSet<String>> = HashMap::new();
    for o in &db_orgs {
        let emails = OrgContactRepo::list(conn, o.value.id)
            .await?
            .into_iter()
            .map(|c| lc(&c.value.email))
            .collect();
        db_org_contacts.insert(lc(&o.value.name), emails);
    }
    let file_orgs: HashSet<String> = cfg.orgs.iter().map(|o| lc(&o.name)).collect();
    for m in &cfg.members {
        for e in &m.enrollments {
            let k = lc(&e.org);
            if file_orgs.contains(&k) {
                continue;
            }
            match db_org_contacts.get(&k) {
                None => errs.push(format!(
                    "member '{}': enrollment org '{}' is neither in this file nor in the household",
                    m.key, e.org
                )),
                Some(have) => {
                    for c in &e.contacts {
                        if !have.contains(&lc(c)) {
                            errs.push(format!(
                                "member '{}', enrollment '{}': unknown contact '{c}'",
                                m.key, e.org
                            ));
                        }
                    }
                }
            }
        }
    }
    for r in &cfg.mail_rules {
        if r.decision_point == DecisionPointKey::EventCalendarRoute
            && let Some(s) = r.value.as_str()
            && split_cal_ref(s).is_some()
            && !cal_by_ref.contains_key(s)
            && uuid::Uuid::parse_str(s).is_err()
        {
            errs.push(format!(
                "mail rule '{}': calendar '{s}' does not exist in the household",
                r.name
            ));
        }
    }
    if !errs.is_empty() {
        return Err(HouseholdTomlError::Invalid(errs).into());
    }

    mail_defaults::ensure(conn, hid).await?;

    // Categories.
    let existing = CategoryRepo::list(conn, true).await?;
    let mut next_ord = existing
        .iter()
        .map(|v| v.value.ord)
        .max()
        .map_or(0, |o| o + 1);
    for c in &cfg.categories {
        let found = existing.iter().find(|v| v.value.key == c.key);
        let behaviour = c.behaviour.unwrap_or(CategoryBehaviour::Keep);
        let notify = c.notify_time_sensitive.unwrap_or(false);
        let actions = c
            .auto_actions
            .clone()
            .unwrap_or_else(|| vec![AutoAction::Label]);
        let tasks = c.email_tasks.unwrap_or(EmailTasksMode::Suggest);
        let color = blank_none(&c.color);
        let now = Utc::now();
        match found {
            Some(v) => {
                let mut n = v.value.clone();
                n.label = c.label.trim().to_string();
                n.behaviour = behaviour;
                n.notify_time_sensitive = notify;
                n.auto_actions = actions;
                n.email_tasks = tasks;
                n.color = color;
                n.archived_at = match (c.archived, v.value.archived_at) {
                    (true, None) => Some(now),
                    (true, at) => at,
                    (false, _) => None,
                };
                let o = &v.value;
                let changed = n.label != o.label
                    || n.behaviour != o.behaviour
                    || n.notify_time_sensitive != o.notify_time_sensitive
                    || n.auto_actions != o.auto_actions
                    || n.email_tasks != o.email_tasks
                    || n.color != o.color
                    || n.archived_at.is_some() != o.archived_at.is_some();
                if changed {
                    CategoryRepo::update(conn, &n, v.version).await?;
                }
                pick(
                    false,
                    changed,
                    &mut report.categories_created,
                    &mut report.categories_updated,
                    &mut report.categories_unchanged,
                )
                .push(c.key.clone());
            }
            None => {
                let cat = Category {
                    id: CategoryId::new(),
                    household_id: hid,
                    key: c.key.clone(),
                    label: c.label.trim().to_string(),
                    behaviour,
                    notify_time_sensitive: notify,
                    auto_actions: actions,
                    email_tasks: tasks,
                    color,
                    ord: next_ord,
                    archived_at: c.archived.then_some(now),
                    created_at: now,
                    updated_at: now,
                };
                next_ord += 1;
                let inserted = CategoryRepo::insert(conn, &cat).await?;
                if c.archived {
                    CategoryRepo::update(conn, &cat, inserted.version).await?;
                }
                report.categories_created.push(c.key.clone());
            }
        }
    }

    // Orgs and their contacts.
    for oc in &cfg.orgs {
        let domains = norm_list(&oc.domains);
        let senders = norm_list(&oc.senders);
        let tpl = oc.bill_template.clone().unwrap_or(BillTemplateConfig {
            amount: None,
            due: None,
            account: None,
        });
        let cal = |r: &Option<String>| r.as_ref().and_then(|r| cal_ids.get(r).copied());
        let mode = oc.contact_mode.unwrap_or(ContactMode::Suggest);
        let now = Utc::now();
        let found = OrgRepo::get_by_name(conn, oc.name.trim()).await?;
        let (org_id, org_changed, created) = match found {
            Some(v) => {
                let mut n = v.value.clone();
                n.kind = oc.kind;
                n.name = oc.name.trim().to_string();
                n.domains = domains;
                n.senders = senders;
                n.ics_calendar_id = cal(&oc.ics_calendar);
                n.default_calendar_id = cal(&oc.default_calendar);
                n.contact_mode = mode;
                n.bill_amount_regex = blank_none(&tpl.amount);
                n.bill_due_regex = blank_none(&tpl.due);
                n.bill_account_regex = blank_none(&tpl.account);
                let o = &v.value;
                let changed = n.kind != o.kind
                    || n.name != o.name
                    || n.domains != o.domains
                    || n.senders != o.senders
                    || n.ics_calendar_id != o.ics_calendar_id
                    || n.default_calendar_id != o.default_calendar_id
                    || n.contact_mode != o.contact_mode
                    || n.bill_amount_regex != o.bill_amount_regex
                    || n.bill_due_regex != o.bill_due_regex
                    || n.bill_account_regex != o.bill_account_regex;
                if changed {
                    OrgRepo::update(conn, &n, v.version).await?;
                }
                (o.id, changed, false)
            }
            None => {
                let o = Org {
                    id: OrgId::new(),
                    household_id: hid,
                    kind: oc.kind,
                    name: oc.name.trim().to_string(),
                    domains,
                    senders,
                    ics_calendar_id: cal(&oc.ics_calendar),
                    default_calendar_id: cal(&oc.default_calendar),
                    contact_mode: mode,
                    bill_amount_regex: blank_none(&tpl.amount),
                    bill_due_regex: blank_none(&tpl.due),
                    bill_account_regex: blank_none(&tpl.account),
                    archived_at: None,
                    created_at: now,
                    updated_at: now,
                };
                OrgRepo::insert(conn, &o).await?;
                (o.id, true, true)
            }
        };
        let have = OrgContactRepo::list(conn, org_id).await?;
        let mut contacts_changed = false;
        for cc in &oc.contacts {
            let email = lc(&cc.email);
            let role = cc.role.unwrap_or(ContactRole::Other);
            match have.iter().find(|v| lc(&v.value.email) == email) {
                Some(v) => {
                    if v.value.name != cc.name.trim() || v.value.role != role {
                        let mut n = v.value.clone();
                        n.name = cc.name.trim().to_string();
                        n.role = role;
                        OrgContactRepo::update(conn, &n, v.version).await?;
                        contacts_changed = true;
                    }
                }
                None => {
                    OrgContactRepo::insert(
                        conn,
                        &OrgContact {
                            id: OrgContactId::new(),
                            household_id: hid,
                            org_id,
                            name: cc.name.trim().to_string(),
                            email,
                            role,
                            archived_at: None,
                            created_at: now,
                            updated_at: now,
                        },
                    )
                    .await?;
                    contacts_changed = true;
                }
            }
        }
        pick(
            created,
            org_changed || contacts_changed,
            &mut report.orgs_created,
            &mut report.orgs_updated,
            &mut report.orgs_unchanged,
        )
        .push(oc.name.clone());
    }

    // Enrollments (after orgs and contacts exist).
    let orgs_now = OrgRepo::list(conn, false).await?;
    let org_by_name: HashMap<String, OrgId> = orgs_now
        .iter()
        .map(|v| (lc(&v.value.name), v.value.id))
        .collect();
    let mut contact_ids: HashMap<(OrgId, String), OrgContactId> = HashMap::new();
    for o in &orgs_now {
        for c in OrgContactRepo::list(conn, o.value.id).await? {
            contact_ids.insert((o.value.id, lc(&c.value.email)), c.value.id);
        }
    }
    for mc in &cfg.members {
        let Some(&mid) = member_ids.get(mc.key.as_str()) else {
            continue;
        };
        let have = EnrollmentRepo::list(conn, Some(mid)).await?;
        for ec in &mc.enrollments {
            let Some(&org_id) = org_by_name.get(&lc(&ec.org)) else {
                continue;
            };
            let want_contacts: Vec<OrgContactId> = {
                let mut v: Vec<OrgContactId> = ec
                    .contacts
                    .iter()
                    .filter_map(|e| contact_ids.get(&(org_id, lc(e))).copied())
                    .collect();
                v.sort_by_key(|c| c.as_uuid());
                v.dedup();
                v
            };
            let grade = blank_none(&ec.grade);
            let class_name = blank_none(&ec.class_name);
            let label = format!("{}: {}", mc.key, ec.org);
            let now = Utc::now();
            match have
                .iter()
                .find(|v| v.value.org_id == org_id && v.value.from_date == ec.from)
            {
                Some(v) => {
                    let o = &v.value;
                    let have_contacts = EnrollmentRepo::contacts(conn, o.id).await?;
                    let changed = o.grade != grade
                        || o.class_name != class_name
                        || o.until_date != ec.until
                        || have_contacts != want_contacts;
                    if changed {
                        let mut n = o.clone();
                        n.grade = grade;
                        n.class_name = class_name;
                        n.until_date = ec.until;
                        EnrollmentRepo::update(conn, &n, v.version).await?;
                        EnrollmentRepo::set_contacts(conn, o.id, &want_contacts).await?;
                    }
                    pick(
                        false,
                        changed,
                        &mut report.enrollments_created,
                        &mut report.enrollments_updated,
                        &mut report.enrollments_unchanged,
                    )
                    .push(label);
                }
                None => {
                    let e = Enrollment {
                        id: EnrollmentId::new(),
                        household_id: hid,
                        member_id: mid,
                        org_id,
                        grade,
                        class_name,
                        from_date: ec.from,
                        until_date: ec.until,
                        created_at: now,
                        updated_at: now,
                    };
                    EnrollmentRepo::insert(conn, &e).await?;
                    EnrollmentRepo::set_contacts(conn, e.id, &want_contacts).await?;
                    report.enrollments_created.push(label);
                }
            }
        }
    }

    // Mail sources.
    let sources = MailSourceRepo::list(conn, false).await?;
    for sc in &cfg.mail_sources {
        let owners: Vec<MemberId> = {
            let mut v: Vec<MemberId> = sc
                .owners
                .iter()
                .filter_map(|k| member_ids.get(k.as_str()).copied())
                .collect();
            v.sort_by_key(|m| m.as_uuid());
            v.dedup();
            v
        };
        let found = sources
            .iter()
            .find(|(v, _)| lc(&v.value.name) == lc(&sc.name));
        let imap = sc.imap.as_ref();
        let folders = |old: &[String]| -> Vec<String> {
            match imap {
                Some(i) if !i.folders.is_empty() => i.folders.clone(),
                Some(_) => vec!["INBOX".to_string()],
                None => old.to_vec(),
            }
        };
        match found {
            Some((v, have_owners)) => {
                let o = &v.value;
                let mut n = o.clone();
                n.name = sc.name.trim().to_string();
                if let Some(a) = &sc.address {
                    n.address = lc(a);
                }
                n.visibility = sc.visibility.unwrap_or(o.visibility);
                n.read_only = sc.read_only.unwrap_or(o.read_only);
                n.backfill_days = sc.backfill_days.unwrap_or(o.backfill_days);
                n.query = blank_none(&sc.query);
                n.poll_minutes = sc.poll_minutes.unwrap_or(o.poll_minutes);
                if o.provider == MailProvider::Imap
                    && let Some(i) = imap
                {
                    n.imap_host = Some(i.host.trim().to_string());
                    n.imap_port = i.port.map(i32::from).or(o.imap_port);
                    n.imap_username = Some(i.username.trim().to_string());
                    n.folders = folders(&o.folders);
                    n.archive_folder = blank_none(&i.archive_folder);
                }
                let mut want_owners = owners.clone();
                want_owners.sort_by_key(|m| m.as_uuid());
                let mut cur_owners = have_owners.clone();
                cur_owners.sort_by_key(|m| m.as_uuid());
                let changed = n.name != o.name
                    || n.address != o.address
                    || n.visibility != o.visibility
                    || n.read_only != o.read_only
                    || n.backfill_days != o.backfill_days
                    || n.query != o.query
                    || n.poll_minutes != o.poll_minutes
                    || n.imap_host != o.imap_host
                    || n.imap_port != o.imap_port
                    || n.imap_username != o.imap_username
                    || n.folders != o.folders
                    || n.archive_folder != o.archive_folder;
                if changed {
                    MailSourceRepo::update(conn, &n, v.version).await?;
                }
                let owners_changed = want_owners != cur_owners;
                if owners_changed {
                    MailSourceRepo::set_owners(conn, o.id, &owners).await?;
                }
                pick(
                    false,
                    changed || owners_changed,
                    &mut report.mail_sources_created,
                    &mut report.mail_sources_updated,
                    &mut report.mail_sources_unchanged,
                )
                .push(sc.name.clone());
            }
            None if sc.provider == MailProvider::Gmail => {
                // A Gmail source needs an OAuth account, which only the UI flow can create.
                tracing::warn!(
                    source = %sc.name,
                    "gmail mail source not created by import: connect the account in Settings"
                );
                report.mail_sources_skipped.push(sc.name.clone());
            }
            None => {
                let now = Utc::now();
                let i = imap.cloned().unwrap_or(ImapSection {
                    host: String::new(),
                    port: None,
                    username: String::new(),
                    folders: vec![],
                    archive_folder: None,
                });
                let s = MailSource {
                    id: MailSourceId::new(),
                    household_id: hid,
                    provider: sc.provider,
                    name: sc.name.trim().to_string(),
                    address: lc(sc.address.as_deref().unwrap_or(&i.username)),
                    oauth_account_id: None,
                    imap_host: Some(i.host.trim().to_string()),
                    imap_port: Some(i32::from(i.port.unwrap_or(993))),
                    imap_username: Some(i.username.trim().to_string()),
                    secret_id: None,
                    folders: folders(&[]),
                    archive_folder: blank_none(&i.archive_folder),
                    gmail_ext: false,
                    read_only: sc.read_only.unwrap_or(true),
                    visibility: sc.visibility.unwrap_or_default(),
                    backfill_days: sc.backfill_days.unwrap_or(30),
                    query: blank_none(&sc.query),
                    poll_minutes: sc.poll_minutes.unwrap_or(5),
                    history_id: None,
                    state: MailSourceState::NeedsReauth,
                    last_synced_at: None,
                    last_error: None,
                    archived_at: None,
                    created_at: now,
                    updated_at: now,
                };
                MailSourceRepo::insert(conn, &s, &owners).await?;
                report.mail_sources_created.push(sc.name.clone());
            }
        }
    }

    // Mail rules.
    let refs = Refs {
        members: member_ids,
        cal_by_ref: &cal_by_ref,
    };
    let rules = MailRuleRepo::list(conn, None, false).await?;
    let mut next_priority = rules
        .iter()
        .map(|v| v.value.priority)
        .max()
        .map_or(0, |p| p + 10);
    for rc in &cfg.mail_rules {
        let value = to_store_value(rc.decision_point, &rc.value, &refs);
        let confidence = rc.confidence.unwrap_or(0.9);
        match rules.iter().find(|v| lc(&v.value.name) == lc(&rc.name)) {
            Some(v) => {
                let o = &v.value;
                let mut n = o.clone();
                n.decision_point = rc.decision_point;
                n.priority = rc.priority.unwrap_or(o.priority);
                n.conditions = rc.conditions.clone();
                n.value = value;
                n.confidence = confidence;
                n.enabled = rc.enabled;
                let changed = n.decision_point != o.decision_point
                    || n.priority != o.priority
                    || n.conditions != o.conditions
                    || n.value != o.value
                    || n.confidence != o.confidence
                    || n.enabled != o.enabled;
                if changed {
                    MailRuleRepo::update(conn, &n, v.version).await?;
                }
                pick(
                    false,
                    changed,
                    &mut report.mail_rules_created,
                    &mut report.mail_rules_updated,
                    &mut report.mail_rules_unchanged,
                )
                .push(rc.name.clone());
            }
            None => {
                let priority = rc.priority.unwrap_or_else(|| {
                    let p = next_priority;
                    next_priority += 10;
                    p
                });
                let now = Utc::now();
                MailRuleRepo::insert(
                    conn,
                    &MailRule {
                        id: MailRuleId::new(),
                        household_id: hid,
                        decision_point: rc.decision_point,
                        name: rc.name.trim().to_string(),
                        priority,
                        conditions: rc.conditions.clone(),
                        value,
                        confidence,
                        enabled: rc.enabled,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                )
                .await?;
                report.mail_rules_created.push(rc.name.clone());
            }
        }
    }
    Ok(())
}

// ---- export ----

pub(crate) struct ExportedMail {
    pub orgs: Vec<OrgConfig>,
    pub mail_sources: Vec<MailSourceConfig>,
    pub categories: Vec<CategoryConfig>,
    pub mail_rules: Vec<MailRuleConfig>,
    /// Member id to its enrollments.
    pub enrollments: HashMap<MemberId, Vec<EnrollmentConfig>>,
}

/// Everything except secrets: no passwords, tokens, message data or decisions.
/// `cal_ref_of` maps a calendar id to its `"source/calendar"` reference.
pub(crate) async fn export_mail(
    conn: &mut PgConnection,
    id_to_key: &HashMap<MemberId, String>,
    cal_ref_of: &HashMap<fa_domain::CalendarId, String>,
) -> Result<ExportedMail, ImportError> {
    let db_orgs = OrgRepo::list(conn, false).await?;
    let mut org_name: HashMap<OrgId, String> = HashMap::new();
    let mut contact_email: HashMap<OrgContactId, String> = HashMap::new();
    let mut orgs = Vec::new();
    for v in &db_orgs {
        let o = &v.value;
        org_name.insert(o.id, o.name.clone());
        let mut contacts: Vec<OrgContactConfig> = Vec::new();
        for c in OrgContactRepo::list(conn, o.id).await? {
            let c = c.value;
            contact_email.insert(c.id, c.email.clone());
            contacts.push(OrgContactConfig {
                name: c.name,
                email: c.email,
                role: Some(c.role),
            });
        }
        contacts.sort_by_key(|c| (lc(&c.name), c.email.clone()));
        let tpl = BillTemplateConfig {
            amount: o.bill_amount_regex.clone(),
            due: o.bill_due_regex.clone(),
            account: o.bill_account_regex.clone(),
        };
        let cal = |c: Option<fa_domain::CalendarId>| c.and_then(|c| cal_ref_of.get(&c).cloned());
        orgs.push(OrgConfig {
            name: o.name.clone(),
            kind: o.kind,
            domains: o.domains.clone(),
            senders: o.senders.clone(),
            ics_calendar: cal(o.ics_calendar_id),
            default_calendar: cal(o.default_calendar_id),
            contact_mode: Some(o.contact_mode),
            bill_template: (!tpl.is_empty()).then_some(tpl),
            contacts,
        });
    }

    let mut enrollments: HashMap<MemberId, Vec<EnrollmentConfig>> = HashMap::new();
    for v in EnrollmentRepo::list(conn, None).await? {
        let e = v.value;
        if !id_to_key.contains_key(&e.member_id) {
            continue;
        }
        let Some(org) = org_name.get(&e.org_id) else {
            continue;
        };
        let mut contacts: Vec<String> = EnrollmentRepo::contacts(conn, e.id)
            .await?
            .into_iter()
            .filter_map(|c| contact_email.get(&c).cloned())
            .collect();
        contacts.sort();
        enrollments
            .entry(e.member_id)
            .or_default()
            .push(EnrollmentConfig {
                org: org.clone(),
                grade: e.grade,
                class_name: e.class_name,
                from: e.from_date,
                until: e.until_date,
                contacts,
            });
    }
    for list in enrollments.values_mut() {
        list.sort_by_key(|e| (lc(&e.org), e.from, e.class_name.clone()));
    }

    let mut mail_sources: Vec<MailSourceConfig> = MailSourceRepo::list(conn, false)
        .await?
        .into_iter()
        .map(|(v, owners)| {
            let s = v.value;
            let mut owners: Vec<String> = owners
                .into_iter()
                .filter_map(|m| id_to_key.get(&m).cloned())
                .collect();
            owners.sort();
            let imap = (s.provider == MailProvider::Imap).then(|| ImapSection {
                host: s.imap_host.clone().unwrap_or_default(),
                port: s.imap_port.and_then(|p| u16::try_from(p).ok()),
                username: s.imap_username.clone().unwrap_or_default(),
                folders: s.folders.clone(),
                archive_folder: s.archive_folder.clone(),
            });
            MailSourceConfig {
                name: s.name,
                provider: s.provider,
                address: Some(s.address).filter(|a| !a.is_empty()),
                owners,
                visibility: Some(s.visibility),
                read_only: Some(s.read_only),
                backfill_days: Some(s.backfill_days),
                query: s.query,
                poll_minutes: Some(s.poll_minutes),
                imap,
            }
        })
        .collect();
    mail_sources.sort_by_key(|s| lc(&s.name));

    // Only categories that differ from the seeded defaults (D-054) are listed: custom ones and
    // edited or archived defaults. Reordering a default is not exported.
    let defaults = fa_domain::default_categories();
    let categories = CategoryRepo::list(conn, true)
        .await?
        .into_iter()
        .filter(|v| {
            let c = &v.value;
            match defaults.iter().find(|d| d.key == c.key) {
                None => true,
                Some(d) => {
                    c.label != d.label
                        || c.behaviour != d.behaviour
                        || c.notify_time_sensitive != d.notify_time_sensitive
                        || c.auto_actions != [AutoAction::Label]
                        || c.email_tasks != EmailTasksMode::Suggest
                        || c.color.is_some()
                        || c.archived_at.is_some()
                }
            }
        })
        .map(|v| {
            let c = v.value;
            CategoryConfig {
                key: c.key,
                label: c.label,
                behaviour: Some(c.behaviour),
                notify_time_sensitive: Some(c.notify_time_sensitive),
                auto_actions: Some(c.auto_actions),
                email_tasks: Some(c.email_tasks),
                color: c.color,
                archived: c.archived_at.is_some(),
            }
        })
        .collect();

    let key_of: HashMap<String, String> = id_to_key
        .iter()
        .map(|(m, k)| (m.to_string(), k.clone()))
        .collect();
    let cal_str: HashMap<String, String> = cal_ref_of
        .iter()
        .map(|(c, r)| (c.to_string(), r.clone()))
        .collect();
    let mail_rules = MailRuleRepo::list(conn, None, false)
        .await?
        .into_iter()
        .map(|v| {
            let r = v.value;
            MailRuleConfig {
                value: to_file_value(r.decision_point, &r.value, &key_of, &cal_str),
                name: r.name,
                decision_point: r.decision_point,
                priority: Some(r.priority),
                conditions: r.conditions,
                confidence: Some(r.confidence),
                enabled: r.enabled,
            }
        })
        .collect();

    Ok(ExportedMail {
        orgs,
        mail_sources,
        categories,
        mail_rules,
        enrollments,
    })
}
