//! Task lists, task definitions, routines and notification preferences in the household TOML
//! (arch/Tasks.md §8, plan/M2.md *Household TOML additions*).

use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use fa_domain::authz::ListCompletionPolicy;
use fa_domain::tasks::{default_on_missed, format_mmdd, list_defaults, parse_mmdd};
use fa_domain::timefmt::{format_hhmm, parse_hhmm};
use fa_domain::{
    AdvanceOn, CalendarDisplay, ChildCanAdd, DayFilter, HouseholdId, MemberId, NotifyTrigger,
    OnMissed, PriorityFlag, Role, Routine, RoutineId, Schedule, Strategy, TaskDefinition,
    TaskDefinitionId, TaskList, TaskListId, TaskListKind, TaskListPatch,
};
use fa_store::{
    CalendarRepo, CalendarSourceRepo, NotifyPref, NotifyPrefRepo, RoutineRepo, RoutineStepSpec,
    TaskDefinitionRepo, TaskListRepo, WeatherRuleRepo,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::household::{HouseholdConfig, ImportError, ImportReport};

pub const DEFAULT_DUE_SOON_MINUTES: i32 = 120;
pub const DEFAULT_NUDGE_MINUTES: i32 = 10;
pub const DEFAULT_TRIGGERS: [NotifyTrigger; 3] = [
    NotifyTrigger::Assigned,
    NotifyTrigger::Overdue,
    NotifyTrigger::Digest,
];

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskListConfig {
    pub name: String,
    /// Defaults to `general`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<TaskListKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_policy: Option<ListCompletionPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_can_add: Option<ChildCanAdd>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_add: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_default: Option<CalendarDisplay>,
    /// Omitted: a claim lasts until the end of the local day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_ttl_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_on_dashboard: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDefinitionConfig {
    /// Name of a `[[task_lists]]` entry.
    pub list: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    /// Repeat this many days after completion. Exactly one of `every_days` and `rrule`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_days: Option<i32>,
    /// RRULE body without DTSTART, e.g. `FREQ=WEEKLY;BYDAY=SA`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rrule: Option<String>,
    /// Defaults to the import day (household time zone) for new definitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    /// `"HH:MM"`; omitted means end of day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead_days: Option<i32>,
    /// Defaults by schedule kind (`roll_over` after done, `auto_skip` for rrule).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_missed: Option<OnMissed>,
    /// Defaults to `fixed` with one candidate, else `rotation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
    /// Member keys, in order.
    #[serde(default)]
    pub candidates: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_on: Option<AdvanceOn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_advances: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityFlag>,
    /// `"MM-DD"`; with `season_until`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_until: Option<String>,
    /// Weather-rule DSL; validated by `fa-server config import`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_condition: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineStepConfig {
    pub text: String,
    /// `"HH:MM"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineConfig {
    pub name: String,
    /// Member keys.
    pub members: Vec<String>,
    /// `"HH:MM"`.
    pub starts_at: String,
    /// `"HH:MM"`.
    pub done_by: String,
    /// Defaults to `weekdays`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_filter: Option<DayFilter>,
    /// `"source name/calendar name"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_calendar: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub no_school_match: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_on_dashboard: Option<bool>,
    #[serde(default)]
    pub steps: Vec<RoutineStepConfig>,
}

/// Per-member notification preferences. Topics are never part of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotifyConfig {
    /// Defaults to `assigned`, `overdue`, `digest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers: Option<Vec<NotifyTrigger>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_soon_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nudge_minutes: Option<i32>,
    /// `"HH:MM"`; both ends or neither (omitted: the household's quiet time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_until: Option<String>,
    /// Member key of an adult who gets a copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_to: Option<String>,
    /// How approval requests reach this member: `immediate`, `batched` (default), `digest`, `off`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_mode: Option<fa_domain::ApprovalMode>,
}

fn normalized_triggers(t: &Option<Vec<NotifyTrigger>>) -> Vec<NotifyTrigger> {
    let src: &[NotifyTrigger] = t.as_deref().unwrap_or(&DEFAULT_TRIGGERS);
    NotifyTrigger::ALL
        .iter()
        .copied()
        .filter(|x| src.contains(x))
        .collect()
}

