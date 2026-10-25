//! Calendar sources and weather rules in the household TOML (arch/Feeds.md §5).

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use fa_domain::api::CalendarPatch;
use fa_domain::{
    CalendarSource, CalendarSourceId, HouseholdId, MemberId, SourceKind, SourceState, SuggestedBy,
    WeatherRule, WeatherRuleId,
};
use fa_store::{
    CalendarRepo, CalendarSourceRepo, ConfigExtRepo, MemberRepo, StoreError, WeatherRuleRepo,
};
use sqlx::PgConnection;

use crate::household::{
    CalendarConfig, CalendarSourceConfig, HouseholdConfig, ImportError, ImportReport,
    WeatherRuleConfig,
};

const DEFAULT_POLL_ICS: i32 = 60;
const DEFAULT_POLL: i32 = 15;

fn default_poll(kind: SourceKind) -> i32 {
    if kind == SourceKind::Ics {
        DEFAULT_POLL_ICS
    } else {
        DEFAULT_POLL
    }
}

fn is_slug(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 64
        && k.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// 1-based line of the first line mentioning `"needle"` (quoted) at or after `after_line`,
/// ignoring the member definitions themselves.
fn find_line(src: &str, needle: &str, after_line: usize) -> Option<usize> {
    let quoted = format!("\"{needle}\"");
    let hit = |skip: usize| {
        src.lines().enumerate().skip(skip).find_map(|(i, l)| {
            let t = l.trim_start();
            (l.contains(&quoted)
                && !t.starts_with("key")
                && !t.starts_with("display_name")
                && !t.starts_with('#'))
            .then_some(i + 1)
        })
    };
    hit(after_line.saturating_sub(1)).or_else(|| hit(0))
}

pub(crate) fn validate_feeds(cfg: &HouseholdConfig, src: Option<&str>, errs: &mut Vec<String>) {
    let member_keys: HashSet<&str> = cfg.members.iter().map(|m| m.key.as_str()).collect();
    let at = |needle: &str, after: usize| -> String {
        src.and_then(|s| find_line(s, needle, after))
            .map(|l| format!(" (line {l})"))
            .unwrap_or_default()
    };
    let mut keys = HashSet::new();
    let mut ident = HashSet::new();
    for s in &cfg.calendar_sources {
        if !is_slug(&s.key) {
            errs.push(format!(
                "calendar source key '{}' must be a slug of a-z, 0-9, '-' or '_' (max 64)",
                s.key
            ));
        }
        if !keys.insert(s.key.as_str()) {
            errs.push(format!("duplicate calendar source key '{}'", s.key));
        }
        if !ident.insert((s.kind, s.name.as_str())) {
            errs.push(format!(
                "calendar source '{}': another source has the same kind and name",
                s.key
            ));
        }
        if s.name.trim().is_empty() {
            errs.push(format!(
                "calendar source '{}': name must not be empty",
                s.key
            ));
        }
        if let Some(p) = s.poll_minutes
            && !(1..=10080).contains(&p)
        {
            errs.push(format!(
                "calendar source '{}': poll_minutes must be 1-10080",
                s.key
            ));
        }
        let start = src
            .and_then(|t| {
                t.lines()
                    .position(|l| l.trim() == format!("key = \"{}\"", s.key))
            })
            .map(|i| i + 1)
            .unwrap_or(0);
        if let Some(o) = &s.owner
            && !member_keys.contains(o.as_str())
        {
            errs.push(format!(
                "calendar source '{}': unknown member key '{o}' in owner{}",
                s.key,
                at(o, start)
            ));
        }
        let mut rids = HashSet::new();
        for c in &s.calendars {
            if c.remote_id.trim().is_empty() || !rids.insert(c.remote_id.as_str()) {
                errs.push(format!(
                    "calendar source '{}': remote_id '{}' is empty or duplicated",
                    s.key, c.remote_id
                ));
            }
            let cal_start = src
                .and_then(|t| {
                    t.lines()
                        .enumerate()
                        .skip(start)
                        .find(|(_, l)| l.contains(&format!("\"{}\"", c.remote_id)))
                        .map(|(i, _)| i + 1)
                })
                .unwrap_or(start);
            for m in &c.members {
                if !member_keys.contains(m.as_str()) {
                    errs.push(format!(
                        "calendar source '{}', calendar '{}': unknown member key '{m}'{}",
                        s.key,
                        c.remote_id,
                        at(m, cal_start)
                    ));
                }
            }
        }
    }
    let mut names = HashSet::new();
    for r in &cfg.weather_rules {
        if r.name.trim().is_empty() || r.condition.trim().is_empty() || r.message.trim().is_empty()
        {
            errs.push(format!(
                "weather rule '{}': name, condition and message must not be empty",
                r.name
            ));
        }
        if !names.insert(r.name.as_str()) {
            errs.push(format!("duplicate weather rule name '{}'", r.name));
        }
    }
}

/// Adds a "needs reconnecting" comment above each `[[calendar_sources]]` header.
pub(crate) fn annotate_sources(toml: &str, sources: &[CalendarSourceConfig]) -> String {
    let mut out = String::with_capacity(toml.len() + 256);
    let mut i = 0;
    for line in toml.lines() {
        if line == "[[calendar_sources]]"
            && let Some(s) = sources.get(i)
        {
            i += 1;
            let note = match s.kind {
                SourceKind::Google => "Google account is not exported: reconnect it in Settings.",
                SourceKind::Caldav => {
                    "CalDAV password is not exported: enter it again in Settings."
                }
                SourceKind::Ics => "ICS URL is not exported: paste it again in Settings.",
            };
            out.push_str("# ");
            out.push_str(note);
            out.push('\n');
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn same_members(a: &[MemberId], b: &[MemberId]) -> bool {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_by_key(|m| m.as_uuid());
    b.sort_by_key(|m| m.as_uuid());
    a == b
}

pub(crate) async fn import_feeds(
    conn: &mut PgConnection,
    hid: HouseholdId,
    cfg: &HouseholdConfig,
    report: &mut ImportReport,
) -> Result<(), ImportError> {
    let mut member_ids: HashMap<&str, MemberId> = HashMap::new();
    for m in &cfg.members {
        let v = MemberRepo::get_by_config_key(conn, hid, &m.key)
            .await?
            .ok_or(StoreError::NotFound)?;
        member_ids.insert(m.key.as_str(), v.value.id);
    }
    let resolve = |k: &str| member_ids.get(k).copied();

    let existing = CalendarSourceRepo::list(conn, false).await?;
    for sc in &cfg.calendar_sources {
        let found = existing
            .iter()
            .find(|v| v.value.kind == sc.kind && v.value.name == sc.name);
        let owner = sc.owner.as_deref().and_then(resolve);
        let (id, has_creds, changed) = match found {
            Some(v) => {
                let s = &v.value;
                let has_creds = s.secret_id.is_some() || s.oauth_account.is_some();
                let (url, username) = if sc.kind == SourceKind::Caldav {
                    (
                        sc.url.clone().or_else(|| s.url.clone()),
                        sc.username.clone().or_else(|| s.username.clone()),
                    )
                } else {
                    (s.url.clone(), s.username.clone())
                };
                let poll = sc.poll_minutes.unwrap_or(s.poll_minutes);
                let mut changed = false;
                if s.owner_member != owner
                    || s.url != url
                    || s.username != username
                    || s.poll_minutes != poll
                {
                    ConfigExtRepo::set_source_config(
                        conn,
                        s.id,
                        owner,
                        url.as_deref(),
                        username.as_deref(),
                        poll,
                    )
                    .await?;
                    changed = true;
                }
                let was_enabled = ConfigExtRepo::source_enabled(conn, s.id).await?;
                let enabled = has_creds && sc.enabled.unwrap_or(was_enabled);
                if enabled != was_enabled {
                    CalendarSourceRepo::set_enabled(conn, s.id, enabled).await?;
                    changed = true;
                }
                if !has_creds && s.state != SourceState::NeedsReauth {
                    CalendarSourceRepo::set_sync_result(
                        conn,
                        s.id,
                        SourceState::NeedsReauth,
                        None,
                        None,
                    )
                    .await?;
                    changed = true;
                }
                (s.id, has_creds, changed)
            }
            None => {
                let now = Utc::now();
                let id = CalendarSourceId::new();
                CalendarSourceRepo::insert(
                    conn,
                    &CalendarSource {
                        id,
                        household_id: hid,
                        kind: sc.kind,
                        name: sc.name.clone(),
                        owner_member: owner,
                        oauth_account: None,
                        url: (sc.kind == SourceKind::Caldav)
                            .then(|| sc.url.clone())
                            .flatten(),
                        username: (sc.kind == SourceKind::Caldav)
                            .then(|| sc.username.clone())
                            .flatten(),
                        secret_id: None,
                        writable: false,
                        poll_minutes: sc.poll_minutes.unwrap_or_else(|| default_poll(sc.kind)),
                        state: SourceState::NeedsReauth,
                        last_synced_at: None,
                        last_error: None,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                )
                .await?;
                CalendarSourceRepo::set_enabled(conn, id, false).await?;
                (id, false, true)
            }
        };
        let mut changed = changed;
        let was_new = found.is_none();
        changed |= import_calendars(conn, id, has_creds, &sc.calendars, &resolve).await?;
        let list = if was_new {
            &mut report.sources_created
        } else if changed {
            &mut report.sources_updated
        } else {
            &mut report.sources_unchanged
        };
        list.push(sc.key.clone());
    }

    let rules = WeatherRuleRepo::list(conn, None).await?;
    for rc in &cfg.weather_rules {
        match rules.iter().find(|v| v.value.name == rc.name) {
            Some(v) => {
                let r = &v.value;
                if r.condition == rc.condition
                    && r.action == rc.action
                    && r.severity == rc.severity
                    && r.message == rc.message
                    && r.status == rc.status
                {
                    report.rules_unchanged.push(rc.name.clone());
                } else {
                    let mut n = r.clone();
                    n.condition = rc.condition.clone();
                    n.action = rc.action;
                    n.severity = rc.severity;
                    n.message = rc.message.clone();
                    n.status = rc.status;
                    WeatherRuleRepo::update(conn, &n, v.version).await?;
                    report.rules_updated.push(rc.name.clone());
                }
            }
            None => {
                let now = Utc::now();
                WeatherRuleRepo::insert(
                    conn,
                    &WeatherRule {
                        id: WeatherRuleId::new(),
                        household_id: hid,
                        name: rc.name.clone(),
                        condition: rc.condition.clone(),
                        action: rc.action,
                        severity: rc.severity,
                        message: rc.message.clone(),
                        task_definition: None,
                        status: rc.status,
                        suggested_by: SuggestedBy::User,
                        task_mode: fa_domain::TaskMode::Create,
                        archived_at: None,
                        created_at: now,
                        updated_at: now,
                    },
                )
                .await?;
                report.rules_created.push(rc.name.clone());
            }
        }
    }
    Ok(())
}

/// Returns whether anything changed. Calendars come in disabled unless the source has credentials.
async fn import_calendars(
    conn: &mut PgConnection,
    source: CalendarSourceId,
    has_creds: bool,
    calendars: &[CalendarConfig],
    resolve: &impl Fn(&str) -> Option<MemberId>,
) -> Result<bool, ImportError> {
    let mut changed = false;
    for c in calendars {
        let name = c.name.clone().unwrap_or_else(|| c.remote_id.clone());
        let (v, created) =
            CalendarRepo::upsert_discovered(conn, source, &c.remote_id, &name, c.color.as_deref())
                .await?;
        changed |= created;
        let enabled = has_creds && c.enabled;
        let cal = &v.value;
        let patch = CalendarPatch {
            name: (cal.name != name).then(|| name.clone()),
            color: c
                .color
                .clone()
                .filter(|col| cal.color.as_ref() != Some(col)),
            enabled: (cal.enabled != enabled).then_some(enabled),
            members: None,
        };
        if patch.name.is_some() || patch.color.is_some() || patch.enabled.is_some() {
            CalendarRepo::update(conn, cal.id, &patch, v.version).await?;
            changed = true;
        }
        let want: Vec<MemberId> = c.members.iter().filter_map(|k| resolve(k)).collect();
        let have = CalendarRepo::members(conn, cal.id).await?;
        if !same_members(&want, &have) {
            CalendarRepo::set_members(conn, cal.id, &want).await?;
            changed = true;
        }
    }
    Ok(changed)
}

fn slug(s: &str) -> String {
    let t: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let t = t.trim_matches('-').to_string();
    if t.is_empty() { "source".into() } else { t }
}

pub(crate) async fn export_feeds(
    conn: &mut PgConnection,
    id_to_key: &HashMap<MemberId, String>,
    ex: &crate::tasks::ExportedTasks,
) -> Result<(Vec<CalendarSourceConfig>, Vec<WeatherRuleConfig>), ImportError> {
    let key_of = |m: MemberId| id_to_key.get(&m).cloned();
    let mut used = HashSet::new();
    let mut sources = Vec::new();
    for v in CalendarSourceRepo::list(conn, false).await? {
        let s = v.value;
        let base = slug(&s.name);
        let mut key = base.clone();
        let mut n = 2;
        while !used.insert(key.clone()) {
            key = format!("{base}-{n}");
            n += 1;
        }
        let enabled = ConfigExtRepo::source_enabled(conn, s.id).await?;
        let mut calendars = Vec::new();
        for cv in CalendarRepo::list(conn, Some(s.id)).await? {
            let c = cv.value;
            let mut members: Vec<String> = CalendarRepo::members(conn, c.id)
                .await?
                .into_iter()
                .filter_map(key_of)
                .collect();
            members.sort();
            calendars.push(CalendarConfig {
                remote_id: c.remote_id,
                name: Some(c.name),
                enabled: c.enabled,
                color: c.color,
                members,
            });
        }
        let caldav = s.kind == SourceKind::Caldav;
        sources.push(CalendarSourceConfig {
            key,
            kind: s.kind,
            name: s.name,
            owner: s.owner_member.and_then(key_of),
            url: if caldav { s.url } else { None },
            username: if caldav { s.username } else { None },
            poll_minutes: Some(s.poll_minutes),
            enabled: Some(enabled),
            calendars,
        });
    }

    // Untouched default rules are re-seeded by `fa_weather::ensure_default_rules`.
    let rules = WeatherRuleRepo::list(conn, None)
        .await?
        .into_iter()
        .filter(|v| !(v.value.suggested_by == SuggestedBy::Default && v.version == 1))
        .map(|v| {
            let r = v.value;
            WeatherRuleConfig {
                name: r.name,
                condition: r.condition,
                action: r.action,
                severity: r.severity,
                message: r.message,
                status: r.status,
                task_mode: (r.task_mode != fa_domain::TaskMode::Create).then_some(r.task_mode),
                task_definition: r
                    .task_definition
                    .as_ref()
                    .and_then(|v| crate::tasks::definition_spec(v, ex)),
            }
        })
        .collect();
    Ok((sources, rules))
}
