use std::collections::HashSet;

use chrono::{NaiveDate, Utc};
use fa_domain::Role;
use fa_domain::system_api::NewSetupHousehold;
use fa_domain::theme::Hue;
use fa_domain::timefmt::{format_hhmm, parse_hhmm};
use fa_domain::{
    AuditEntry, AuditEntryId, Gender, Household, HouseholdId, Location, Member, MemberId,
    RuleAction, RuleStatus, Severity, SourceKind, Units,
};
use fa_store::{AuditRepo, HouseholdRepo, MemberRepo, Store, StoreError};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const SUPPORTED_SECTIONS: &[&str] = &[
    "schema_version",
    "household",
    "members",
    "calendar_sources",
    "weather_rules",
    "task_lists",
    "task_definitions",
    "routines",
    "retention",
    "orgs",
    "mail_sources",
    "categories",
    "mail_rules",
];

pub const DEFAULT_UNITS: Units = Units::Metric;
pub const DEFAULT_CALENDAR_PAST_DAYS: i32 = 30;
pub const DEFAULT_CALENDAR_FUTURE_DAYS: i32 = 400;
pub const DEFAULT_WEATHER_RETENTION_DAYS: i32 = 7;
pub const DEFAULT_PRIVATE_EVENTS_AS_BUSY: bool = true;

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub household: HouseholdSection,
    #[serde(default)]
    pub members: Vec<MemberConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calendar_sources: Vec<CalendarSourceConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weather_rules: Vec<WeatherRuleConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_lists: Vec<crate::tasks::TaskListConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_definitions: Vec<crate::tasks::TaskDefinitionConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routines: Vec<crate::tasks::RoutineConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention: Option<crate::mail::RetentionConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orgs: Vec<crate::mail::OrgConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mail_sources: Vec<crate::mail::MailSourceConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<crate::mail::CategoryConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mail_rules: Vec<crate::mail::MailRuleConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdSection {
    pub name: String,
    pub tz: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<Units>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_past_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_future_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather_retention_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_events_as_busy: Option<bool>,
    /// Let dashboards tick off tasks (omitted: unchanged; new households: on).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_completion_override: Option<bool>,
    /// Size of the Now list (1-50, default 7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now_list_size: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now_weights: Option<fa_domain::tasks::NowWeights>,
    /// `"HH:MM"`, default 21:00.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_from: Option<String>,
    /// `"HH:MM"`, default 07:00.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_until: Option<String>,
    /// ISO 4217 code for bills, default `USD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Words that mean girls / boys in school mail (omitted: unchanged; new households: English).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender_terms: Option<crate::mail::GenderTermsConfig>,
    /// Keys of built-in classification signals to switch off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_signals: Option<Vec<String>>,
    /// Days before a bill's due date its pay task is due (0-60, default 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bill_lead_days: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<crate::mail::EventsConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox: Option<crate::mail::InboxConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationConfig {
    pub label: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberConfig {
    /// Stable slug identifying the member across imports.
    pub key: String,
    pub display_name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "hue_or_legacy_hex"
    )]
    pub color: Option<Hue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub hobbies: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<crate::tasks::NotifyConfig>,
    /// Other names the member goes by in mail (nicknames, short forms).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    /// Words that mark mail as relevant to this member (a team, an instrument).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relevance_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enrollments: Vec<crate::mail::EnrollmentConfig>,
}

/// A calendar account. Secrets are never part of the file: Google needs reconnecting, a CalDAV
/// password and an ICS URL must be entered again in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarSourceConfig {
    /// File-local identifier (used in error messages). Sources are matched by `kind` and `name`.
    pub key: String,
    pub kind: SourceKind,
    pub name: String,
    /// Member key; absent means a household source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// CalDAV base URL. Accepted but ignored for ICS (the URL is a secret, enter it in the UI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calendars: Vec<CalendarConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarConfig {
    pub remote_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    /// Calendars no longer have a colour; old files that still carry one load and drop it.
    #[serde(default, skip_serializing)]
    pub color: Option<String>,
    /// Member keys. Empty means a household calendar.
    #[serde(default)]
    pub members: Vec<String>,
}