pub(crate) fn validate_tasks(cfg: &HouseholdConfig, errs: &mut Vec<String>) {
    let h = &cfg.household;
    if let Some(n) = h.now_list_size
        && !(1..=50).contains(&n)
    {
        errs.push("household.now_list_size must be 1-50".to_string());
    }
    if let Some(w) = &h.now_weights
        && [w.due, w.flag, w.weather]
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
    {
        errs.push("household.now_weights must be finite and not negative".to_string());
    }
    for (name, v) in [
        ("household.quiet_from", &h.quiet_from),
        ("household.quiet_until", &h.quiet_until),
    ] {
        if let Some(s) = v
            && parse_hhmm(s).is_none()
        {
            errs.push(format!("{name} '{s}' must be HH:MM"));
        }
    }

    let members: HashMap<&str, Role> = cfg
        .members
        .iter()
        .map(|m| (m.key.as_str(), m.role))
        .collect();

    for m in &cfg.members {
        let Some(n) = &m.notify else { continue };
        let who = format!("member '{}': notify", m.key);
        if n.due_soon_minutes.is_some_and(|x| x < 0) || n.nudge_minutes.is_some_and(|x| x < 0) {
            errs.push(format!(
                "{who}: due_soon_minutes and nudge_minutes must not be negative"
            ));
        }
        match (&n.quiet_from, &n.quiet_until) {
            (Some(_), None) | (None, Some(_)) => errs.push(format!(
                "{who}: set both quiet_from and quiet_until or neither"
            )),
            _ => {}
        }
        for (f, v) in [
            ("quiet_from", &n.quiet_from),
            ("quiet_until", &n.quiet_until),
        ] {
            if let Some(s) = v
                && parse_hhmm(s).is_none()
            {
                errs.push(format!("{who}: {f} '{s}' must be HH:MM"));
            }
        }
        if let Some(c) = &n.copy_to {
            match members.get(c.as_str()) {
                None => errs.push(format!("{who}: unknown member key '{c}' in copy_to")),
                Some(Role::Child) => errs.push(format!("{who}: copy_to '{c}' must be an adult")),
                Some(Role::Adult) => {}
            }
        }
    }

    let mut lists = HashSet::new();
    for l in &cfg.task_lists {
        if l.name.trim().is_empty() {
            errs.push("task list name must not be empty".to_string());
        }
        if !lists.insert(l.name.as_str()) {
            errs.push(format!("duplicate task list name '{}'", l.name));
        }
        if l.claim_ttl_minutes.is_some_and(|x| x <= 0) {
            errs.push(format!(
                "task list '{}': claim_ttl_minutes must be positive",
                l.name
            ));
        }
    }

    let mut titles = HashSet::new();
    for d in &cfg.task_definitions {
        let who = format!("task definition '{}'", d.title);
        if d.title.trim().is_empty() {
            errs.push("task definition title must not be empty".to_string());
        }
        if !lists.contains(d.list.as_str()) {
            errs.push(format!("{who}: unknown task list '{}'", d.list));
        }
        if !titles.insert((d.list.as_str(), d.title.as_str())) {
            errs.push(format!("{who}: duplicate title in list '{}'", d.list));
        }
        match (d.every_days, d.rrule.as_deref()) {
            (Some(_), Some(_)) | (None, None) => {
                errs.push(format!("{who}: set exactly one of every_days and rrule"))
            }
            (Some(n), None) if n < 1 => errs.push(format!("{who}: every_days must be at least 1")),
            (None, Some(r)) if r.trim().is_empty() => {
                errs.push(format!("{who}: rrule must not be empty"))
            }
            _ => {}
        }
        if d.lead_days.is_some_and(|x| x < 0) {
            errs.push(format!("{who}: lead_days must not be negative"));
        }
        if let Some(t) = &d.due_time
            && parse_hhmm(t).is_none()
        {
            errs.push(format!("{who}: due_time '{t}' must be HH:MM"));
        }
        if d.steps.iter().any(|s| s.trim().is_empty()) {
            errs.push(format!("{who}: steps must not be empty strings"));
        }
        match (&d.season_from, &d.season_until) {
            (Some(_), None) | (None, Some(_)) => errs.push(format!(
                "{who}: set both season_from and season_until or neither"
            )),
            _ => {}
        }
        for (f, v) in [
            ("season_from", &d.season_from),
            ("season_until", &d.season_until),
        ] {
            if let Some(s) = v
                && parse_mmdd(s).is_err()
            {
                errs.push(format!("{who}: {f} '{s}' must be MM-DD"));
            }
        }
        if d.season_condition
            .as_deref()
            .is_some_and(|c| c.trim().is_empty())
        {
            errs.push(format!("{who}: season_condition must not be empty"));
        }
        let mut seen = HashSet::new();
        for c in &d.candidates {
            if !members.contains_key(c.as_str()) {
                errs.push(format!("{who}: unknown member key '{c}' in candidates"));
            }
            if !seen.insert(c.as_str()) {
                errs.push(format!("{who}: duplicate candidate '{c}'"));
            }
        }
        let strategy = d.strategy.unwrap_or(if d.candidates.len() == 1 {
            Strategy::Fixed
        } else {
            Strategy::Rotation
        });
        match strategy {
            Strategy::Fixed if d.candidates.len() != 1 => errs.push(format!(
                "{who}: a fixed strategy needs exactly one candidate"
            )),
            _ if d.candidates.is_empty() => {
                errs.push(format!("{who}: needs at least one candidate"))
            }
            _ => {}
        }
    }

    let mut names = HashSet::new();
    for r in &cfg.routines {
        let who = format!("routine '{}'", r.name);
        if r.name.trim().is_empty() {
            errs.push("routine name must not be empty".to_string());
        }
        if !names.insert(r.name.as_str()) {
            errs.push(format!("duplicate routine name '{}'", r.name));
        }
        for m in &r.members {
            if !members.contains_key(m.as_str()) {
                errs.push(format!("{who}: unknown member key '{m}'"));
            }
        }
        for (f, v) in [("starts_at", &r.starts_at), ("done_by", &r.done_by)] {
            if parse_hhmm(v).is_none() {
                errs.push(format!("{who}: {f} '{v}' must be HH:MM"));
            }
        }
        if let Some(c) = &r.school_calendar {
            match c.split_once('/') {
                Some((a, b)) if !a.trim().is_empty() && !b.trim().is_empty() => {}
                _ => errs.push(format!(
                    "{who}: school_calendar '{c}' must be \"source name/calendar name\""
                )),
            }
        }
        for s in &r.steps {
            if s.text.trim().is_empty() {
                errs.push(format!("{who}: step text must not be empty"));
            }
            if let Some(t) = &s.target_time
                && parse_hhmm(t).is_none()
            {
                errs.push(format!("{who}: step target_time '{t}' must be HH:MM"));
            }
        }
    }

    for w in &cfg.weather_rules {
        if w.task_definition
            .as_deref()
            .is_some_and(|t| t.trim().is_empty())
        {
            errs.push(format!(
                "weather rule '{}': task_definition must not be empty",
                w.name
            ));
        }
    }
}

