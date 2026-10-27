//! `household.toml` import and export (the web equivalent of `fa-server config`).

use axum::Json;
use axum::extract::State;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use fa_config::{HouseholdConfig, ImportMode, ImportReport};
use fa_domain::{Action, Resource};
use serde::Deserialize;
use serde_json::json;
use utoipa::IntoParams;

use crate::error::Problem;
use crate::extract::ApiQuery;
use crate::routes::{ApiResult, audit, household};
use crate::{ApiError, AppState, Auth, require};

/// Largest accepted import body.
pub const IMPORT_MAX_BYTES: usize = 1024 * 1024;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ImportQuery {
    /// Validate and report without writing anything.
    #[serde(default)]
    dry_run: bool,
    /// Archive keyed members the file no longer lists (default: merge).
    #[serde(default)]
    replace: bool,
}

fn config_err(e: fa_config::HouseholdTomlError) -> ApiError {
    ApiError::Unprocessable(e.to_string())
}

/// Every RRULE and weather or season condition must parse before anything is written.
fn check(cfg: &HouseholdConfig) -> ApiResult<()> {
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
        Err(ApiError::Unprocessable(errors.join("; ")))
    }
}

#[utoipa::path(post, path = "/api/v1/config/import", tag = "config", params(ImportQuery),
    request_body(content = String, content_type = "text/plain", description = "household TOML, at most 1 MiB"),
    security(("cookie" = [])),
    responses((status = 200, body = ImportReport, description = "with `dry_run=true` nothing is written"),
              (status = 403, body = Problem), (status = 413, body = Problem),
              (status = 415, body = Problem), (status = 422, body = Problem)))]
pub async fn import_config(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<ImportQuery>,
    headers: HeaderMap,
    body: Result<String, axum::extract::rejection::StringRejection>,
) -> ApiResult<Json<ImportReport>> {
    require(
        &auth.actor,
        Action::ManageHouseholdSettings,
        &Resource::Household,
    )?;
    let is_text = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim().to_ascii_lowercase().starts_with("text/plain"));
    if !is_text {
        return Err(ApiError::UnsupportedMedia("send text/plain TOML".into()));
    }
    let text = body.map_err(|r| {
        if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::PayloadTooLarge("the file must be at most 1 MiB".into())
        } else {
            ApiError::BadRequest("the body must be UTF-8 text".into())
        }
    })?;
    let hid = household(&auth.actor)?;
    let cfg = HouseholdConfig::from_toml(&text).map_err(config_err)?;
    check(&cfg)?;
    let mode = if q.replace {
        ImportMode::Replace
    } else {
        ImportMode::Merge
    };
    let report = fa_config::import(&s.store, Some(hid), &cfg, mode, q.dry_run)
        .await
        .map_err(|e| match e {
            fa_config::ImportError::Config(c) => config_err(c),
            fa_config::ImportError::Store(st) => st.into(),
            other => ApiError::Internal(other.to_string()),
        })?;
    if !q.dry_run {
        let mut tx = s.store.tx_for(hid).await?;
        fa_weather::ensure_default_rules(&mut tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        audit(
            &mut tx,
            &auth.actor,
            "config.import",
            "household",
            Some(hid.to_string()),
            Some(json!({ "replace": q.replace })),
        )
        .await?;
        for kind in [
            "household.updated",
            "member.updated",
            "tasks.updated",
            "calendar.updated",
        ] {
            fa_store::ChangeBus::publish(&mut tx, hid, kind, &json!({})).await?;
        }
        tx.commit().await?;
    }
    Ok(Json(report))
}

#[utoipa::path(get, path = "/api/v1/config/export", tag = "config",
    security(("cookie" = [])),
    responses((status = 200, content_type = "text/plain", description = "household TOML as an attachment; no secrets"),
              (status = 403, body = Problem)))]
pub async fn export_config(State(s): State<AppState>, auth: Auth) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::ManageHouseholdSettings,
        &Resource::Household,
    )?;
    let hid = household(&auth.actor)?;
    let toml = fa_config::export(&s.store, hid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .to_toml()
        .map_err(config_err)?;
    Ok((
        [
            (
                CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (
                CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"household.toml\""),
            ),
        ],
        toml,
    )
        .into_response())
}
