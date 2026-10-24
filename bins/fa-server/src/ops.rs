//! Operator commands that talk to the database directly (no running server needed).

use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use fa_config::{HouseholdConfig, ImportMode};
use fa_domain::{
    AuditEntry, AuditEntryId, Device, DeviceId, DeviceKind, HouseholdId, Member, Role, UserAccount,
    UserAccountId, validate_password, validate_pin,
};
use fa_store::{
    AuditRepo, DeviceElevationRepo, DeviceRepo, HouseholdRepo, MemberRepo, SessionRepo, Store, Tx,
    UserAccountRepo,
};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::cli::{ConfigAction, DeviceAction, PinAction, UserAction};

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
            let mode = if replace {
                ImportMode::Replace
            } else {
                ImportMode::Merge
            };
            let hid = resolve_household(store, household).await?;
            let report = fa_config::import(store, hid, &cfg, mode).await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
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
            let username = username.trim().to_owned();
            if username.is_empty() {
                bail!("username must not be empty");
            }
            let password = read_secret("Password: ", password_stdin)?;
            validate_password(m.role, &password)?;
            let account = UserAccount {
                id: UserAccountId::new(),
                household_id: hid,
                member_id: m.id,
                username: username.clone(),
                password_hash: fa_auth::hash_password(&password)?,
                disabled_at: None,
                created_at: Utc::now(),
            };
            UserAccountRepo::insert(&mut tx, &account).await?;
            audit(
                &mut tx,
                hid,
                "user.add",
                "user_account",
                account.id.to_string(),
            )
            .await?;
            tx.commit().await?;
            println!(
                "created account '{username}' for {} ({:?})",
                m.display_name, m.role
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
            let hid = found.value.household_id;
            let mut tx = store.tx_for(hid).await?;
            let m = MemberRepo::get(&mut tx, found.value.member_id)
                .await?
                .ok_or_else(|| anyhow!("account has no member"))?
                .value;
            let password = read_secret("New password: ", password_stdin)?;
            validate_password(m.role, &password)?;
            let hash = fa_auth::hash_password(&password)?;
            UserAccountRepo::update_password(&mut tx, found.value.id, &hash, found.version).await?;
            SessionRepo::delete_for_member(&mut tx, m.id).await?;
            audit(
                &mut tx,
                hid,
                "user.reset_password",
                "user_account",
                found.value.id.to_string(),
            )
            .await?;
            tx.commit().await?;
            println!("password reset for '{username}'; existing sessions ended");
        }
    }
    Ok(())
}

fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub async fn device(
    store: &Store,
    public_base_url: &str,
    household: Option<HouseholdId>,
    action: DeviceAction,
) -> Result<()> {
    match action {
        DeviceAction::Pair { name } => {
            let (token, id) = pair_device(store, household, &name).await?;
            println!("device id: {id}");
            println!("token:     {token}");
            println!(
                "pair url:  {}/dashboard#device={token}",
                public_base_url.trim_end_matches('/')
            );
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

async fn pair_device(
    store: &Store,
    household: Option<HouseholdId>,
    name: &str,
) -> Result<(String, DeviceId)> {
    let hid = require_household(store, household).await?;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let device = Device {
        id: DeviceId::new(),
        household_id: hid,
        kind: DeviceKind::Dashboard,
        name: name.trim().to_owned(),
        token_hash: token_hash(&token),
        last_seen: None,
        revoked_at: None,
        created_at: Utc::now(),
    };
    let mut tx = store.tx_for(hid).await?;
    DeviceRepo::insert(&mut tx, &device, None).await?;
    audit(&mut tx, hid, "device.pair", "device", device.id.to_string()).await?;
    tx.commit().await?;
    Ok((token, device.id))
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
        let (token, id) = pair_device(&store, Some(report.household_id), "Kitchen")
            .await
            .unwrap();
        let auth = AuthService::new(store.clone(), AuthConfig::default());
        let device = auth.authenticate_bearer(&token).await.unwrap();
        assert_eq!(device.id, id);
    }
}