fn default_rule_status() -> RuleStatus {
    RuleStatus::Active
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherRuleConfig {
    pub name: String,
    /// Weather-rule DSL; validated by `fa-server config import`, stored as given.
    pub condition: String,
    pub action: RuleAction,
    pub severity: Severity,
    pub message: String,
    #[serde(default = "default_rule_status")]
    pub status: RuleStatus,
    /// Defaults to `create`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_mode: Option<fa_domain::TaskMode>,
    /// Title of a task definition (or `"List name/Title"` when ambiguous), for action `task`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_definition: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum HouseholdTomlError {
    #[error("invalid household TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported top-level section(s) {found:?}; supported sections: {supported}")]
    UnknownSections {
        found: Vec<String>,
        supported: String,
    },
    #[error("unsupported schema_version {0}; this build supports {SCHEMA_VERSION}")]
    SchemaVersion(u32),
    #[error("invalid household config:\n  - {}", .0.join("\n  - "))]
    Invalid(Vec<String>),
    #[error("could not serialize household TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error(transparent)]
    Config(#[from] HouseholdTomlError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("household {0} not found")]
    HouseholdNotFound(HouseholdId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    /// Upsert what the file lists; leave other members alone.
    Merge,
    /// Like merge, and archive keyed members the file no longer lists.
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ImportReport {
    pub household_id: HouseholdId,
    pub household_created: bool,
    pub household_updated: bool,
    pub members_created: Vec<String>,
    pub members_updated: Vec<String>,
    pub members_unchanged: Vec<String>,
    pub members_archived: Vec<String>,
    pub sources_created: Vec<String>,
    pub sources_updated: Vec<String>,
    pub sources_unchanged: Vec<String>,
    pub rules_created: Vec<String>,
    pub rules_updated: Vec<String>,
    pub rules_unchanged: Vec<String>,
    pub task_lists_created: Vec<String>,
    pub task_lists_updated: Vec<String>,
    pub task_lists_unchanged: Vec<String>,
    pub task_definitions_created: Vec<String>,
    pub task_definitions_updated: Vec<String>,
    pub task_definitions_unchanged: Vec<String>,
    pub routines_created: Vec<String>,
    pub routines_updated: Vec<String>,
    pub routines_unchanged: Vec<String>,
    pub notify_created: Vec<String>,
    pub notify_updated: Vec<String>,
    pub notify_unchanged: Vec<String>,
    pub orgs_created: Vec<String>,
    pub orgs_updated: Vec<String>,
    pub orgs_unchanged: Vec<String>,
    pub enrollments_created: Vec<String>,
    pub enrollments_updated: Vec<String>,
    pub enrollments_unchanged: Vec<String>,
    pub mail_sources_created: Vec<String>,
    pub mail_sources_updated: Vec<String>,
    pub mail_sources_unchanged: Vec<String>,
    /// Gmail sources that need an OAuth connection made in the UI first.
    pub mail_sources_skipped: Vec<String>,
    pub categories_created: Vec<String>,
    pub categories_updated: Vec<String>,
    pub categories_unchanged: Vec<String>,
    pub mail_rules_created: Vec<String>,
    pub mail_rules_updated: Vec<String>,
    pub mail_rules_unchanged: Vec<String>,
}

impl HouseholdConfig {
    /// `(definition title, rrule)` of every task definition, for syntax checks by the caller.
    pub fn rrules(&self) -> impl Iterator<Item = (&str, &str)> {
        self.task_definitions
            .iter()
            .filter_map(|d| d.rrule.as_deref().map(|r| (d.title.as_str(), r)))
    }

    /// `(label, condition)` of every weather-rule condition and task-definition
    /// `season_condition`, for DSL checks by the caller.
    pub fn conditions(&self) -> impl Iterator<Item = (String, &str)> {
        let rules = self
            .weather_rules
            .iter()
            .map(|r| (format!("weather rule '{}'", r.name), r.condition.as_str()));
        let seasons = self.task_definitions.iter().filter_map(|d| {
            d.season_condition
                .as_deref()
                .map(|c| (format!("task definition '{}' season_condition", d.title), c))
        });
        rules.chain(seasons)
    }

    pub fn from_toml(s: &str) -> Result<Self, HouseholdTomlError> {
        let table: toml::Table = s.parse()?;
        let unknown: Vec<String> = table
            .keys()
            .filter(|k| !SUPPORTED_SECTIONS.contains(&k.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(HouseholdTomlError::UnknownSections {
                found: unknown,
                supported: SUPPORTED_SECTIONS.join(", "),
            });
        }
        let cfg: Self = toml::from_str(s)?;
        cfg.validate_with_source(Some(s))?;
        Ok(cfg)
    }

    pub fn to_toml(&self) -> Result<String, HouseholdTomlError> {
        let out = toml::to_string_pretty(self)?;
        Ok(crate::feeds::annotate_sources(&out, &self.calendar_sources))
    }

    pub fn validate(&self) -> Result<(), HouseholdTomlError> {
        self.validate_with_source(None)
    }

    /// With the file text, unknown member keys are reported with their line number.
    fn validate_with_source(&self, src: Option<&str>) -> Result<(), HouseholdTomlError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(HouseholdTomlError::SchemaVersion(self.schema_version));
        }
        let mut errs = Vec::new();
        if self.household.name.trim().is_empty() {
            errs.push("household.name must not be empty".to_string());
        }
        if self.household.tz.parse::<chrono_tz::Tz>().is_err() {
            errs.push(format!(
                "household.tz '{}' is not a valid IANA time zone",
                self.household.tz
            ));
        }
        if let Some(l) = &self.household.location
            && !((-90.0..=90.0).contains(&l.lat) && (-180.0..=180.0).contains(&l.lon))
        {
            errs.push("household.location lat/lon out of range".to_string());
        }
        let h = &self.household;
        if let Some(d) = h.calendar_past_days
            && !(0..=3650).contains(&d)
        {
            errs.push("household.calendar_past_days must be 0-3650".to_string());
        }
        if let Some(d) = h.calendar_future_days
            && !(1..=3650).contains(&d)
        {
            errs.push("household.calendar_future_days must be 1-3650".to_string());
        }
        if let Some(d) = h.weather_retention_days
            && !(1..=365).contains(&d)
        {
            errs.push("household.weather_retention_days must be 1-365".to_string());
        }
        let mut seen = HashSet::new();
        for m in &self.members {
            let ok = !m.key.is_empty()
                && m.key.len() <= 64
                && m.key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
            if !ok {
                errs.push(format!(
                    "member key '{}' must be a slug of a-z, 0-9, '-' or '_' (max 64)",
                    m.key
                ));
            }
            if !seen.insert(m.key.as_str()) {
                errs.push(format!("duplicate member key '{}'", m.key));
            }
            if m.display_name.trim().is_empty() {
                errs.push(format!(
                    "member '{}': display_name must not be empty",
                    m.key
                ));
            }
        }
        crate::feeds::validate_feeds(self, src, &mut errs);
        crate::tasks::validate_tasks(self, &mut errs);
        crate::mail::validate_mail(self, &mut errs);
        if errs.is_empty() {
            Ok(())
        } else {
            Err(HouseholdTomlError::Invalid(errs))
        }
    }
}

/// A hue name, or a legacy `#rrggbb` string (maps to blue).
fn hue_or_legacy_hex<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Hue>, D::Error> {
    let Some(s) = Option::<String>::deserialize(d)? else {
        return Ok(None);
    };
    if s.starts_with('#') {
        return Ok(Some(Hue::DEFAULT));
    }
    s.parse().map(Some).map_err(serde::de::Error::custom)
}

fn apply(m: &mut Member, c: &MemberConfig) -> bool {
    let before = (
        m.display_name.clone(),
        m.role,
        m.avatar.clone(),
        m.color,
        m.birthdate,
        m.grade.clone(),
        m.interests.clone(),
        m.hobbies.clone(),
        m.responsibilities.clone(),
        m.aliases.clone(),
        m.gender,
        m.relevance_tags.clone(),
    );
    m.display_name = c.display_name.clone();
    m.role = c.role;
    m.avatar = c.avatar.clone();
    m.color = c.color;
    m.birthdate = c.birthdate;
    m.grade = c.grade.clone();
    m.interests = c.interests.clone();
    m.hobbies = c.hobbies.clone();
    m.responsibilities = c.responsibilities.clone();
    m.aliases = c.aliases.clone();
    m.gender = c.gender;
    m.relevance_tags = c.relevance_tags.clone();
    before
        != (
            m.display_name.clone(),
            m.role,
            m.avatar.clone(),
            m.color,
            m.birthdate,
            m.grade.clone(),
            m.interests.clone(),
            m.hobbies.clone(),
            m.responsibilities.clone(),
            m.aliases.clone(),
            m.gender,
            m.relevance_tags.clone(),
        )
}

/// Imports `cfg` into the store. Creates the household when missing (with `household_id` if
/// given, else a fresh id). Never touches accounts, PINs or secrets. A dry run does all the
/// work inside the transaction and builds the report, then rolls back.
pub async fn import(
    store: &Store,
    household_id: Option<HouseholdId>,
    cfg: &HouseholdConfig,
    mode: ImportMode,
    dry_run: bool,
) -> Result<ImportReport, ImportError> {
    cfg.validate()?;
    let hid = household_id.unwrap_or_default();
    let sec = &cfg.household;
    let location = cfg.household.location.as_ref().map(|l| Location {
        lat: l.lat,
        lon: l.lon,
        label: l.label.clone(),
    });
    let mut tx = store.tx_system().await?;
    let mut report = ImportReport {
        household_id: hid,
        household_created: false,
        household_updated: false,
        members_created: vec![],
        members_updated: vec![],
        members_unchanged: vec![],
        members_archived: vec![],
        sources_created: vec![],
        sources_updated: vec![],
        sources_unchanged: vec![],
        rules_created: vec![],
        rules_updated: vec![],
        rules_unchanged: vec![],
        task_lists_created: vec![],
        task_lists_updated: vec![],
        task_lists_unchanged: vec![],
        task_definitions_created: vec![],
        task_definitions_updated: vec![],
        task_definitions_unchanged: vec![],
        routines_created: vec![],
        routines_updated: vec![],
        routines_unchanged: vec![],
        notify_created: vec![],
        notify_updated: vec![],
        notify_unchanged: vec![],
        orgs_created: vec![],
        orgs_updated: vec![],
        orgs_unchanged: vec![],
        enrollments_created: vec![],
        enrollments_updated: vec![],
        enrollments_unchanged: vec![],
        mail_sources_created: vec![],
        mail_sources_updated: vec![],
        mail_sources_unchanged: vec![],
        mail_sources_skipped: vec![],
        categories_created: vec![],
        categories_updated: vec![],
        categories_unchanged: vec![],
        mail_rules_created: vec![],
        mail_rules_updated: vec![],
        mail_rules_unchanged: vec![],
    };

    match HouseholdRepo::get(&mut tx, hid).await? {
        None => {
            let mut h = new_household(
                hid,
                &cfg.household.name,
                &cfg.household.tz,
                location,
                sec.units.unwrap_or(DEFAULT_UNITS),
                sec.currency
                    .clone()
                    .unwrap_or_else(|| fa_domain::DEFAULT_CURRENCY.to_string()),
            );
            h.dashboard_completion_override = sec.dashboard_completion_override.unwrap_or(true);
            h.calendar_past_days = sec.calendar_past_days.unwrap_or(DEFAULT_CALENDAR_PAST_DAYS);
            h.calendar_future_days = sec
                .calendar_future_days
                .unwrap_or(DEFAULT_CALENDAR_FUTURE_DAYS);
            h.weather_retention_days = sec
                .weather_retention_days
                .unwrap_or(DEFAULT_WEATHER_RETENTION_DAYS);
            h.private_events_as_busy = sec
                .private_events_as_busy
                .unwrap_or(DEFAULT_PRIVATE_EVENTS_AS_BUSY);
            h.now_list_size = sec.now_list_size.unwrap_or(7);
            h.now_weights = sec.now_weights.unwrap_or_default();
            h.quiet_from = quiet(sec.quiet_from.as_deref(), 21);
            h.quiet_until = quiet(sec.quiet_until.as_deref(), 7);
            h.gender_terms = gender_terms(sec, Default::default());
            h.disabled_signals = disabled_signals(sec);
            h.events = events(sec, Default::default());
            h.inbox = inbox(sec, Default::default());
            h.retention_mail = retention(cfg, Default::default());
            h.bill_lead_days = sec
                .bill_lead_days
                .unwrap_or(fa_domain::DEFAULT_BILL_LEAD_DAYS);
            HouseholdRepo::insert(&mut tx, &h).await?;
            report.household_created = true;
        }
        Some(v) => {
            let mut h = v.value;
            let before = h.clone();
            h.name = sec.name.clone();
            h.tz = sec.tz.clone();
            h.location = location;
            if let Some(u) = sec.units {
                h.units = u;
            }
            if let Some(d) = sec.calendar_past_days {
                h.calendar_past_days = d;
            }
            if let Some(d) = sec.calendar_future_days {
                h.calendar_future_days = d;
            }
            if let Some(d) = sec.weather_retention_days {
                h.weather_retention_days = d;
            }
            if let Some(b) = sec.private_events_as_busy {
                h.private_events_as_busy = b;
            }
            if let Some(b) = sec.dashboard_completion_override {
                h.dashboard_completion_override = b;
            }
            if let Some(n) = sec.now_list_size {
                h.now_list_size = n;
            }
            if let Some(w) = sec.now_weights {
                h.now_weights = w;
            }
            if let Some(t) = sec.quiet_from.as_deref().and_then(parse_hhmm) {
                h.quiet_from = t;
            }
            if let Some(t) = sec.quiet_until.as_deref().and_then(parse_hhmm) {
                h.quiet_until = t;
            }
            if let Some(c) = &sec.currency {
                h.currency = c.clone();
            }
            h.gender_terms = gender_terms(sec, h.gender_terms.clone());
            if sec.disabled_signals.is_some() {
                h.disabled_signals = disabled_signals(sec);
            }
            if let Some(n) = sec.bill_lead_days {
                h.bill_lead_days = n;
            }
            h.events = events(sec, h.events.clone());
            h.inbox = inbox(sec, h.inbox.clone());
            h.retention_mail = retention(cfg, h.retention_mail.clone());
            let changed = h.currency != before.currency
                || h.gender_terms != before.gender_terms
                || h.disabled_signals != before.disabled_signals
                || h.bill_lead_days != before.bill_lead_days
                || h.events != before.events
                || h.inbox != before.inbox
                || h.retention_mail != before.retention_mail
                || h.now_list_size != before.now_list_size
                || h.now_weights != before.now_weights
                || h.quiet_from != before.quiet_from
                || h.quiet_until != before.quiet_until
                || h.name != before.name
                || h.tz != before.tz
                || h.location != before.location
                || h.units != before.units
                || h.calendar_past_days != before.calendar_past_days
                || h.calendar_future_days != before.calendar_future_days
                || h.weather_retention_days != before.weather_retention_days
                || h.private_events_as_busy != before.private_events_as_busy
                || h.dashboard_completion_override != before.dashboard_completion_override;
            if changed {
                HouseholdRepo::update(&mut tx, &h, v.version).await?;
                report.household_updated = true;
            }
        }
    }

    // Scope the rest of the transaction to this household: the repos' list and by-name reads
    // (calendar sources, orgs, rules, ...) must not see other households' rows.
    sqlx::query(
        "SELECT set_config('app.system', 'off', true), set_config('app.household_id', $1, true)",
    )
    .bind(hid.to_string())
    .execute(&mut *tx)
    .await
    .map_err(StoreError::from)?;

    for c in &cfg.members {
        match MemberRepo::get_by_config_key(&mut tx, hid, &c.key).await? {
            Some(v) => {
                let mut m = v.value;
                let was_archived = m.archived_at.is_some();
                if was_archived {
                    MemberRepo::unarchive(&mut tx, m.id).await?;
                }
                let v_version = if was_archived {
                    v.version + 1
                } else {
                    v.version
                };
                if apply(&mut m, c) {
                    MemberRepo::update(&mut tx, &m, v_version).await?;
                    report.members_updated.push(c.key.clone());
                } else if was_archived {
                    report.members_updated.push(c.key.clone());
                } else {
                    report.members_unchanged.push(c.key.clone());
                }
            }
            None => {
                let now = Utc::now();
                let m = Member {
                    id: MemberId::new(),
                    household_id: hid,
                    display_name: c.display_name.clone(),
                    role: c.role,
                    avatar: c.avatar.clone(),
                    color: c.color,
                    birthdate: c.birthdate,
                    grade: c.grade.clone(),
                    interests: c.interests.clone(),
                    hobbies: c.hobbies.clone(),
                    responsibilities: c.responsibilities.clone(),
                    aliases: c.aliases.clone(),
                    gender: c.gender,
                    relevance_tags: c.relevance_tags.clone(),
                    pin_hash: None,
                    archived_at: None,
                    created_at: now,
                    updated_at: now,
                };
                MemberRepo::insert(&mut tx, &m, Some(&c.key), None).await?;
                report.members_created.push(c.key.clone());
            }
        }
    }

    if mode == ImportMode::Replace {
        let listed: HashSet<&str> = cfg.members.iter().map(|m| m.key.as_str()).collect();
        for v in MemberRepo::list(&mut tx, false).await? {
            let m = v.value;
            if m.household_id != hid {
                continue;
            }
            if let Some(key) = MemberRepo::config_key(&mut tx, m.id).await?
                && !listed.contains(key.as_str())
            {
                MemberRepo::archive(&mut tx, m.id).await?;
                report.members_archived.push(key);
            }
        }
    }

    crate::feeds::import_feeds(&mut tx, hid, cfg, &mut report).await?;
    let tz: chrono_tz::Tz = cfg.household.tz.parse().unwrap_or(chrono_tz::UTC);
    let mut member_ids: std::collections::HashMap<&str, MemberId> = Default::default();
    for m in &cfg.members {
        let v = MemberRepo::get_by_config_key(&mut tx, hid, &m.key)
            .await?
            .ok_or(StoreError::NotFound)?;
        member_ids.insert(m.key.as_str(), v.value.id);
    }
    crate::tasks::import_tasks(&mut tx, hid, tz, cfg, &member_ids, &mut report).await?;
    crate::tasks::import_rule_tasks(&mut tx, cfg, &mut report).await?;
    crate::mail::import_mail(&mut tx, hid, cfg, &member_ids, &mut report).await?;

    AuditRepo::insert(
        &mut tx,
        &AuditEntry {
            id: AuditEntryId::new(),
            household_id: hid,
            actor_member: None,
            device: None,
            action: "config.import".into(),
            subject_type: "household".into(),
            subject_id: Some(hid.to_string()),
            before: None,
            after: serde_json::to_value(&report).ok().map(|mut v| {
                v["mode"] = format!("{mode:?}").to_lowercase().into();
                v
            }),
            at: Utc::now(),
        },
    )
    .await?;
    if dry_run {
        tx.rollback().await?;
        tracing::info!(household = %hid, ?mode, "household config import dry run");
    } else {
        tx.commit().await?;
        tracing::info!(household = %hid, ?mode, "household config imported");
    }
    Ok(report)
}

/// A household with every setting at its default.
fn new_household(
    id: HouseholdId,
    name: &str,
    tz: &str,
    location: Option<Location>,
    units: Units,
    currency: String,
) -> Household {
    let now = Utc::now();
    Household {
        id,
        name: name.to_string(),
        tz: tz.to_string(),
        location,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: true,
        units,
        week_start: Default::default(),
        calendar_past_days: DEFAULT_CALENDAR_PAST_DAYS,
        calendar_future_days: DEFAULT_CALENDAR_FUTURE_DAYS,
        weather_retention_days: DEFAULT_WEATHER_RETENTION_DAYS,
        private_events_as_busy: DEFAULT_PRIVATE_EVENTS_AS_BUSY,
        now_list_size: 7,
        now_weights: Default::default(),
        quiet_from: quiet(None, 21),
        quiet_until: quiet(None, 7),
        currency,
        gender_terms: Default::default(),
        disabled_signals: vec![],
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: now,
        updated_at: now,
    }
}

/// Creates a household with defaults from the setup wizard's first step.
pub async fn create_household(
    store: &Store,
    new: &NewSetupHousehold,
) -> Result<HouseholdId, ImportError> {
    let mut errs = Vec::new();
    let name = new.name.trim();
    if name.is_empty() {
        errs.push("household name must not be empty".to_string());
    }
    if new.tz.parse::<chrono_tz::Tz>().is_err() {
        errs.push(format!("'{}' is not an IANA time zone", new.tz));
    }
    let currency = new.currency.trim().to_ascii_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        errs.push(format!(
            "'{}' is not an ISO 4217 currency code",
            new.currency
        ));
    }
    if let Some(l) = &new.location
        && !((-90.0..=90.0).contains(&l.lat) && (-180.0..=180.0).contains(&l.lon))
    {
        errs.push("location latitude or longitude is out of range".to_string());
    }
    if !errs.is_empty() {
        return Err(HouseholdTomlError::Invalid(errs).into());
    }
    let id = HouseholdId::new();
    let h = new_household(
        id,
        name,
        &new.tz,
        new.location.as_ref().map(|l| Location {
            lat: l.lat,
            lon: l.lon,
            label: l.label.clone(),
        }),
        new.units,
        currency,
    );
    let mut tx = store.tx_system().await?;
    HouseholdRepo::insert(&mut tx, &h).await?;
    tx.commit().await?;
    tracing::info!(household = %id, "household created");
    Ok(id)
}

fn gender_terms(sec: &HouseholdSection, current: fa_domain::GenderTerms) -> fa_domain::GenderTerms {
    match &sec.gender_terms {
        Some(g) => fa_domain::GenderTerms {
            female: g.female.iter().map(|t| t.trim().to_string()).collect(),
            male: g.male.iter().map(|t| t.trim().to_string()).collect(),
        },
        None => current,
    }
}

fn disabled_signals(sec: &HouseholdSection) -> Vec<String> {
    let mut v = sec.disabled_signals.clone().unwrap_or_default();
    v.sort();
    v.dedup();
    v
}

fn events(sec: &HouseholdSection, cur: fa_domain::EventsSettings) -> fa_domain::EventsSettings {
    let Some(e) = &sec.events else { return cur };
    fa_domain::EventsSettings {
        auto_write: e.auto_write.unwrap_or(cur.auto_write),
        extract_threshold: e.extract_threshold.unwrap_or(cur.extract_threshold),
        route_threshold: e.route_threshold.unwrap_or(cur.route_threshold),
    }
}

fn inbox(sec: &HouseholdSection, cur: fa_domain::InboxSettings) -> fa_domain::InboxSettings {
    let Some(i) = &sec.inbox else { return cur };
    fa_domain::InboxSettings {
        task_expiry_days: i.task_expiry_days.unwrap_or(cur.task_expiry_days),
        contact_expiry_days: i.contact_expiry_days.unwrap_or(cur.contact_expiry_days),
    }
}

fn retention(cfg: &HouseholdConfig, cur: fa_domain::MailRetention) -> fa_domain::MailRetention {
    let Some(m) = cfg.retention.as_ref().and_then(|r| r.mail.as_ref()) else {
        return cur;
    };
    fa_domain::MailRetention {
        content_policy: m.content_policy.unwrap_or(cur.content_policy),
        examples_per_category: m.examples_per_category.unwrap_or(cur.examples_per_category),
        max_pending_days: m.max_pending_days.unwrap_or(cur.max_pending_days),
    }
}

fn quiet(s: Option<&str>, default_hour: u32) -> chrono::NaiveTime {
    s.and_then(parse_hhmm)
        .or_else(|| chrono::NaiveTime::from_hms_opt(default_hour, 0, 0))
        .unwrap_or(chrono::NaiveTime::MIN)
}

fn slug(s: &str) -> String {
    let mut out: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    out = out.trim_matches('-').to_string();
    if out.is_empty() { "member".into() } else { out }
}

/// Redacted export: profile data only, no hashes, tokens, accounts or secrets. Members created
/// outside a config file get a key derived from their display name.
pub async fn export(
    store: &Store,
    household_id: HouseholdId,
) -> Result<HouseholdConfig, ImportError> {
    let mut tx = store.tx_for(household_id).await?;
    let h = HouseholdRepo::get(&mut tx, household_id)
        .await?
        .ok_or(ImportError::HouseholdNotFound(household_id))?
        .value;
    let mut used = HashSet::new();
    let mut members = Vec::new();
    let mut keyless = Vec::new();
    for v in MemberRepo::list(&mut tx, false).await? {
        let m = v.value;
        match MemberRepo::config_key(&mut tx, m.id).await? {
            Some(k) => {
                used.insert(k.clone());
                members.push((k, m));
            }
            None => keyless.push(m),
        }
    }
    for m in keyless {
        let base = slug(&m.display_name);
        let mut key = base.clone();
        let mut n = 2;
        while !used.insert(key.clone()) {
            key = format!("{base}-{n}");
            n += 1;
        }
        members.push((key, m));
    }
    let id_to_key: std::collections::HashMap<MemberId, String> =
        members.iter().map(|(k, m)| (m.id, k.clone())).collect();
    let ex = crate::tasks::export_tasks(&mut tx, &id_to_key).await?;
    let (calendar_sources, weather_rules, cal_refs) =
        crate::feeds::export_feeds(&mut tx, &id_to_key, &ex).await?;
    let mut mail = crate::mail::export_mail(&mut tx, &id_to_key, &cal_refs).await?;
    tx.rollback().await?;

    Ok(HouseholdConfig {
        schema_version: SCHEMA_VERSION,
        household: HouseholdSection {
            name: h.name,
            tz: h.tz,
            location: h.location.map(|l| LocationConfig {
                label: l.label,
                lat: l.lat,
                lon: l.lon,
            }),
            units: Some(h.units),
            calendar_past_days: Some(h.calendar_past_days),
            calendar_future_days: Some(h.calendar_future_days),
            weather_retention_days: Some(h.weather_retention_days),
            private_events_as_busy: Some(h.private_events_as_busy),
            dashboard_completion_override: Some(h.dashboard_completion_override),
            now_list_size: Some(h.now_list_size),
            now_weights: Some(h.now_weights),
            quiet_from: Some(format_hhmm(h.quiet_from)),
            quiet_until: Some(format_hhmm(h.quiet_until)),
            currency: Some(h.currency),
            gender_terms: Some(crate::mail::GenderTermsConfig {
                female: h.gender_terms.female,
                male: h.gender_terms.male,
            }),
            disabled_signals: Some(h.disabled_signals),
            bill_lead_days: Some(h.bill_lead_days),
            events: Some(crate::mail::EventsConfig {
                auto_write: Some(h.events.auto_write),
                extract_threshold: Some(h.events.extract_threshold),
                route_threshold: Some(h.events.route_threshold),
            }),
            inbox: Some(crate::mail::InboxConfig {
                task_expiry_days: Some(h.inbox.task_expiry_days),
                contact_expiry_days: Some(h.inbox.contact_expiry_days),
            }),
        },
        members: members
            .into_iter()
            .map(|(key, m)| MemberConfig {
                notify: ex.notify.get(&m.id).cloned(),
                key,
                display_name: m.display_name,
                role: m.role,
                avatar: m.avatar,
                color: m.color,
                birthdate: m.birthdate,
                grade: m.grade,
                interests: m.interests,
                hobbies: m.hobbies,
                responsibilities: m.responsibilities,
                aliases: m.aliases,
                gender: m.gender,
                relevance_tags: m.relevance_tags,
                enrollments: mail.enrollments.remove(&m.id).unwrap_or_default(),
            })
            .collect(),
        calendar_sources,
        weather_rules,
        task_lists: ex.task_lists,
        task_definitions: ex.task_definitions,
        routines: ex.routines,
        retention: Some(crate::mail::RetentionConfig {
            mail: Some(crate::mail::MailRetentionConfig {
                content_policy: Some(h.retention_mail.content_policy),
                examples_per_category: Some(h.retention_mail.examples_per_category),
                max_pending_days: Some(h.retention_mail.max_pending_days),
            }),
        }),
        orgs: mail.orgs,
        mail_sources: mail.mail_sources,
        categories: mail.categories,
        mail_rules: mail.mail_rules,
    })
}
