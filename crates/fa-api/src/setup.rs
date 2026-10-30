//! First-run setup (D-073): a household, then its first adult, then the wizard's "done".
//!
//! The first two steps take `pg_advisory_xact_lock`, so two visitors cannot both win; the loser
//! gets 409 `setup_closed`. Until both exist, [`crate::setup_gate`] answers 409 `setup_required`
//! on every other call.

use axum::Json;
use axum::extract::State;
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use fa_domain::system_api::{NewSetupHousehold, NewSetupOwner, SetupStatus};
use fa_domain::{Action, HouseholdId, Resource};
use fa_store::{InstanceSettingRepo, SetupRepo, SetupState};
use serde::Serialize;
use serde_json::json;
use url::Url;
use utoipa::ToSchema;

use crate::conn::Conn;
use crate::error::Problem;
use crate::extract::ApiJson;
use crate::routes::{ApiResult, MeResponse, audit, household, me_for, set_cookie};
use crate::{ApiError, AppState, Auth, require};

pub(crate) async fn state(s: &AppState) -> ApiResult<SetupState> {
    let mut tx = s.store.tx_system().await?;
    let st = SetupRepo::state(&mut tx).await?;
    tx.rollback().await?;
    Ok(st)
}

fn closed() -> ApiError {
    ApiError::coded(
        StatusCode::CONFLICT,
        "setup_closed",
        "setup has already been done",
    )
}

#[utoipa::path(get, path = "/api/v1/setup", tag = "setup",
    responses((status = 200, body = SetupStatus)))]
pub async fn get_setup(State(s): State<AppState>, conn: Conn) -> ApiResult<Json<SetupStatus>> {
    let st = state(&s).await?;
    Ok(Json(SetupStatus {
        needed: st.household.is_none(),
        owner_needed: st.household.is_some() && !st.has_adult_account,
        completed: st.completed_at.is_some(),
        secure: conn.secure,
        origin: conn.origin.clone(),
        via_tailscale: conn.via_tailscale(),
    }))
}

#[derive(Serialize, ToSchema)]
pub struct SetupHouseholdCreated {
    #[schema(value_type = String, format = Uuid)]
    pub household_id: HouseholdId,
}

/// The settings key and the normalised origin for the browser's `location.origin`: an https
/// `*.ts.net` origin is the tailnet address, anything else the public one.
fn base_url_setting(raw: &str) -> ApiResult<(&'static str, serde_json::Value)> {
    let bad = |m: &str| ApiError::Unprocessable(format!("base_url: {m}"));
    let url = Url::parse(raw.trim()).map_err(|_| bad("not a valid URL"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(bad("must be an http:// or https:// origin"));
    }
    let origin = url.origin().ascii_serialization();
    let tailnet = url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|h| h.to_ascii_lowercase().ends_with(".ts.net"));
    let key = if tailnet {
        "tailnet_base_url"
    } else {
        "public_base_url"
    };
    let value = fa_config::validate_value(key, &json!(origin)).map_err(|m| bad(&m))?;
    Ok((key, value))
}

#[utoipa::path(post, path = "/api/v1/setup/household", tag = "setup", request_body = NewSetupHousehold,
    responses((status = 201, body = SetupHouseholdCreated),
              (status = 409, body = Problem, description = "code `setup_closed`"),
              (status = 422, body = Problem)))]
pub async fn setup_household(
    State(s): State<AppState>,
    ApiJson(new): ApiJson<NewSetupHousehold>,
) -> ApiResult<Response> {
    let (key, value) = base_url_setting(&new.base_url)?;
    let mut tx = s.store.tx_system().await?;
    fa_auth::lock_setup(&mut tx).await?;
    if SetupRepo::state(&mut tx).await?.household.is_some() {
        return Err(closed());
    }
    let hid = fa_config::create_household(&s.store, &new)
        .await
        .map_err(|e| match e {
            fa_config::HouseholdError::Invalid(errs) => ApiError::Unprocessable(errs.join("; ")),
            other => ApiError::Internal(other.to_string()),
        })?;
    // An environment variable that sets the key wins; there is nothing to store then.
    if !s.cfg.runtime.is_locked(key) {
        InstanceSettingRepo::set(&mut tx, key, Some(&value)).await?;
    }
    tx.commit().await?;
    let seeded = async {
        let mut tx = s.store.tx_for(hid).await?;
        fa_weather::ensure_default_rules(&mut tx).await?;
        tx.commit().await?;
        Ok::<_, ApiError>(())
    }
    .await;
    if let Err(e) = seeded {
        tracing::warn!(error = %e, "default weather rules not added");
    }
    Ok((
        StatusCode::CREATED,
        Json(SetupHouseholdCreated { household_id: hid }),
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/setup/owner", tag = "setup", request_body = NewSetupOwner,
    responses((status = 201, body = MeResponse, description = "sets the fa_session cookie"),
              (status = 409, body = Problem, description = "code `setup_closed`"),
              (status = 422, body = Problem)))]
pub async fn setup_owner(
    State(s): State<AppState>,
    conn: Conn,
    headers: HeaderMap,
    ApiJson(new): ApiJson<NewSetupOwner>,
) -> ApiResult<Response> {
    let hid = state(&s).await?.household.ok_or_else(|| {
        ApiError::coded(
            StatusCode::CONFLICT,
            "setup_required",
            "create the household first",
        )
    })?;
    let ua = headers.get(USER_AGENT).and_then(|v| v.to_str().ok());
    let out = s.auth.setup_owner(hid, &new, ua).await?;
    let actor = fa_domain::Actor::User {
        member_id: out.member_id,
        role: out.role,
        household_id: out.household_id,
    };
    let me = me_for(&s, &actor).await?;
    let max_age = s.auth.config().session_ttl(out.role).as_secs();
    let mut res = (StatusCode::CREATED, Json(me)).into_response();
    set_cookie(&mut res, &conn, &s.cfg, &out.session_id, max_age);
    res.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    Ok(res)
}

#[utoipa::path(post, path = "/api/v1/setup/complete", tag = "setup",
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem)))]
pub async fn setup_complete(State(s): State<AppState>, auth: Auth) -> ApiResult<StatusCode> {
    require(
        &auth.actor,
        Action::ManageHouseholdSettings,
        &Resource::Household,
    )?;
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    SetupRepo::complete(&mut tx, hid).await?;
    audit(
        &mut tx,
        &auth.actor,
        "setup.complete",
        "household",
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
