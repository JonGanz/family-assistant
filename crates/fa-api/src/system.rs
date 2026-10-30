//! System routes (D-074, D-075): the one-time master key backup and the runtime settings.
//!
//! A settings change is written to `instance_setting` and announced on `fa_settings`; the server
//! applies it by restarting its services in-process (T-07). This process keeps answering from its
//! start-up snapshot, so [`settings_view`] overlays the stored values to show what is pending.

use std::path::Path;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use fa_config::runtime::runtime_key;
use fa_domain::system_api::{
    AboutView, ConnectionInfo, MasterKeyRequest, MasterKeyView, SettingSource, SettingView,
    SystemSettingsPatch, SystemSettingsView,
};
use fa_domain::{Action, Resource};
use fa_store::{InstanceSettingRepo, MasterKey};
use serde_json::{Value, json};

use crate::conn::Conn;
use crate::error::Problem;
use crate::extract::{ApiJson, ClientIp};
use crate::routes::{ApiResult, audit};
use crate::{ApiError, AppState, Auth, require};

const BACKUP_KEY: &str = "key_backup_confirmed_at";

fn adult(actor: &fa_domain::Actor) -> ApiResult<()> {
    require(actor, Action::ManageHouseholdSettings, &Resource::Household)
}

/// The 32 key bytes: the file holds them raw, or as 64 hex characters.
fn key_bytes(path: &Path) -> ApiResult<[u8; 32]> {
    let raw = std::fs::read(path).map_err(|e| {
        tracing::warn!(error = %e, "cannot read the master key file");
        ApiError::Unavailable("the master key file cannot be read".into())
    })?;
    let bad = || ApiError::Internal("the master key file is malformed".into());
    if let Ok(k) = <[u8; 32]>::try_from(raw.as_slice()) {
        return Ok(k);
    }
    let text = std::str::from_utf8(&raw).map_err(|_| bad())?.trim();
    if text.len() != 64 || !text.is_ascii() {
        return Err(bad());
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(out)
}

#[utoipa::path(post, path = "/api/v1/system/master-key", tag = "system", request_body = MasterKeyRequest,
    security(("cookie" = [])),
    responses((status = 200, body = MasterKeyView, description = "the key as base64; the response is not cacheable"),
              (status = 401, body = Problem, description = "wrong password; counts toward the login lockout"),
              (status = 403, body = Problem), (status = 423, body = Problem),
              (status = 429, body = Problem), (status = 503, body = Problem)))]
pub async fn master_key(
    State(s): State<AppState>,
    auth: Auth,
    ClientIp(ip): ClientIp,
    ApiJson(req): ApiJson<MasterKeyRequest>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    adult(&auth.actor)?;
    s.auth
        .confirm_password(&auth.actor, &req.password, ip)
        .await?;
    let bytes = key_bytes(&s.cfg.master_key_path)?;
    let mut tx = s
        .store
        .tx_for(crate::routes::household(&auth.actor)?)
        .await?;
    audit(
        &mut tx,
        &auth.actor,
        "system.master_key_download",
        "instance",
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    let body = MasterKeyView {
        key_base64: STANDARD.encode(bytes),
    };
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(body),
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/system/master-key/confirm", tag = "system",
    security(("cookie" = [])),
    responses((status = 204, description = "the key backup is recorded as confirmed"),
              (status = 403, body = Problem)))]
pub async fn master_key_confirm(State(s): State<AppState>, auth: Auth) -> ApiResult<StatusCode> {
    adult(&auth.actor)?;
    let mut tx = s.store.tx_system().await?;
    InstanceSettingRepo::set(&mut tx, BACKUP_KEY, Some(&json!(Utc::now().to_rfc3339()))).await?;
    audit(
        &mut tx,
        &auth.actor,
        "system.master_key_confirm",
        "instance",
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// What the System page shows: the start-up snapshot with stored (pending) values laid over every
/// key that no environment variable locks. Secrets never carry a value.
async fn settings_view(s: &AppState, conn: &Conn) -> ApiResult<SystemSettingsView> {
    let mut tx = s.store.tx_system().await?;
    let rows = InstanceSettingRepo::all(&mut tx).await?;
    tx.rollback().await?;
    let settings = s
        .cfg
        .runtime
        .views()
        .into_iter()
        .map(|mut v: SettingView| {
            if v.source == SettingSource::Env {
                return v;
            }
            let secret = runtime_key(&v.key).is_some_and(|k| k.secret);
            match rows.iter().find(|r| r.key == v.key) {
                Some(r) if secret && r.has_secret => {
                    v.is_set = true;
                    v.source = SettingSource::Db;
                }
                Some(r) if !secret && r.value.is_some() => {
                    v.value = r.value.clone();
                    v.is_set = true;
                    v.source = SettingSource::Db;
                }
                _ if v.source == SettingSource::Db => {
                    // Reset in the database: back to the default once applied.
                    v.value = None;
                    v.is_set = false;
                    v.source = SettingSource::Default;
                }
                _ => {}
            }
            v
        })
        .collect();
    let key_backup_confirmed_at = rows
        .iter()
        .find(|r| r.key == BACKUP_KEY)
        .and_then(|r| r.value.as_ref()?.as_str())
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc));
    Ok(SystemSettingsView {
        settings,
        connection: ConnectionInfo {
            secure: conn.secure,
            origin: conn.origin.clone(),
        },
        key_backup_confirmed_at,
    })
}