fn bad(msg: String) -> ImportError {
    ImportError::Config(crate::household::HouseholdTomlError::Invalid(vec![msg]))
}

fn time(s: &str) -> NaiveTime {
    parse_hhmm(s).unwrap_or(NaiveTime::MIN)
}

/// Resolves `"source/calendar"` against the stored calendars, trying each `/` as the separator.
async fn find_calendar(
    conn: &mut PgConnection,
    spec: &str,
) -> Result<Option<fa_domain::CalendarId>, ImportError> {
    let sources = CalendarSourceRepo::list(conn, false).await?;
    let cals = CalendarRepo::list(conn, None).await?;
    for (i, _) in spec.match_indices('/') {
        let (sn, cn) = (&spec[..i], &spec[i + 1..]);
        for s in sources.iter().filter(|s| s.value.name == sn) {
            if let Some(c) = cals
                .iter()
                .find(|c| c.value.source == s.value.id && c.value.name == cn)
            {
                return Ok(Some(c.value.id));
            }
        }
    }
    Ok(None)
}

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

pub(crate) async fn import_tasks(
    conn: &mut PgConnection,
    hid: HouseholdId,
    tz: Tz,
    cfg: &HouseholdConfig,
    member_ids: &HashMap<&str, MemberId>,
    report: &mut ImportReport,
) -> Result<(), ImportError> {
    let resolve = |k: &str| member_ids.get(k).copied();

    // Notification preferences.
    for mc in &cfg.members {
        let (Some(n), Some(mid)) = (&mc.notify, resolve(&mc.key)) else {
            continue;
        };
        let existing = NotifyPrefRepo::get_for_member(conn, mid).await?;
        let triggers: Vec<String> = normalized_triggers(&n.triggers)
            .iter()
            .map(|t| t.as_str().to_string())
            .collect();
        let (qf, qu) = match (&n.quiet_from, &n.quiet_until) {
            (Some(f), Some(u)) => (Some(time(f)), Some(time(u))),
            _ => (None, None),
        };
        let mut pref = NotifyPref {
            id: Uuid::now_v7(),
            household_id: hid,
            member_id: mid,
            channel: "ntfy".into(),
            topic: new_topic(),
            triggers,
            due_soon_minutes: n.due_soon_minutes.unwrap_or(DEFAULT_DUE_SOON_MINUTES),
            nudge_minutes: n.nudge_minutes.unwrap_or(DEFAULT_NUDGE_MINUTES),
            quiet_from: qf,
            quiet_until: qu,
            copy_to_member_id: n.copy_to.as_deref().and_then(resolve),
            approval_mode: n
                .approval_mode
                .or(existing.as_ref().map(|e| e.value.approval_mode))
                .unwrap_or_default(),
            archived_at: None,
        };
        let changed = match &existing {
            Some(v) => {
                let e = &v.value;
                pref.id = e.id;
                pref.topic = e.topic.clone();
                pref.channel = e.channel.clone();
                let mut have = e.triggers.clone();
                have.sort();
                let mut want = pref.triggers.clone();
                want.sort();
                have != want
                    || e.due_soon_minutes != pref.due_soon_minutes
                    || e.nudge_minutes != pref.nudge_minutes
                    || e.quiet_from != pref.quiet_from
                    || e.quiet_until != pref.quiet_until
                    || e.copy_to_member_id != pref.copy_to_member_id
                    || e.approval_mode != pref.approval_mode
            }
            None => true,
        };
        if changed {
            NotifyPrefRepo::upsert_for_member(conn, &pref, None).await?;
        }
        pick(
            existing.is_none(),
            changed,
            &mut report.notify_created,
            &mut report.notify_updated,
            &mut report.notify_unchanged,
        )
        .push(mc.key.clone());
    }

    // Lists.
    let mut list_ids: HashMap<&str, TaskListId> = HashMap::new();
    for (i, lc) in cfg.task_lists.iter().enumerate() {
        let kind = lc.kind.unwrap_or(TaskListKind::General);
        let d = list_defaults(kind);
        let completion_policy = lc.completion_policy.unwrap_or(d.completion_policy);
        let child_can_add = lc.child_can_add.unwrap_or(d.child_can_add);
        let dashboard_add = lc.dashboard_add.unwrap_or(d.dashboard_add);
        let calendar_default = lc.calendar_default.unwrap_or(d.calendar_default);
        let visible = lc.visible_on_dashboard.unwrap_or(true);
        let sort = i as i32;
        match TaskListRepo::get_by_name(conn, &lc.name).await? {
            Some(v) => {
                let e = &v.value;
                let changed = e.kind != kind
                    || e.completion_policy != completion_policy
                    || e.child_can_add != child_can_add
                    || e.dashboard_add != dashboard_add
                    || e.calendar_default != calendar_default
                    || e.claim_ttl_minutes != lc.claim_ttl_minutes
                    || e.visible_on_dashboard != visible
                    || e.color != lc.color
                    || e.sort != sort;
                if changed {
                    let patch = TaskListPatch {
                        name: None,
                        kind: Some(kind),
                        completion_policy: Some(completion_policy),
                        child_can_add: Some(child_can_add),
                        dashboard_add: Some(dashboard_add),
                        calendar_default: Some(calendar_default),
                        claim_ttl_minutes: Some(lc.claim_ttl_minutes),
                        visible_on_dashboard: Some(visible),
                        color: Some(lc.color.clone()),
                        sort: Some(sort),
                    };
                    TaskListRepo::update(conn, e.id, &patch, v.version).await?;
                    report.task_lists_updated.push(lc.name.clone());
                } else {
                    report.task_lists_unchanged.push(lc.name.clone());
                }
                list_ids.insert(lc.name.as_str(), e.id);
            }
            None => {
                let now = Utc::now();
                let id = TaskListId::new();
                TaskListRepo::insert(
                    conn,
                    &TaskList {
                        id,
                        household_id: hid,
                        name: lc.name.clone(),
                        kind,
                        completion_policy,
                        child_can_add,
                        dashboard_add,
                        calendar_default,
                        claim_ttl_minutes: lc.claim_ttl_minutes,
                        visible_on_dashboard: visible,
                        color: lc.color.clone(),
                        sort,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                )
                .await?;
                list_ids.insert(lc.name.as_str(), id);
                report.task_lists_created.push(lc.name.clone());
            }
        }
    }

    // Definitions.
    for dc in &cfg.task_definitions {
        let list = *list_ids
            .get(dc.list.as_str())
            .ok_or_else(|| bad(format!("task list '{}' not found", dc.list)))?;
        let schedule = match (dc.every_days, &dc.rrule) {
            (Some(n), _) => Schedule::AfterDone { interval_days: n },
            (None, Some(r)) => Schedule::Rrule { rrule: r.clone() },
            (None, None) => return Err(bad(format!("{}: no schedule", dc.title))),
        };
        let candidates: Vec<MemberId> = dc.candidates.iter().filter_map(|k| resolve(k)).collect();
        let strategy = dc.strategy.unwrap_or(if candidates.len() == 1 {
            Strategy::Fixed
        } else {
            Strategy::Rotation
        });
        let on_missed = dc
            .on_missed
            .unwrap_or_else(|| default_on_missed(schedule.kind()));
        let due_time = dc.due_time.as_deref().map(time);
        let lead_days = dc.lead_days.unwrap_or(0);
        let advance_on = dc.advance_on.unwrap_or_default();
        let skip_advances = dc.skip_advances.unwrap_or(true);
        let priority = dc.priority.unwrap_or_default();
        let season = match (&dc.season_from, &dc.season_until) {
            (Some(f), Some(u)) => match (parse_mmdd(f), parse_mmdd(u)) {
                (Ok(f), Ok(u)) => Some((f, u)),
                _ => None,
            },
            _ => None,
        };
        match TaskDefinitionRepo::get_by_title(conn, list, &dc.title).await? {
            Some(v) => {
                let e = &v.value;
                let start_date = dc.start_date.unwrap_or(e.start_date);
                let schedule_changed = e.schedule != schedule
                    || e.start_date != start_date
                    || e.due_time != due_time
                    || e.lead_days != lead_days
                    || e.season != season;
                let have = TaskDefinitionRepo::candidates(conn, e.id).await?;
                let have: Vec<MemberId> = have.into_iter().map(|(m, _)| m).collect();
                let cands_changed = have != candidates;
                let changed = schedule_changed
                    || cands_changed
                    || e.notes != dc.notes
                    || e.steps_template != dc.steps
                    || e.on_missed != on_missed
                    || e.strategy != strategy
                    || e.advance_on != advance_on
                    || e.skip_advances != skip_advances
                    || e.priority_flag != priority
                    || e.season_condition != dc.season_condition
                    || e.paused != dc.paused;
                if changed {
                    let mut n = e.clone();
                    n.notes = dc.notes.clone();
                    n.steps_template = dc.steps.clone();
                    n.schedule = schedule;
                    n.start_date = start_date;
                    n.due_time = due_time;
                    n.lead_days = lead_days;
                    n.on_missed = on_missed;
                    n.strategy = strategy;
                    n.advance_on = advance_on;
                    n.skip_advances = skip_advances;
                    n.priority_flag = priority;
                    n.season = season;
                    n.season_condition = dc.season_condition.clone();
                    n.paused = dc.paused;
                    if schedule_changed {
                        n.next_due_at = None;
                    }
                    TaskDefinitionRepo::update(
                        conn,
                        &n,
                        cands_changed.then_some(candidates.as_slice()),
                        v.version,
                    )
                    .await?;
                    report.task_definitions_updated.push(dc.title.clone());
                } else {
                    report.task_definitions_unchanged.push(dc.title.clone());
                }
            }
            None => {
                let now = Utc::now();
                TaskDefinitionRepo::insert(
                    conn,
                    &TaskDefinition {
                        id: TaskDefinitionId::new(),
                        household_id: hid,
                        list_id: list,
                        title: dc.title.clone(),
                        notes: dc.notes.clone(),
                        steps_template: dc.steps.clone(),
                        schedule,
                        start_date: dc
                            .start_date
                            .unwrap_or_else(|| now.with_timezone(&tz).date_naive()),
                        due_time,
                        lead_days,
                        on_missed,
                        strategy,
                        advance_on,
                        skip_advances,
                        rotation_cursor: 0,
                        priority_flag: priority,
                        season,
                        season_condition: dc.season_condition.clone(),
                        condition_ok: None,
                        condition_checked_at: None,
                        last_done_at: None,
                        next_due_at: None,
                        paused: dc.paused,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                    &candidates,
                )
                .await?;
                report.task_definitions_created.push(dc.title.clone());
            }
        }
    }

    // Routines.
    for rc in &cfg.routines {
        let members: Vec<MemberId> = rc.members.iter().filter_map(|k| resolve(k)).collect();
        let steps: Vec<RoutineStepSpec> = rc
            .steps
            .iter()
            .map(|s| RoutineStepSpec {
                text: s.text.clone(),
                target_time: s.target_time.as_deref().map(time),
            })
            .collect();
        let school = match &rc.school_calendar {
            Some(spec) => Some(find_calendar(conn, spec).await?.ok_or_else(|| {
                bad(format!(
                    "routine '{}': school calendar '{spec}' not found",
                    rc.name
                ))
            })?),
            None => None,
        };
        let day_filter = rc.day_filter.unwrap_or(DayFilter::Weekdays);
        let show = rc.show_on_dashboard.unwrap_or(true);
        let (starts_at, done_by) = (time(&rc.starts_at), time(&rc.done_by));
        match RoutineRepo::get_by_name(conn, &rc.name).await? {
            Some(full) => {
                let e = &full.routine.value;
                let have_steps: Vec<RoutineStepSpec> = full
                    .steps
                    .iter()
                    .map(|s| RoutineStepSpec {
                        text: s.text.clone(),
                        target_time: s.target_time,
                    })
                    .collect();
                let steps_changed = have_steps != steps;
                let members_changed = !same_set(&full.members, &members);
                let changed = steps_changed
                    || members_changed
                    || e.starts_at != starts_at
                    || e.done_by != done_by
                    || e.day_filter != day_filter
                    || e.school_calendar_id != school
                    || e.no_school_match != rc.no_school_match
                    || e.show_on_dashboard != show;
                if changed {
                    let mut n = e.clone();
                    n.starts_at = starts_at;
                    n.done_by = done_by;
                    n.day_filter = day_filter;
                    n.school_calendar_id = school;
                    n.no_school_match = rc.no_school_match.clone();
                    n.show_on_dashboard = show;
                    RoutineRepo::update(
                        conn,
                        &n,
                        members_changed.then_some(members.as_slice()),
                        steps_changed.then_some(steps.as_slice()),
                        full.routine.version,
                    )
                    .await?;
                    report.routines_updated.push(rc.name.clone());
                } else {
                    report.routines_unchanged.push(rc.name.clone());
                }
            }
            None => {
                let now = Utc::now();
                RoutineRepo::insert(
                    conn,
                    &Routine {
                        id: RoutineId::new(),
                        household_id: hid,
                        name: rc.name.clone(),
                        starts_at,
                        done_by,
                        day_filter,
                        school_calendar_id: school,
                        no_school_match: rc.no_school_match.clone(),
                        show_on_dashboard: show,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                    &members,
                    &steps,
                )
                .await?;
                report.routines_created.push(rc.name.clone());
            }
        }
    }
    Ok(())
}

fn same_set(a: &[MemberId], b: &[MemberId]) -> bool {
    let a: HashSet<MemberId> = a.iter().copied().collect();
    let b: HashSet<MemberId> = b.iter().copied().collect();
    a == b
}

/// A fresh, unguessable ntfy topic (never exported).
fn new_topic() -> String {
    let (a, b) = (
        Uuid::now_v7().simple().to_string(),
        Uuid::now_v7().simple().to_string(),
    );
    format!("fa-{}{}", &a[16..], &b[16..])
}

pub(crate) struct ExportedTasks {
    pub notify: HashMap<MemberId, NotifyConfig>,
    pub task_lists: Vec<TaskListConfig>,
    pub task_definitions: Vec<TaskDefinitionConfig>,
    pub routines: Vec<RoutineConfig>,
    /// Definition id to (list name, title), for weather rules.
    pub definition_names: HashMap<TaskDefinitionId, (String, String)>,
}

pub(crate) async fn export_tasks(
    conn: &mut PgConnection,
    id_to_key: &HashMap<MemberId, String>,
) -> Result<ExportedTasks, ImportError> {
    let key_of = |m: MemberId| id_to_key.get(&m).cloned();

    let mut notify = HashMap::new();
    for &mid in id_to_key.keys() {
        if let Some(v) = NotifyPrefRepo::get_for_member(conn, mid).await? {
            let p = v.value;
            let triggers: Vec<NotifyTrigger> =
                p.triggers.iter().filter_map(|t| t.parse().ok()).collect();
            notify.insert(
                mid,
                NotifyConfig {
                    triggers: Some(normalized_triggers(&Some(triggers))),
                    due_soon_minutes: Some(p.due_soon_minutes),
                    nudge_minutes: Some(p.nudge_minutes),
                    quiet_from: p.quiet_from.map(format_hhmm),
                    quiet_until: p.quiet_until.map(format_hhmm),
                    copy_to: p.copy_to_member_id.and_then(key_of),
                    approval_mode: Some(p.approval_mode),
                },
            );
        }
    }

    let mut list_names = HashMap::new();
    let mut task_lists = Vec::new();
    for v in TaskListRepo::list(conn, false).await? {
        let l = v.value;
        list_names.insert(l.id, l.name.clone());
        task_lists.push(TaskListConfig {
            name: l.name,
            kind: Some(l.kind),
            completion_policy: Some(l.completion_policy),
            child_can_add: Some(l.child_can_add),
            dashboard_add: Some(l.dashboard_add),
            calendar_default: Some(l.calendar_default),
            claim_ttl_minutes: l.claim_ttl_minutes,
            visible_on_dashboard: Some(l.visible_on_dashboard),
            color: l.color,
        });
    }

    let mut definition_names = HashMap::new();
    let mut task_definitions = Vec::new();
    for v in TaskDefinitionRepo::list(conn, None, false).await? {
        let d = v.value;
        let Some(list) = list_names.get(&d.list_id) else {
            continue;
        };
        definition_names.insert(d.id, (list.clone(), d.title.clone()));
        let candidates: Vec<String> = TaskDefinitionRepo::candidates(conn, d.id)
            .await?
            .into_iter()
            .filter_map(|(m, _)| key_of(m))
            .collect();
        if candidates.is_empty() {
            tracing::warn!(definition = %d.title, "task definition has no active candidates, not exported");
            continue;
        }
        let (every_days, rrule) = match d.schedule {
            Schedule::AfterDone { interval_days } => (Some(interval_days), None),
            Schedule::Rrule { rrule } => (None, Some(rrule)),
        };
        task_definitions.push(TaskDefinitionConfig {
            list: list.clone(),
            title: d.title,
            notes: d.notes,
            steps: d.steps_template,
            every_days,
            rrule,
            start_date: Some(d.start_date),
            due_time: d.due_time.map(format_hhmm),
            lead_days: Some(d.lead_days),
            on_missed: Some(d.on_missed),
            strategy: Some(d.strategy),
            candidates,
            advance_on: Some(d.advance_on),
            skip_advances: Some(d.skip_advances),
            priority: Some(d.priority_flag),
            season_from: d.season.map(|(f, _)| format_mmdd(f)),
            season_until: d.season.map(|(_, u)| format_mmdd(u)),
            season_condition: d.season_condition,
            paused: d.paused,
        });
    }

    let mut routines = Vec::new();
    for full in RoutineRepo::list(conn, false).await? {
        let r = full.routine.value;
        let school_calendar = match r.school_calendar_id {
            Some(cid) => match CalendarRepo::get(conn, cid).await? {
                Some(c) => CalendarSourceRepo::get(conn, c.value.source)
                    .await?
                    .map(|s| format!("{}/{}", s.value.name, c.value.name)),
                None => None,
            },
            None => None,
        };
        let mut members: Vec<String> = full.members.iter().filter_map(|m| key_of(*m)).collect();
        members.sort();
        routines.push(RoutineConfig {
            name: r.name,
            members,
            starts_at: format_hhmm(r.starts_at),
            done_by: format_hhmm(r.done_by),
            day_filter: Some(r.day_filter),
            school_calendar,
            no_school_match: r.no_school_match,
            show_on_dashboard: Some(r.show_on_dashboard),
            steps: full
                .steps
                .into_iter()
                .map(|s| RoutineStepConfig {
                    text: s.text,
                    target_time: s.target_time.map(format_hhmm),
                })
                .collect(),
        });
    }

    Ok(ExportedTasks {
        notify,
        task_lists,
        task_definitions,
        routines,
        definition_names,
    })
}

/// Resolves a weather rule's `task_definition` (`"Title"` or `"List name/Title"`) to an id.
async fn resolve_definition(
    conn: &mut PgConnection,
    spec: &str,
) -> Result<TaskDefinitionId, ImportError> {
    let lists: HashMap<TaskListId, String> = TaskListRepo::list(conn, false)
        .await?
        .into_iter()
        .map(|v| (v.value.id, v.value.name))
        .collect();
    let defs = TaskDefinitionRepo::list(conn, None, false).await?;
    let qualified = |d: &TaskDefinition| {
        format!(
            "{}/{}",
            lists.get(&d.list_id).map(String::as_str).unwrap_or(""),
            d.title
        )
    };
    let by_title: Vec<&TaskDefinition> = defs
        .iter()
        .map(|v| &v.value)
        .filter(|d| d.title == spec)
        .collect();
    match by_title.as_slice() {
        [d] => return Ok(d.id),
        [_, _, ..] => {
            return Err(bad(format!(
                "task definition '{spec}' is ambiguous; use \"List name/Title\""
            )));
        }
        [] => {}
    }
    defs.iter()
        .map(|v| &v.value)
        .find(|d| qualified(d) == spec)
        .map(|d| d.id)
        .ok_or_else(|| bad(format!("task definition '{spec}' not found")))
}

/// Sets `task_mode` and `task_definition` of the file's weather rules; run after the
/// definitions and rules are imported.
pub(crate) async fn import_rule_tasks(
    conn: &mut PgConnection,
    cfg: &HouseholdConfig,
    report: &mut ImportReport,
) -> Result<(), ImportError> {
    for rc in &cfg.weather_rules {
        let mode = rc.task_mode.unwrap_or(fa_domain::TaskMode::Create);
        let def = match &rc.task_definition {
            Some(t) => Some(serde_json::json!({
                "definition": resolve_definition(conn, t).await?.to_string()
            })),
            None => None,
        };
        let rules = WeatherRuleRepo::list(conn, None).await?;
        let Some(v) = rules.iter().find(|v| v.value.name == rc.name) else {
            continue;
        };
        if v.value.task_mode == mode && v.value.task_definition == def {
            continue;
        }
        let mut n = v.value.clone();
        n.task_mode = mode;
        n.task_definition = def;
        WeatherRuleRepo::update(conn, &n, v.version).await?;
        if !report.rules_created.contains(&rc.name) && !report.rules_updated.contains(&rc.name) {
            report.rules_unchanged.retain(|x| x != &rc.name);
            report.rules_updated.push(rc.name.clone());
        }
    }
    Ok(())
}

/// The TOML spelling of a rule's stored `task_definition`: the title, or `"List/Title"` when the
/// title exists in more than one list.
pub(crate) fn definition_spec(value: &serde_json::Value, ex: &ExportedTasks) -> Option<String> {
    let id: TaskDefinitionId = value
        .get("definition")?
        .as_str()?
        .parse::<Uuid>()
        .ok()?
        .into();
    let (list, title) = ex.definition_names.get(&id)?;
    let dup = ex
        .definition_names
        .values()
        .filter(|(_, t)| t == title)
        .count()
        > 1;
    Some(if dup {
        format!("{list}/{title}")
    } else {
        title.clone()
    })
}
