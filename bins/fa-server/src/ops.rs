//! Operator commands that talk to the database directly (no running server needed).

use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use fa_auth::{AccountService, DeviceService};
use fa_config::{HouseholdConfig, ImportMode, ProcessConfig};
use fa_domain::calendar::{HouseholdFlags, Viewer};
use fa_domain::{
    Actor, AuditEntry, AuditEntryId, CalendarSourceId, HouseholdId, Member, Role, validate_pin,
};
use fa_store::{
    AdvisoryRepo, AuditRepo, CalendarSourceRepo, DeviceElevationRepo, DeviceRepo, HouseholdRepo,
    MemberRepo, Store, Tx, UserAccountRepo,
};

use crate::cli::{
    CalendarAction, ConfigAction, DeviceAction, MailAction, NotifyAction, PinAction, TasksAction,
    UserAction, WeatherAction,
};
use crate::wiring;

pub fn gen_openapi(out: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(&fa_api::openapi())?;
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, json + "\n").with_context(|| format!("writing {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

/// The household to act on: the given one, or the only one in the database.
async fn resolve_household(
    store: &Store,
    given: Option<HouseholdId>,
) -> Result<Option<HouseholdId>> {
    if given.is_some() {
        return Ok(given);
    }
    let mut tx = store.tx_system().await?;
    let all = HouseholdRepo::list(&mut tx).await?;
    tx.rollback().await?;
    match all.as_slice() {
        [] => Ok(None),
        [h] => Ok(Some(h.value.id)),
        _ => bail!("several households exist; pass --household <id>"),
    }
}

async fn require_household(store: &Store, given: Option<HouseholdId>) -> Result<HouseholdId> {
    resolve_household(store, given)
        .await?
        .ok_or_else(|| anyhow!("no household yet; run `fa-server config import <file>` first"))
}

/// Every RRULE and weather/season condition must parse before anything is written; all errors are
/// reported together with their location.
fn check_config(cfg: &HouseholdConfig) -> Result<()> {
    let mut errors = Vec::new();
    for (title, rrule) in cfg.rrules() {
        if let Err(m) = fa_tasks::validate_rrule(rrule) {
            errors.push(format!("task definition '{title}': invalid rrule: {m}"));
        }
    }
    for (label, condition) in cfg.conditions() {
        if let Err(e) = fa_weather::validate_condition(condition) {
            errors.push(format!(
                "{label}: invalid condition at position {}: {}",
                e.position, e.message
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        bail!(
            "{} error(s) in the household file:\n  {}",
            errors.len(),
            errors.join("\n  ")
        )
    }
}

pub async fn config(
    store: &Store,
    household: Option<HouseholdId>,
    action: ConfigAction,
) -> Result<()> {
    match action {
        ConfigAction::Import { file, replace } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let cfg = HouseholdConfig::from_toml(&text)?;
            check_config(&cfg)?;
            let mode = if replace {
                ImportMode::Replace
            } else {
                ImportMode::Merge
            };
            let hid = resolve_household(store, household).await?;
            let report = fa_config::import(store, hid, &cfg, mode).await?;
            let mut tx = store.tx_for(report.household_id).await?;
            let seeded = fa_weather::ensure_default_rules(&mut tx).await?;
            tx.commit().await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if !report.mail_sources_skipped.is_empty() {
                println!(
                    "connect these in Settings -> Mail (Gmail sources need a Google sign-in): {}",
                    report.mail_sources_skipped.join(", ")
                );
            }
            if seeded > 0 {
                println!("default weather rules added: {seeded}");
            }
        }
        ConfigAction::Export { out } => {
            let hid = require_household(store, household).await?;
            let toml = fa_config::export(store, hid).await?.to_toml()?;
            match out {
                Some(path) => {
                    std::fs::write(&path, toml)
                        .with_context(|| format!("writing {}", path.display()))?;
                    println!("wrote {}", path.display());
                }
                None => print!("{toml}"),
            }
        }
    }
    Ok(())
}

/// Reads a secret from the first stdin line, or prompts (without echo on a terminal).
fn read_secret(prompt: &str, from_stdin: bool) -> Result<String> {
    let stdin = std::io::stdin();
    if from_stdin || !stdin.is_terminal() {
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        return Ok(line.trim_end_matches(['\r', '\n']).to_owned());
    }
    let first = prompt_hidden(prompt)?;
    let second = prompt_hidden("Repeat: ")?;
    if first != second {
        bail!("entries do not match");
    }
    Ok(first)
}

fn prompt_hidden(prompt: &str) -> Result<String> {
    let stty = |arg: &str| {
        let _ = std::process::Command::new("stty").arg(arg).status();
    };
    eprint!("{prompt}");
    std::io::stderr().flush()?;
    stty("-echo");
    let mut line = String::new();
    let res = std::io::stdin().lock().read_line(&mut line);
    stty("echo");
    eprintln!();
    res?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

async fn audit(
    tx: &mut Tx,
    household: HouseholdId,
    action: &str,
    subject_type: &str,
    subject_id: String,
) -> Result<()> {
    AuditRepo::insert(
        tx,
        &AuditEntry {
            id: AuditEntryId::new(),
            household_id: household,
            actor_member: None,
            device: None,
            action: format!("cli.{action}"),
            subject_type: subject_type.into(),
            subject_id: Some(subject_id),
            before: None,
            after: None,
            at: Utc::now(),
        },
    )
    .await?;
    Ok(())
}

async fn member_by_key(tx: &mut Tx, hid: HouseholdId, key: &str) -> Result<Member> {
    Ok(MemberRepo::get_by_config_key(tx, hid, key)
        .await?
        .ok_or_else(|| anyhow!("no member with key '{key}'"))?
        .value)
}

pub async fn user(store: &Store, household: Option<HouseholdId>, action: UserAction) -> Result<()> {
    let accounts = AccountService::new(store.clone());
    match action {
        UserAction::Add {
            member,
            username,
            password_stdin,
        } => {
            let hid = require_household(store, household).await?;
            let mut tx = store.tx_for(hid).await?;
            let m = member_by_key(&mut tx, hid, &member).await?;
            if m.archived_at.is_some() {
                bail!("member '{member}' is archived");
            }
            if UserAccountRepo::get_by_member(&mut tx, m.id)
                .await?
                .is_some()
            {
                bail!("member '{member}' already has an account (use reset-password)");
            }
            tx.rollback().await?;
            let password = read_secret("Password: ", password_stdin)?;
            let account = accounts
                .create_account(&Actor::System, hid, m.id, &username, &password)
                .await?;
            println!(
                "created account '{}' for {} ({:?})",
                account.username, m.display_name, m.role
            );
        }
        UserAction::ResetPassword {
            username,
            password_stdin,
        } => {
            let mut sys = store.tx_system().await?;
            let found = UserAccountRepo::find_by_username(&mut sys, household, &username).await?;
            sys.rollback().await?;
            let found = found.ok_or_else(|| anyhow!("no account '{username}'"))?;
            let password = read_secret("New password: ", password_stdin)?;
            accounts
                .reset_password(
                    &Actor::System,
                    found.value.household_id,
                    &username,
                    &password,
                )
                .await?;
            println!("password reset for '{username}'; existing sessions ended");
        }
    }
    Ok(())
}

pub async fn device(
    store: &Store,
    public_base_url: &str,
    household: Option<HouseholdId>,
    action: DeviceAction,
) -> Result<()> {
    match action {
        DeviceAction::Pair { name } => {
            let hid = require_household(store, household).await?;
            let paired = DeviceService::new(store.clone(), public_base_url)
                .pair_system(hid, &name)
                .await?;
            println!("device id: {}", paired.device.id);
            println!("token:     {}", paired.token);
            println!("pair url:  {}", paired.pair_url);
            if let Some(lan) = crate::serve::lan_base_url() {
                println!("home network: {lan}/dashboard?device={}", paired.token);
            }
            println!("The token is shown once.");
        }
        DeviceAction::Revoke { id } => {
            let mut tx = store.tx_system().await?;
            let d = DeviceRepo::get(&mut tx, id)
                .await?
                .ok_or_else(|| anyhow!("no device {id}"))?
                .value;
            DeviceRepo::archive(&mut tx, id).await?;
            DeviceElevationRepo::clear(&mut tx, id).await?;
            audit(
                &mut tx,
                d.household_id,
                "device.revoke",
                "device",
                id.to_string(),
            )
            .await?;
            tx.commit().await?;
            println!("revoked device {id}");
        }
    }
    Ok(())
}

pub async fn calendar(
    cfg: &ProcessConfig,
    store: &Store,
    household: Option<HouseholdId>,
    action: CalendarAction,
) -> Result<()> {
    let CalendarAction::Sync { source } = action;
    let services = wiring::build_services(cfg, store)?;
    let (hid, name) = source_household(store, household, source).await?;
    let before = window_event_count(&services, store, hid).await?;
    services
        .calendar
        .sync_source(hid, source)
        .await
        .with_context(|| format!("syncing '{name}'"))?;
    let after = window_event_count(&services, store, hid).await?;
    let mut tx = store.tx_for(hid).await?;
    let src = CalendarSourceRepo::get(&mut tx, source)
        .await?
        .ok_or_else(|| anyhow!("source {source} vanished"))?
        .value;
    let calendars = fa_store::CalendarRepo::list(&mut tx, Some(source)).await?;
    tx.rollback().await?;
    let enabled = calendars.iter().filter(|c| c.value.enabled).count();
    println!("source '{name}': state {:?}", src.state);
    println!("calendars: {} ({enabled} enabled)", calendars.len());
    println!(
        "events in the window: {before} before, {after} after ({:+})",
        after as i64 - before as i64
    );
    if let Some(e) = src.last_error {
        println!("note: {e}");
    }
    Ok(())
}

async fn source_household(
    store: &Store,
    given: Option<HouseholdId>,
    source: CalendarSourceId,
) -> Result<(HouseholdId, String)> {
    let mut tx = store.tx_system().await?;
    let found = CalendarSourceRepo::get(&mut tx, source).await?;
    tx.rollback().await?;
    let src = found
        .filter(|s| s.value.archived_at.is_none())
        .ok_or_else(|| anyhow!("no calendar source {source}"))?
        .value;
    if given.is_some_and(|g| g != src.household_id) {
        bail!("source {source} belongs to another household");
    }
    Ok((src.household_id, src.name))
}

/// Events an adult sees in the household's sync window; the sync's effect shows as the delta.
async fn window_event_count(
    services: &fa_api::Services,
    store: &Store,
    hid: HouseholdId,
) -> Result<usize> {
    let mut tx = store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or_else(|| anyhow!("household vanished"))?
        .value;
    tx.rollback().await?;
    let now = Utc::now();
    let w = fa_calendar::window_of(&h, now);
    let from = w
        .from
        .max(now - chrono::Duration::days(fa_calendar::MAX_RANGE_DAYS / 2));
    let to =
        w.to.min(from + chrono::Duration::days(fa_calendar::MAX_RANGE_DAYS));
    let events = services
        .calendar
        .events(
            hid,
            Viewer::AdultUser,
            HouseholdFlags {
                private_events_as_busy: h.private_events_as_busy,
            },
            from,
            to,
            None,
        )
        .await?;
    Ok(events.len())
}

pub async fn weather(
    cfg: &ProcessConfig,
    store: &Store,
    household: Option<HouseholdId>,
    action: WeatherAction,
) -> Result<()> {
    let WeatherAction::Fetch = action;
    let hid = require_household(store, household).await?;
    let svc = fa_weather::WeatherService::new(store.clone(), wiring::weather_config(cfg))?;
    svc.fetch(hid).await.context("fetching the forecast")?;
    svc.evaluate(hid)
        .await
        .context("evaluating the weather rules")?;
    match svc.latest_view(hid).await? {
        Some(v) => {
            println!("fetched at {}", v.fetched_at.to_rfc3339());
            println!("current: {}", serde_json::to_string(&v.current)?);
        }
        None => println!("no forecast stored (does the household have a location?)"),
    }
    let mut tx = store.tx_for(hid).await?;
    let active = AdvisoryRepo::active(&mut tx, Utc::now()).await?;
    tx.rollback().await?;
    if active.is_empty() {
        println!("advisories: none");
    }
    for a in active {
        println!("advisory [{:?}]: {}", a.severity, a.text);
    }
    Ok(())
}

pub async fn notify(
    cfg: &ProcessConfig,
    store: &Store,
    household: Option<HouseholdId>,
    action: NotifyAction,
) -> Result<()> {
    let NotifyAction::Test { member } = action;
    let hid = require_household(store, household).await?;
    let mut tx = store.tx_for(hid).await?;
    let m = member_by_key(&mut tx, hid, &member).await?;
    tx.rollback().await?;
    let svc = fa_notify::NotifyService::new(store.clone(), wiring::notify_config(cfg));
    match svc.send_test(hid, m.id).await {
        Ok(()) => {}
        Err(fa_notify::NotifyError::NotConfigured) => {
            bail!(
                "ntfy is not configured; set FA_NTFY_URL (and FA_NTFY_TOKEN if the server needs one)"
            )
        }
        Err(e) => return Err(anyhow!(e)).context("publishing the test notification"),
    }
    let prefs = svc.get_prefs(hid, m.id).await?;
    println!("test notification sent to {}", m.display_name);
    if let Some(url) = prefs.subscribe_url {
        println!("subscribe url: {url}");
    }
    Ok(())
}

pub async fn tasks(
    store: &Store,
    household: Option<HouseholdId>,
    action: TasksAction,
) -> Result<()> {
    let TasksAction::RunNow = action;
    let hid = require_household(store, household).await?;
    let svc = fa_tasks::TaskService::new(store.clone());
    let now = Utc::now();
    let created = svc
        .materialize_all(now)
        .await
        .context("tasks.materialize")?;
    svc.tick(hid, now).await.context("tasks.tick")?;
    let runs = svc
        .routines_materialize(hid, now)
        .await
        .context("routines.materialize")?;
    println!("tasks.materialize: {created} task(s) created");
    println!("tasks.tick: done");
    println!("routines.materialize: {runs} run(s) created");
    Ok(())
}

pub async fn mail(
    cfg: &ProcessConfig,
    store: &Store,
    household: Option<HouseholdId>,
    action: MailAction,
) -> Result<()> {
    let hid = require_household(store, household).await?;
    let services = wiring::build_services(cfg, store)?;
    crate::mail::run(
        store,
        wiring::mail_deps(cfg, store, &services.oauth)?,
        hid,
        action,
    )
    .await
}

pub async fn pin(store: &Store, household: Option<HouseholdId>, action: PinAction) -> Result<()> {
    let PinAction::Set { member, pin_stdin } = action;
    let hid = require_household(store, household).await?;
    let mut tx = store.tx_for(hid).await?;
    let m = member_by_key(&mut tx, hid, &member).await?;
    if m.role != Role::Adult {
        bail!("only adults have a PIN");
    }
    let pin = read_secret("PIN (4 to 6 digits): ", pin_stdin)?;
    validate_pin(&pin)?;
    let hash = fa_auth::hash_password(&pin)?;
    MemberRepo::set_pin_hash(&mut tx, m.id, Some(&hash)).await?;
    audit(&mut tx, hid, "pin.set", "member", m.id.to_string()).await?;
    tx.commit().await?;
    println!("PIN set for {}", m.display_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_auth::{AuthConfig, AuthService};
    use sqlx::PgPool;

    #[test]
    fn gen_openapi_writes_document() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("nested/openapi.json");
        gen_openapi(&out).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        assert!(v["paths"]["/api/v1/me"].is_object());
    }

    #[sqlx::test(migrations = "../../crates/fa-store/migrations")]
    async fn cli_paired_token_authenticates(pool: PgPool) {
        let store = Store::from_pool(pool);
        let toml = include_str!("../../../config/examples/household.example.toml");
        let cfg = HouseholdConfig::from_toml(toml).unwrap();
        let report = fa_config::import(&store, None, &cfg, ImportMode::Merge)
            .await
            .unwrap();
        let paired = DeviceService::new(store.clone(), "http://localhost:8080")
            .pair_system(report.household_id, "Kitchen")
            .await
            .unwrap();
        let auth = AuthService::new(store.clone(), AuthConfig::default());
        let device = auth.authenticate_bearer(&paired.token).await.unwrap();
        assert_eq!(device.id, paired.device.id);
    }

    #[test]
    fn config_import_rejects_a_bad_rule_with_its_name_and_position() {
        let toml = include_str!("../../../config/examples/household.example.toml");
        let mut cfg = HouseholdConfig::from_toml(toml).unwrap();
        check_config(&cfg).unwrap();
        cfg.weather_rules.push(fa_config::WeatherRuleConfig {
            name: "Broken".into(),
            condition: "temperature >>> 3".into(),
            action: fa_domain::RuleAction::Alert,
            severity: fa_domain::Severity::Info,
            message: "x".into(),
            status: fa_domain::RuleStatus::Active,
            task_mode: None,
            task_definition: None,
        });
        let err = check_config(&cfg).unwrap_err().to_string();
        assert!(
            err.contains("'Broken'") && err.contains("position"),
            "{err}"
        );
    }

    #[test]
    fn config_import_reports_every_bad_rrule_and_season_condition() {
        let toml = include_str!("../../../config/examples/household.example.toml");
        let extra = r#"
[[task_lists]]
name = "Chores"

[[task_definitions]]
list = "Chores"
title = "Bins"
rrule = "FREQ=SOMETIMES"
candidates = ["alex"]

[[task_definitions]]
list = "Chores"
title = "Mow"
rrule = "FREQ=WEEKLY"
season_condition = "temperature >>> 3"
candidates = ["alex"]
"#;
        let cfg = HouseholdConfig::from_toml(&format!("{toml}\n{extra}")).unwrap();
        let err = check_config(&cfg).unwrap_err().to_string();
        assert!(err.contains("2 error(s)"), "{err}");
        assert!(err.contains("'Bins'") && err.contains("rrule"), "{err}");
        assert!(err.contains("'Mow'") && err.contains("position"), "{err}");
    }
}