#[utoipa::path(get, path = "/api/v1/system/settings", tag = "system",
    security(("cookie" = [])),
    responses((status = 200, body = SystemSettingsView), (status = 403, body = Problem)))]
pub async fn get_settings(
    State(s): State<AppState>,
    conn: Conn,
    auth: Auth,
) -> ApiResult<Json<SystemSettingsView>> {
    adult(&auth.actor)?;
    Ok(Json(settings_view(&s, &conn).await?))
}

#[utoipa::path(patch, path = "/api/v1/system/settings", tag = "system", request_body = SystemSettingsPatch,
    security(("cookie" = [])),
    responses((status = 200, body = SystemSettingsView,
               description = "stored; the server applies the change by restarting its services"),
              (status = 403, body = Problem),
              (status = 409, body = Problem, description = "code `setting_locked`: an environment variable sets the key"),
              (status = 422, body = Problem, description = "code `invalid_setting`; `field` is the key")))]
pub async fn patch_settings(
    State(s): State<AppState>,
    conn: Conn,
    auth: Auth,
    ApiJson(patch): ApiJson<SystemSettingsPatch>,
) -> ApiResult<Json<SystemSettingsView>> {
    adult(&auth.actor)?;
    let invalid = |key: &str, msg: String| ApiError::Coded {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        code: "invalid_setting",
        field: Some(key.to_owned()),
        message: format!("{key}: {msg}"),
    };
    let mut changes: Vec<(&str, bool, Option<Value>)> = Vec::new();
    for (key, value) in &patch.values {
        let rk = runtime_key(key).ok_or_else(|| invalid(key, "unknown setting".into()))?;
        if s.cfg.runtime.is_locked(key) {
            return Err(ApiError::Coded {
                status: StatusCode::CONFLICT,
                code: "setting_locked",
                field: Some(key.clone()),
                message: format!("{key} is set by an environment variable"),
            });
        }
        let value = match value {
            Some(v) if !v.is_null() => {
                Some(fa_config::validate_value(key, v).map_err(|m| invalid(key, m))?)
            }
            _ => None,
        };
        changes.push((rk.key, rk.secret, value));
    }
    if !changes.is_empty() {
        let master = if changes.iter().any(|(_, secret, v)| *secret && v.is_some()) {
            Some(
                MasterKey::from_bytes(&key_bytes(&s.cfg.master_key_path)?, 1).map_err(|e| {
                    tracing::warn!(error = %e, "master key unusable");
                    ApiError::Unavailable("the master key file cannot be used".into())
                })?,
            )
        } else {
            None
        };
        let mut tx = s.store.tx_system().await?;
        for (key, secret, value) in &changes {
            match (secret, master.as_ref()) {
                (true, Some(m)) => {
                    InstanceSettingRepo::set_secret(
                        &mut tx,
                        key,
                        m,
                        value.as_ref().and_then(Value::as_str),
                    )
                    .await?;
                }
                (true, None) => {
                    // A reset needs no key: it deletes the row.
                    InstanceSettingRepo::set(&mut tx, key, None).await?;
                }
                (false, _) => {
                    InstanceSettingRepo::set(&mut tx, key, value.as_ref()).await?;
                }
            }
        }
        InstanceSettingRepo::notify_changed(&mut tx).await?;
        // Keys only; values (and secrets) are never audited.
        let keys: Vec<&str> = changes.iter().map(|(k, _, _)| *k).collect();
        audit(
            &mut tx,
            &auth.actor,
            "system.settings_update",
            "instance",
            None,
            Some(json!({ "keys": keys })),
        )
        .await?;
        tx.commit().await?;
    }
    Ok(Json(settings_view(&s, &conn).await?))
}

#[utoipa::path(get, path = "/api/v1/system/about", tag = "system", security(("cookie" = [])),
    responses((status = 200, body = AboutView, description = "version and commit of the running server"),
              (status = 403, body = Problem)))]
pub async fn get_about(State(state): State<AppState>, auth: Auth) -> ApiResult<Json<AboutView>> {
    adult(&auth.actor)?;
    let b = &state.cfg.build;
    Ok(Json(AboutView {
        version: b.version.clone(),
        commit: b.commit.clone(),
        built_at: b.built_at.clone(),
    }))
}
