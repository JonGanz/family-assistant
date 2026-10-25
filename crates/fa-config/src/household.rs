use std::collections::HashSet;

use chrono::{NaiveDate, Utc};
use fa_domain::Role;
use fa_domain::timefmt::{format_hhmm, parse_hhmm};
use fa_domain::{
    AuditEntry, AuditEntryId, Household, HouseholdId, Location, Member, MemberId, RuleAction,
    RuleStatus, Severity, SourceKind, Units,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        if errs.is_empty() {
            Ok(())
        } else {
            Err(HouseholdTomlError::Invalid(errs))
        }
    }
}

fn apply(m: &mut Member, c: &MemberConfig) -> bool {
    let before = (
        m.display_name.clone(),
        m.role,
        m.avatar.clone(),
        m.color.clone(),
        m.birthdate,
        m.grade.clone(),
        m.interests.clone(),
        m.hobbies.clone(),
        m.responsibilities.clone(),
    );
    m.display_name = c.display_name.clone();
    m.role = c.role;
    m.avatar = c.avatar.clone();
    m.color = c.color.clone();
    m.birthdate = c.birthdate;
    m.grade = c.grade.clone();
    m.interests = c.interests.clone();
    m.hobbies = c.hobbies.clone();
    m.responsibilities = c.responsibilities.clone();
    before
        != (
            m.display_name.clone(),
            m.role,
            m.avatar.clone(),
            m.color.clone(),
            m.birthdate,
            m.grade.clone(),
            m.interests.clone(),
            m.hobbies.clone(),
            m.responsibilities.clone(),
        )
}

/// Imports `cfg` into the store. Creates the household when missing (with `household_id` if
/// given, else a fresh id). Never touches accounts, PINs or secrets.
pub async fn import(
    store: &Store,
    household_id: Option<HouseholdId>,
    cfg: &HouseholdConfig,
    mode: ImportMode,
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
    };

    match HouseholdRepo::get(&mut tx, hid).await? {
        None => {
            let now = Utc::now();
            HouseholdRepo::insert(
                &mut tx,
                &Household {
                    id: hid,
                    name: cfg.household.name.clone(),
                    tz: cfg.household.tz.clone(),
                    location,
                    meal_plan_horizon_weeks: 2,
                    dashboard_completion_override: sec
                        .dashboard_completion_override
                        .unwrap_or(true),
                    units: sec.units.unwrap_or(DEFAULT_UNITS),
                    calendar_past_days: sec
                        .calendar_past_days
                        .unwrap_or(DEFAULT_CALENDAR_PAST_DAYS),
                    calendar_future_days: sec
                        .calendar_future_days
                        .unwrap_or(DEFAULT_CALENDAR_FUTURE_DAYS),
                    weather_retention_days: sec
                        .weather_retention_days
                        .unwrap_or(DEFAULT_WEATHER_RETENTION_DAYS),
                    private_events_as_busy: sec
                        .private_events_as_busy
                        .unwrap_or(DEFAULT_PRIVATE_EVENTS_AS_BUSY),
                    now_list_size: sec.now_list_size.unwrap_or(7),
                    now_weights: sec.now_weights.unwrap_or_default(),
                    quiet_from: quiet(sec.quiet_from.as_deref(), 21),
                    quiet_until: quiet(sec.quiet_until.as_deref(), 7),
                    created_at: now,
                    updated_at: now,
                },
            )
            .await?;
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
            let changed = h.now_list_size != before.now_list_size
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
                    color: c.color.clone(),
                    birthdate: c.birthdate,
                    grade: c.grade.clone(),
                    interests: c.interests.clone(),
                    hobbies: c.hobbies.clone(),
                    responsibilities: c.responsibilities.clone(),
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
    tx.commit().await?;
    tracing::info!(household = %hid, ?mode, "household config imported");
    Ok(report)
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
    let (calendar_sources, weather_rules) =
        crate::feeds::export_feeds(&mut tx, &id_to_key, &ex).await?;
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
            })
            .collect(),
        calendar_sources,
        weather_rules,
        task_lists: ex.task_lists,
        task_definitions: ex.task_definitions,
        routines: ex.routines,
    })
}
