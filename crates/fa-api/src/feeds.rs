//! M1 routes: household settings, avatars, OAuth, calendar sources, events, weather, advisories,
//! weather rules, device settings, the dashboard snapshot and the status page.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::extract::rejection::BytesRejection;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, HeaderName, IF_NONE_MATCH, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Duration as ChronoDuration, NaiveTime, TimeZone, Utc};
use fa_domain::api::*;
use fa_domain::calendar::{HouseholdFlags, Viewer};
use fa_domain::{
    Action, Actor, AdvisoryId, CalendarId, CalendarSourceId, DashboardSettings, DeviceId,
    Household, HouseholdId, MemberId, MemberPublicDto, Resource, Role, RuleStatus, SuggestedBy,
    WeatherRule, WeatherRuleId,
};
use fa_store::{
    AdvisoryRepo, CalendarRepo, CalendarSourceRepo, ChangeBus, DeviceRepo, HouseholdRepo, JobRepo,
    MemberAvatarRepo, MemberRepo, NewJob, SystemStatusRepo, Tx, WeatherRuleRepo,
    WeatherSnapshotRepo,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::routes::{ApiResult, audit, avatar_urls, etag, household, parse_version};
use crate::{ApiError, AppState, Auth, require};

/// Largest accepted avatar upload.
pub const AVATAR_MAX_BYTES: usize = 512 * 1024;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(20);
const NOSNIFF: HeaderName = HeaderName::from_static("x-content-type-options");

fn trim_name(raw: &str, what: &str) -> ApiResult<String> {
    let n = raw.trim();
    if n.is_empty() || n.chars().count() > 100 {
        return Err(ApiError::Unprocessable(format!(
            "{what} must be 1 to 100 characters"
        )));
    }
    Ok(n.to_owned())
}

async fn load_household(
    s: &AppState,
    hid: HouseholdId,
) -> ApiResult<fa_store::Versioned<Household>> {
    let mut tx = s.store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.rollback().await?;
    Ok(h)
}

fn household_view(h: &Household, version: i32, google: bool) -> HouseholdView {
    HouseholdView {
        name: h.name.clone(),
        tz: h.tz.clone(),
        location: h.location.clone(),
        units: h.units,
        calendar_past_days: h.calendar_past_days,
        calendar_future_days: h.calendar_future_days,
        weather_retention_days: h.weather_retention_days,
        private_events_as_busy: h.private_events_as_busy,
        dashboard_completion_override: h.dashboard_completion_override,
        now_list_size: h.now_list_size,
        now_weights: h.now_weights,
        quiet_from: h.quiet_from,
        quiet_until: h.quiet_until,
        google_oauth_enabled: google,
        currency: h.currency.clone(),
        gender_terms: h.gender_terms.clone(),
        events: h.events.clone(),
        inbox: h.inbox.clone(),
        retention_mail: h.retention_mail.clone(),
        bill_lead_days: h.bill_lead_days,
        version,
    }
}

/// How the actor sees events; devices and children are masked, adults see everything.
fn viewer_of(actor: &Actor) -> ApiResult<Viewer> {
    match actor {
        Actor::User {
            role: Role::Adult, ..
        } => Ok(Viewer::AdultUser),
        Actor::User {
            role: Role::Child,
            member_id,
            ..
        } => Ok(Viewer::ChildUser(*member_id)),
        Actor::Device { .. } => Ok(Viewer::Device),
        Actor::ElevatedDevice { .. } => Ok(Viewer::ElevatedDevice),
        Actor::System => Err(ApiError::Forbidden("forbidden".into())),
    }
}

fn acting_member(actor: &Actor) -> Option<MemberId> {
    match actor {
        Actor::User { member_id, .. } => Some(*member_id),
        Actor::ElevatedDevice { adult, .. } => Some(*adult),
        _ => None,
    }
}

// ---- household ----

#[utoipa::path(get, path = "/api/v1/household", tag = "household",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = HouseholdView), (status = 401, body = Problem)))]
pub async fn get_household(State(s): State<AppState>, auth: Auth) -> ApiResult<Response> {
    require(&auth.actor, Action::ReadDashboard, &Resource::Household)?;
    let h = load_household(&s, household(&auth.actor)?).await?;
    Ok((
        [(ETAG, etag(h.version))],
        Json(household_view(
            &h.value,
            h.version,
            s.services.oauth.enabled(),
        )),
    )
        .into_response())
}

fn validate_household_patch(p: &mut HouseholdPatch) -> ApiResult<()> {
    if let Some(n) = &p.name {
        p.name = Some(trim_name(n, "name")?);
    }
    if let Some(tz) = &p.tz
        && tz.parse::<chrono_tz::Tz>().is_err()
    {
        return Err(ApiError::Unprocessable(format!("unknown time zone '{tz}'")));
    }
    if let Some(l) = &p.location {
        if !(-90.0..=90.0).contains(&l.lat) || !(-180.0..=180.0).contains(&l.lon) {
            return Err(ApiError::Unprocessable(
                "location is outside the valid latitude and longitude range".into(),
            ));
        }
        if l.label.chars().count() > 100 {
            return Err(ApiError::Unprocessable(
                "location label must be at most 100 characters".into(),
            ));
        }
    }
    let range = |v: Option<i32>, lo: i32, hi: i32, what: &str| match v {
        Some(v) if !(lo..=hi).contains(&v) => Err(ApiError::Unprocessable(format!(
            "{what} must be {lo} to {hi}"
        ))),
        _ => Ok(()),
    };
    range(p.calendar_past_days, 0, 3650, "calendar_past_days")?;
    range(p.calendar_future_days, 1, 800, "calendar_future_days")?;
    range(p.weather_retention_days, 1, 365, "weather_retention_days")?;
    range(p.now_list_size, 1, 50, "now_list_size")?;
    range(p.bill_lead_days, 0, 60, "bill_lead_days")?;
    if let Some(c) = &p.currency {
        let c = c.trim().to_uppercase();
        if c.len() != 3 || !c.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return Err(ApiError::Unprocessable(
                "currency must be a 3-letter ISO 4217 code".into(),
            ));
        }
        p.currency = Some(c);
    }
    if let Some(e) = &p.events {
        for (v, what) in [
            (e.extract_threshold, "extract_threshold"),
            (e.route_threshold, "route_threshold"),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return Err(ApiError::Unprocessable(format!(
                    "events.{what} must be 0 to 1"
                )));
            }
        }
    }
    if let Some(i) = &p.inbox {
        range(Some(i.task_expiry_days), 1, 365, "inbox.task_expiry_days")?;
        range(
            Some(i.contact_expiry_days),
            1,
            365,
            "inbox.contact_expiry_days",
        )?;
    }
    if let Some(r) = &p.retention_mail {
        range(
            Some(r.examples_per_category),
            0,
            100_000,
            "retention_mail.examples_per_category",
        )?;
        range(
            Some(r.max_pending_days),
            1,
            365,
            "retention_mail.max_pending_days",
        )?;
    }
    if let Some(w) = &p.now_weights {
        for (v, what) in [(w.due, "due"), (w.flag, "flag"), (w.weather, "weather")] {
            if !(0.0..=10.0).contains(&v) {
                return Err(ApiError::Unprocessable(format!(
                    "now_weights.{what} must be 0 to 10"
                )));
            }
        }
    }
    Ok(())
}

#[utoipa::path(patch, path = "/api/v1/household", tag = "household", request_body = HouseholdPatch,
    params(("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = HouseholdView), (status = 403, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem),
              (status = 428, body = Problem)))]
pub async fn patch_household(
    State(s): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
    ApiJson(mut patch): ApiJson<HouseholdPatch>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::ManageHouseholdSettings,
        &Resource::Household,
    )?;
    let hid = household(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    validate_household_patch(&mut patch)?;
    let mut tx = s.store.tx_for(hid).await?;
    HouseholdRepo::update_settings(&mut tx, hid, &patch, expected).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?;
    if patch.location.is_some() || patch.tz.is_some() {
        // A new place needs a forecast now, not at the next half hour.
        let mut job = NewJob::new("weather.fetch");
        job.household_id = Some(hid);
        job.payload = json!({});
        job.idempotency_key = Some(format!(
            "weather.fetch:{hid}:{}",
            Utc::now().timestamp() / 60
        ));
        JobRepo::enqueue(&mut tx, &job).await?;
    }
    audit(
        &mut tx,
        &auth.actor,
        "household.update",
        "household",
        Some(hid.to_string()),
        None,
    )
    .await?;
    ChangeBus::publish(&mut tx, hid, "household.updated", &json!({})).await?;
    tx.commit().await?;
    Ok((
        [(ETAG, etag(h.version))],
        Json(household_view(
            &h.value,
            h.version,
            s.services.oauth.enabled(),
        )),
    )
        .into_response())
}

// ---- avatars ----

fn sniff_image(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

async fn manageable_member(
    s: &AppState,
    actor: &Actor,
    id: MemberId,
) -> ApiResult<(HouseholdId, Tx)> {
    let hid = household(actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let m = MemberRepo::get(&mut tx, id)
        .await?
        .map(|m| m.value)
        .filter(|m| m.archived_at.is_none())
        .ok_or(ApiError::NotFound)?;
    require(actor, Action::ManageMember, &Resource::Member(id, m.role))?;
    Ok((hid, tx))
}

#[utoipa::path(put, path = "/api/v1/members/{id}/avatar", tag = "members",
    params(("id" = String, Path, description = "member id")),
    request_body(content = Vec<u8>, content_type = "image/png",
        description = "raw image bytes (png, jpeg or webp), at most 512 KB"),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 413, body = Problem), (status = 415, body = Problem)))]
pub async fn put_avatar(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> ApiResult<StatusCode> {
    // The declared type is only a gate; the stored type comes from the magic bytes.
    let declared = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        });
    if !matches!(
        declared.as_deref(),
        Some("image/png" | "image/jpeg" | "image/webp")
    ) {
        return Err(ApiError::UnsupportedMedia(
            "send image/png, image/jpeg or image/webp".into(),
        ));
    }
    let bytes = body.map_err(|r| {
        if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::PayloadTooLarge(format!(
                "the avatar must be at most {AVATAR_MAX_BYTES} bytes"
            ))
        } else {
            ApiError::BadRequest("could not read the request body".into())
        }
    })?;
    if bytes.len() > AVATAR_MAX_BYTES {
        return Err(ApiError::PayloadTooLarge(format!(
            "the avatar must be at most {AVATAR_MAX_BYTES} bytes"
        )));
    }
    let mime = sniff_image(&bytes).ok_or_else(|| {
        ApiError::UnsupportedMedia("the file is not a png, jpeg or webp image".into())
    })?;
    let (hid, mut tx) = manageable_member(&s, &auth.actor, id).await?;
    let sha = format!("{:x}", Sha256::digest(&bytes));
    MemberAvatarRepo::put(&mut tx, id, mime, &bytes, &sha).await?;
    audit(
        &mut tx,
        &auth.actor,
        "member.avatar_set",
        "member",
        Some(id.to_string()),
        None,
    )
    .await?;
    ChangeBus::publish(&mut tx, hid, "member.updated", &json!({ "id": id })).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/api/v1/members/{id}/avatar", tag = "members",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_avatar(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
) -> ApiResult<StatusCode> {
    let (hid, mut tx) = manageable_member(&s, &auth.actor, id).await?;
    MemberAvatarRepo::delete(&mut tx, id).await?;
    audit(
        &mut tx,
        &auth.actor,
        "member.avatar_delete",
        "member",
        Some(id.to_string()),
        None,
    )
    .await?;
    ChangeBus::publish(&mut tx, hid, "member.updated", &json!({ "id": id })).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/members/{id}/avatar", tag = "members",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, content_type = "image/*", description = "the image; ETag is its sha256"),
              (status = 304), (status = 404, body = Problem)))]
pub async fn get_avatar(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    require(&auth.actor, Action::ReadMemberPublic, &Resource::Household)?;
    let mut tx = s.store.tx_for(household(&auth.actor)?).await?;
    let found = MemberAvatarRepo::get(&mut tx, id).await?;
    tx.rollback().await?;
    let (mime, bytes, sha) = found.ok_or(ApiError::NotFound)?;
    let tag = format!("\"{sha}\"");
    let cache = (
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    let etag_value = HeaderValue::from_str(&tag).map_err(|e| ApiError::Internal(e.to_string()))?;
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|t| t.trim().trim_start_matches("W/") == tag)
        });
    if fresh {
        return Ok((StatusCode::NOT_MODIFIED, [cache, (ETAG, etag_value)]).into_response());
    }
    let ct = HeaderValue::from_str(&mime).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((
        [
            (CONTENT_TYPE, ct),
            (ETAG, etag_value),
            cache,
            (NOSNIFF, HeaderValue::from_static("nosniff")),
        ],
        bytes,
    )
        .into_response())
}

// ---- OAuth ----

/// `return_to` may only be a path on this site: never a scheme, host, `//host` or backslash form.
pub(crate) fn safe_return_to(s: &str) -> Option<&str> {
    let ok = s.starts_with('/')
        && !s.starts_with("//")
        && s.len() <= 512
        && !s.chars().any(|c| c == '\\' || c.is_control());
    ok.then_some(s)
}

fn found(location: &str) -> ApiResult<Response> {
    let v = HeaderValue::from_str(location).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((StatusCode::FOUND, [(LOCATION, v)]).into_response())
}

fn done_redirect(pairs: &[(&str, &str)]) -> ApiResult<Response> {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        q.append_pair(k, v);
    }
    found(&format!("/oauth/done?{}", q.finish()))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StartQuery {
    /// A path on this site to return to afterwards.
    return_to: Option<String>,
    /// What the connection is for; adds that scope (`include_granted_scopes`). Default `calendar`.
    purpose: Option<PurposeParam>,
}

#[derive(Deserialize, ToSchema, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum PurposeParam {
    Calendar,
    GmailRead,
    GmailModify,
    CalendarWrite,
}

impl From<PurposeParam> for fa_auth::OAuthPurpose {
    fn from(p: PurposeParam) -> Self {
        match p {
            PurposeParam::Calendar => Self::Calendar,
            PurposeParam::GmailRead => Self::GmailRead,
            PurposeParam::GmailModify => Self::GmailModify,
            PurposeParam::CalendarWrite => Self::CalendarWrite,
        }
    }
}

#[utoipa::path(get, path = "/api/v1/oauth/google/start", tag = "oauth", params(StartQuery),
    security(("cookie" = [])),
    responses((status = 302, description = "redirect to Google's consent page"),
              (status = 400, body = Problem), (status = 403, body = Problem),
              (status = 404, body = Problem, description = "Google is not configured")))]
pub async fn oauth_start(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<StartQuery>,
) -> ApiResult<Response> {
    let session = auth
        .session_id
        .as_deref()
        .filter(|_| matches!(auth.actor, Actor::User { .. }))
        .ok_or_else(|| ApiError::Forbidden("a signed-in adult is required".into()))?;
    let return_to =
        match q.return_to.as_deref() {
            None => None,
            Some(r) => Some(safe_return_to(r).ok_or_else(|| {
                ApiError::BadRequest("return_to must be a path on this site".into())
            })?),
        };
    let url = s
        .services
        .oauth
        .start(
            &auth.actor,
            session,
            return_to,
            q.purpose.unwrap_or(PurposeParam::Calendar).into(),
        )
        .await?;
    found(&url)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Set by Google when the user declined.
    error: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/oauth/google/callback", tag = "oauth", params(CallbackQuery),
    security(("cookie" = [])),
    responses((status = 302, description = "redirect to /oauth/done?account=…&return_to=… or ?error=…"),
              (status = 404, body = Problem), (status = 409, body = Problem)))]
pub async fn oauth_callback(
    State(s): State<AppState>,
    auth: Result<Auth, ApiError>,
    ApiQuery(q): ApiQuery<CallbackQuery>,
) -> ApiResult<Response> {
    if let Some(e) = &q.error {
        let code: String = e
            .chars()
            .filter(|c| c.is_ascii_lowercase() || *c == '_')
            .take(40)
            .collect();
        return done_redirect(&[(
            "error",
            if code.is_empty() {
                "oauth_failed"
            } else {
                &code
            },
        )]);
    }
    let Ok(auth) = auth else {
        return done_redirect(&[("error", "session_required")]);
    };
    let (Some(session), Some(code), Some(state)) = (
        auth.session_id.as_deref(),
        q.code.as_deref(),
        q.state.as_deref(),
    ) else {
        return done_redirect(&[("error", "invalid_request")]);
    };
    match s
        .services
        .oauth
        .callback(&auth.actor, session, code, state)
        .await
    {
        Ok((account, return_to)) => {
            let account = account.to_string();
            let mut pairs = vec![("account", account.as_str())];
            if let Some(r) = return_to.as_deref().and_then(safe_return_to) {
                pairs.push(("return_to", r));
            }
            done_redirect(&pairs)
        }
        Err(fa_auth::AuthError::InvalidState) => done_redirect(&[("error", "invalid_state")]),
        Err(fa_auth::AuthError::OAuth(m)) => {
            tracing::warn!(error = %m, "oauth callback failed");
            done_redirect(&[("error", "oauth_failed")])
        }
        Err(e) => Err(e.into()),
    }
}

#[utoipa::path(get, path = "/api/v1/oauth/accounts", tag = "oauth",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<OAuthAccountView>), (status = 403, body = Problem)))]
pub async fn list_oauth_accounts(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<OAuthAccountView>>> {
    require_adult_sources(&auth.actor)?;
    let list = s.services.oauth.list_accounts(&auth.actor).await?;
    Ok(Json(list.into_iter().map(oauth_view).collect()))
}

fn oauth_view(a: fa_auth::OAuthAccountView) -> OAuthAccountView {
    OAuthAccountView {
        id: a.id,
        provider: a.provider,
        owner_member: a.owner_member,
        email: a.email,
        scopes: a.scopes,
        state: a.state,
        last_error: a.last_error,
    }
}

// ---- calendar sources ----

/// Only signed-in adults reach the source and calendar settings.
fn require_adult_sources(actor: &Actor) -> ApiResult<()> {
    require(
        actor,
        Action::ManageCalendarSource,
        &Resource::Source {
            owner: None,
            owner_role: None,
        },
    )
}

async fn member_roles(s: &AppState, hid: HouseholdId) -> ApiResult<HashMap<MemberId, Role>> {
    let mut tx = s.store.tx_for(hid).await?;
    let list = MemberRepo::list(&mut tx, false).await?;
    tx.rollback().await?;
    Ok(list
        .into_iter()
        .map(|m| (m.value.id, m.value.role))
        .collect())
}

fn may_manage(actor: &Actor, owner: Option<MemberId>, roles: &HashMap<MemberId, Role>) -> bool {
    let owner_role = owner.and_then(|o| roles.get(&o).copied());
    require(
        actor,
        Action::ManageCalendarSource,
        &Resource::Source { owner, owner_role },
    )
    .is_ok()
}

/// Loads the source and checks the D-006 owner rules. Returns the household and the version.
async fn manageable_source(
    s: &AppState,
    actor: &Actor,
    id: CalendarSourceId,
) -> ApiResult<(HouseholdId, i32)> {
    let hid = household(actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let src = CalendarSourceRepo::get(&mut tx, id)
        .await?
        .filter(|v| v.value.archived_at.is_none())
        .ok_or(ApiError::NotFound)?;
    let owner_role = CalendarSourceRepo::owner_role(&mut tx, id).await?;
    tx.rollback().await?;
    require(
        actor,
        Action::ManageCalendarSource,
        &Resource::Source {
            owner: src.value.owner_member,
            owner_role,
        },
    )?;
    Ok((hid, src.version))
}

/// A calendar source plus whether the app may write events to it.
#[derive(serde::Serialize, ToSchema)]
pub struct SourceViewExt {
    #[serde(flatten)]
    pub source: SourceView,
    /// Extracted events may be written to this source's calendars.
    pub writable: bool,
}

/// `SourcePatch` plus the write toggle.
#[derive(Deserialize, ToSchema)]
pub struct SourcePatchExt {
    #[serde(flatten)]
    pub patch: SourcePatch,
    /// Google sources need the account to have granted calendar write access (else 422).
    #[serde(default)]
    pub writable: Option<bool>,
}

/// A calendar plus whether it is a default target for extracted events.
#[derive(serde::Serialize, ToSchema)]
pub struct CalendarViewExt {
    #[serde(flatten)]
    pub calendar: CalendarView,
    pub is_default_target: bool,
}

/// `CalendarPatch` plus the default-target toggle.
#[derive(Deserialize, ToSchema)]
pub struct CalendarPatchExt {
    #[serde(flatten)]
    pub patch: CalendarPatch,
    #[serde(default)]
    pub is_default_target: Option<bool>,
}

#[utoipa::path(get, path = "/api/v1/calendar-sources", tag = "calendar",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<SourceViewExt>), (status = 403, body = Problem)))]
pub async fn list_sources(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<SourceViewExt>>> {
    require_adult_sources(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let roles = member_roles(&s, hid).await?;
    let list = s.services.calendar.list_sources(hid).await?;
    let writable = s.services.calendar.writable_sources(hid).await?;
    Ok(Json(
        list.into_iter()
            .filter(|v| may_manage(&auth.actor, v.owner, &roles))
            .map(|source| SourceViewExt {
                writable: writable.contains(&source.id),
                source,
            })
            .collect(),
    ))
}

fn set_owner(new: &mut NewSource, o: Option<MemberId>) {
    match new {
        NewSource::Google { owner, .. }
        | NewSource::Caldav { owner, .. }
        | NewSource::Ics { owner, .. } => *owner = o,
    }
}

/// Applies the owner rules to a new source (and the account rule for Google) and returns its owner.
async fn authorize_new_source(
    s: &AppState,
    actor: &Actor,
    new: &mut NewSource,
) -> ApiResult<Option<MemberId>> {
    require_adult_sources(actor)?;
    let hid = household(actor)?;
    let mut owner = match new {
        NewSource::Google { owner, .. }
        | NewSource::Caldav { owner, .. }
        | NewSource::Ics { owner, .. } => *owner,
    };
    if let NewSource::Google { oauth_account, .. } = new {
        // Only accounts the actor may use (their own and children's) can back a source.
        let account = s
            .services
            .oauth
            .list_accounts(actor)
            .await?
            .into_iter()
            .find(|a| a.id == *oauth_account)
            .ok_or_else(|| ApiError::Forbidden("that Google account is not available".into()))?;
        owner = owner.or(Some(account.owner_member));
    }
    let roles = member_roles(s, hid).await?;
    if let Some(o) = owner
        && !roles.contains_key(&o)
    {
        return Err(ApiError::Unprocessable("the owner is not a member".into()));
    }
    if !may_manage(actor, owner, &roles) {
        return Err(ApiError::Forbidden("forbidden".into()));
    }
    set_owner(new, owner);
    Ok(owner)
}

#[utoipa::path(post, path = "/api/v1/calendar-sources", tag = "calendar", request_body = NewSource,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = SourceCreated), (status = 403, body = Problem),
              (status = 422, body = Problem), (status = 502, body = Problem),
              (status = 504, body = Problem)))]
pub async fn create_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(mut new): ApiJson<NewSource>,
) -> ApiResult<Response> {
    let owner = authorize_new_source(&s, &auth.actor, &mut new).await?;
    let hid = household(&auth.actor)?;
    let (source, calendars) = tokio::time::timeout(
        DISCOVERY_TIMEOUT,
        s.services.calendar.create_source(hid, owner, new),
    )
    .await
    .map_err(|_| {
        ApiError::GatewayTimeout("the calendar server did not answer in 20 seconds".into())
    })??;
    audit_after(
        &s,
        &auth.actor,
        "calendar_source.create",
        "calendar_source",
        source.id.to_string(),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(source.version))],
        Json(SourceCreated { source, calendars }),
    )
        .into_response())
}

/// Audit rows for service-level mutations are written after the fact; a failure is logged only.
pub(crate) async fn audit_after(
    s: &AppState,
    actor: &Actor,
    action: &str,
    subject_type: &str,
    subject: String,
) {
    let res = async {
        let mut tx = s.store.tx_for(household(actor)?).await?;
        audit(&mut tx, actor, action, subject_type, Some(subject), None).await?;
        tx.commit().await?;
        Ok::<_, ApiError>(())
    }
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, action, "audit write failed");
    }
}

#[utoipa::path(post, path = "/api/v1/calendar-sources/test", tag = "calendar", request_body = NewSource,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = SourceTestResult), (status = 403, body = Problem)))]
pub async fn test_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(mut new): ApiJson<NewSource>,
) -> ApiResult<Json<SourceTestResult>> {
    authorize_new_source(&s, &auth.actor, &mut new).await?;
    let hid = household(&auth.actor)?;
    let res = tokio::time::timeout(
        DISCOVERY_TIMEOUT,
        s.services.calendar.test_source(hid, &new),
    )
    .await
    .unwrap_or_else(|_| SourceTestResult {
        ok: false,
        calendars: vec![],
        sample_events: vec![],
        error: Some("the calendar server did not answer in 20 seconds".into()),
    });
    Ok(Json(res))
}

#[utoipa::path(patch, path = "/api/v1/calendar-sources/{id}", tag = "calendar", request_body = SourcePatchExt,
    params(("id" = String, Path, description = "source id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = SourceViewExt), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<CalendarSourceId>,
    headers: HeaderMap,
    ApiJson(SourcePatchExt {
        mut patch,
        writable,
    }): ApiJson<SourcePatchExt>,
) -> ApiResult<Response> {
    require_adult_sources(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let (hid, _) = manageable_source(&s, &auth.actor, id).await?;
    if let Some(n) = &patch.name {
        patch.name = Some(trim_name(n, "name")?);
    }
    if patch.poll_minutes.is_some_and(|m| !(1..=1440).contains(&m)) {
        return Err(ApiError::Unprocessable(
            "poll_minutes must be 1 to 1440".into(),
        ));
    }
    let view = s
        .services
        .calendar
        .update_source(hid, id, &patch, expected)
        .await?;
    let view = if let Some(w) = writable {
        s.services.calendar.set_source_writable(hid, id, w).await?;
        // The toggle bumped the version again: answer with the row as it is now.
        s.services
            .calendar
            .list_sources(hid)
            .await?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or(ApiError::NotFound)?
    } else {
        view
    };
    let version = view.version;
    let view = SourceViewExt {
        writable: s
            .services
            .calendar
            .writable_sources(hid)
            .await?
            .contains(&id),
        source: view,
    };
    audit_after(
        &s,
        &auth.actor,
        "calendar_source.update",
        "calendar_source",
        id.to_string(),
    )
    .await;
    Ok(([(ETAG, etag(version))], Json(view)).into_response())
}

#[utoipa::path(delete, path = "/api/v1/calendar-sources/{id}", tag = "calendar",
    params(("id" = String, Path, description = "source id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<CalendarSourceId>,
) -> ApiResult<StatusCode> {
    require_adult_sources(&auth.actor)?;
    let (hid, _) = manageable_source(&s, &auth.actor, id).await?;
    s.services.calendar.archive_source(hid, id).await?;
    audit_after(
        &s,
        &auth.actor,
        "calendar_source.archive",
        "calendar_source",
        id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/calendar-sources/{id}/sync", tag = "calendar",
    params(("id" = String, Path, description = "source id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 202, description = "a sync job is queued"), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn sync_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<CalendarSourceId>,
) -> ApiResult<StatusCode> {
    require_adult_sources(&auth.actor)?;
    let (hid, _) = manageable_source(&s, &auth.actor, id).await?;
    s.services.calendar.enqueue_sync(hid, id).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(get, path = "/api/v1/calendars", tag = "calendar",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<CalendarViewExt>), (status = 403, body = Problem)))]
pub async fn list_calendars(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<CalendarViewExt>>> {
    require_adult_sources(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let roles = member_roles(&s, hid).await?;
    let allowed: HashSet<CalendarSourceId> = s
        .services
        .calendar
        .list_sources(hid)
        .await?
        .into_iter()
        .filter(|v| may_manage(&auth.actor, v.owner, &roles))
        .map(|v| v.id)
        .collect();
    let list = s.services.calendar.list_calendars(hid).await?;
    let defaults = s.services.calendar.default_calendars(hid).await?;
    Ok(Json(
        list.into_iter()
            .filter(|c| allowed.contains(&c.source))
            .map(|calendar| CalendarViewExt {
                is_default_target: defaults.contains(&calendar.id),
                calendar,
            })
            .collect(),
    ))
}

#[utoipa::path(patch, path = "/api/v1/calendars/{id}", tag = "calendar", request_body = CalendarPatchExt,
    params(("id" = String, Path, description = "calendar id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = CalendarViewExt), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 428, body = Problem)))]
pub async fn patch_calendar(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<CalendarId>,
    headers: HeaderMap,
    ApiJson(CalendarPatchExt {
        mut patch,
        is_default_target,
    }): ApiJson<CalendarPatchExt>,
) -> ApiResult<Response> {
    require_adult_sources(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let cal = CalendarRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let src = CalendarSourceRepo::get(&mut tx, cal.value.source)
        .await?
        .ok_or(ApiError::NotFound)?;
    let owner_role = CalendarSourceRepo::owner_role(&mut tx, cal.value.source).await?;
    let roles: HashSet<MemberId> = MemberRepo::list(&mut tx, false)
        .await?
        .into_iter()
        .map(|m| m.value.id)
        .collect();
    tx.rollback().await?;
    require(
        &auth.actor,
        Action::ManageCalendarSource,
        &Resource::Source {
            owner: src.value.owner_member,
            owner_role,
        },
    )?;
    if let Some(n) = &patch.name {
        patch.name = Some(trim_name(n, "name")?);
    }
    if patch
        .members
        .as_ref()
        .is_some_and(|m| m.iter().any(|x| !roles.contains(x)))
    {
        return Err(ApiError::Unprocessable("unknown member in members".into()));
    }
    let view = s
        .services
        .calendar
        .update_calendar(hid, id, &patch, expected)
        .await?;
    let view = if let Some(d) = is_default_target {
        s.services.calendar.set_default_target(hid, id, d).await?;
        s.services
            .calendar
            .list_calendars(hid)
            .await?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or(ApiError::NotFound)?
    } else {
        view
    };
    let version = view.version;
    let view = CalendarViewExt {
        is_default_target: s
            .services
            .calendar
            .default_calendars(hid)
            .await?
            .contains(&id),
        calendar: view,
    };
    Ok(([(ETAG, etag(version))], Json(view)).into_response())
}

// ---- events ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventsQuery {
    /// Inclusive start (RFC 3339).
    from: DateTime<Utc>,
    /// Exclusive end (RFC 3339); at most 400 days after `from`.
    to: DateTime<Utc>,
    /// Only events linked to this member.
    member: Option<MemberId>,
}

#[utoipa::path(get, path = "/api/v1/events", tag = "calendar", params(EventsQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<EventView>), (status = 400, body = Problem)))]
pub async fn list_events(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<EventsQuery>,
) -> ApiResult<Json<Vec<EventView>>> {
    require(&auth.actor, Action::ReadCalendar, &Resource::Event)?;
    let hid = household(&auth.actor)?;
    let viewer = viewer_of(&auth.actor)?;
    let h = load_household(&s, hid).await?;
    let flags = HouseholdFlags {
        private_events_as_busy: h.value.private_events_as_busy,
    };
    let events = s
        .services
        .calendar
        .events(hid, viewer, flags, q.from, q.to, q.member)
        .await?;
    Ok(Json(events))
}

// ---- weather & advisories ----

#[utoipa::path(get, path = "/api/v1/weather", tag = "weather",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = WeatherView), (status = 404, body = Problem, description = "no forecast fetched yet")))]
pub async fn get_weather(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<WeatherView>> {
    require(&auth.actor, Action::ReadDashboard, &Resource::Household)?;
    let v = s
        .services
        .weather
        .latest_view(household(&auth.actor)?)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(v))
}

fn advisory_view(a: fa_domain::Advisory) -> AdvisoryView {
    AdvisoryView {
        id: a.id,
        text: a.text,
        severity: a.severity,
        valid_from: a.valid_from,
        valid_until: a.valid_until,
        source: a.source,
    }
}

async fn active_advisories(s: &AppState, hid: HouseholdId) -> ApiResult<Vec<AdvisoryView>> {
    let mut tx = s.store.tx_for(hid).await?;
    let list = AdvisoryRepo::active(&mut tx, Utc::now()).await?;
    tx.rollback().await?;
    Ok(list.into_iter().map(advisory_view).collect())
}

#[utoipa::path(get, path = "/api/v1/advisories", tag = "weather",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<AdvisoryView>)))]
pub async fn list_advisories(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<AdvisoryView>>> {
    require(&auth.actor, Action::ReadDashboard, &Resource::Household)?;
    Ok(Json(active_advisories(&s, household(&auth.actor)?).await?))
}

#[utoipa::path(post, path = "/api/v1/advisories/{id}/dismiss", tag = "weather",
    params(("id" = String, Path, description = "advisory id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 404, body = Problem)))]
pub async fn dismiss_advisory(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<AdvisoryId>,
) -> ApiResult<StatusCode> {
    require(&auth.actor, Action::DismissAdvisory, &Resource::Household)?;
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    AdvisoryRepo::dismiss(&mut tx, id, acting_member(&auth.actor)).await?;
    ChangeBus::publish(&mut tx, hid, "advisory.updated", &json!({ "id": id })).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- weather rules ----

fn rule_view(r: &WeatherRule, version: i32) -> WeatherRuleView {
    WeatherRuleView {
        id: r.id,
        name: r.name.clone(),
        condition: r.condition.clone(),
        action: r.action,
        severity: r.severity,
        message: r.message.clone(),
        status: r.status,
        suggested_by: r.suggested_by,
        task_mode: r.task_mode,
        task_definition: r.task_definition.clone(),
        version,
    }
}

fn require_rules(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ManageWeatherRules, &Resource::Household)
}

fn check_rule_text(name: &str, condition: &str, message: &str) -> ApiResult<()> {
    if name.trim().is_empty() || name.chars().count() > 100 {
        return Err(ApiError::Unprocessable(
            "name must be 1 to 100 characters".into(),
        ));
    }
    if condition.trim().is_empty() || condition.chars().count() > 1000 {
        return Err(ApiError::Unprocessable(
            "condition must be 1 to 1000 characters".into(),
        ));
    }
    if message.chars().count() > 500 {
        return Err(ApiError::Unprocessable(
            "message must be at most 500 characters".into(),
        ));
    }
    Ok(())
}

/// A bad condition becomes a 422 carrying `{position, message}`.
async fn check_condition(s: &AppState, hid: HouseholdId, condition: &str) -> ApiResult<()> {
    let v = s.services.weather.validate(hid, condition, None).await?;
    match v.error {
        Some(e) if !v.ok => Err(ApiError::RuleInvalid {
            position: u32::try_from(e.position).unwrap_or(u32::MAX),
            message: e.message,
        }),
        _ => Ok(()),
    }
}

/// Re-evaluates the rules against the current snapshot after an edit.
async fn enqueue_evaluate(tx: &mut Tx, hid: HouseholdId) -> ApiResult<()> {
    let mut job = NewJob::new("weather.evaluate");
    job.household_id = Some(hid);
    job.payload = json!({ "household": hid });
    JobRepo::enqueue(tx, &job).await?;
    Ok(())
}

#[utoipa::path(get, path = "/api/v1/weather-rules", tag = "weather",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<WeatherRuleView>), (status = 403, body = Problem)))]
pub async fn list_rules(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<WeatherRuleView>>> {
    require_rules(&auth.actor)?;
    let mut tx = s.store.tx_for(household(&auth.actor)?).await?;
    let list = WeatherRuleRepo::list(&mut tx, None).await?;
    tx.rollback().await?;
    Ok(Json(
        list.iter()
            .map(|r| rule_view(&r.value, r.version))
            .collect(),
    ))
}

#[utoipa::path(post, path = "/api/v1/weather-rules", tag = "weather", request_body = NewWeatherRule,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = WeatherRuleView), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "invalid condition: `position` and `message` are set")))]
pub async fn create_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(new): ApiJson<NewWeatherRule>,
) -> ApiResult<Response> {
    require_rules(&auth.actor)?;
    let hid = household(&auth.actor)?;
    check_rule_text(&new.name, &new.condition, &new.message)?;
    check_condition(&s, hid, &new.condition).await?;
    crate::tasks::check_rule_task(&s, hid, new.action, new.task_definition.as_ref()).await?;
    let now = Utc::now();
    let rule = WeatherRule {
        id: WeatherRuleId::new(),
        household_id: hid,
        name: new.name.trim().to_owned(),
        condition: new.condition.trim().to_owned(),
        action: new.action,
        severity: new.severity,
        message: new.message,
        task_definition: new.task_definition.filter(|v| !v.is_null()),
        status: new.status.unwrap_or(RuleStatus::Active),
        suggested_by: SuggestedBy::User,
        task_mode: new.task_mode.unwrap_or(fa_domain::TaskMode::Create),
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    let mut tx = s.store.tx_for(hid).await?;
    let stored = WeatherRuleRepo::insert(&mut tx, &rule).await?;
    audit(
        &mut tx,
        &auth.actor,
        "weather_rule.create",
        "weather_rule",
        Some(rule.id.to_string()),
        None,
    )
    .await?;
    enqueue_evaluate(&mut tx, hid).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(stored.version))],
        Json(rule_view(&stored.value, stored.version)),
    )
        .into_response())
}

#[utoipa::path(patch, path = "/api/v1/weather-rules/{id}", tag = "weather", request_body = WeatherRulePatch,
    params(("id" = String, Path, description = "rule id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = WeatherRuleView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<WeatherRuleId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<WeatherRulePatch>,
) -> ApiResult<Response> {
    require_rules(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let mut tx = s.store.tx_for(hid).await?;
    let mut r = WeatherRuleRepo::get(&mut tx, id)
        .await?
        .filter(|r| r.value.archived_at.is_none())
        .ok_or(ApiError::NotFound)?
        .value;
    tx.rollback().await?;
    if let Some(v) = p.name {
        r.name = v.trim().to_owned();
    }
    if let Some(v) = p.condition {
        r.condition = v.trim().to_owned();
    }
    if let Some(v) = p.action {
        r.action = v;
    }
    if let Some(v) = p.severity {
        r.severity = v;
    }
    if let Some(v) = p.message {
        r.message = v;
    }
    if let Some(v) = p.status {
        r.status = v;
    }
    if let Some(v) = p.task_mode {
        r.task_mode = v;
    }
    let action_changed = p.action.is_some();
    if let Some(v) = p.task_definition.filter(|v| !v.is_null()) {
        r.task_definition = Some(v);
        crate::tasks::check_rule_task(&s, hid, r.action, r.task_definition.as_ref()).await?;
    } else if action_changed {
        if r.action == fa_domain::RuleAction::Alert {
            r.task_definition = None;
        }
        crate::tasks::check_rule_task(&s, hid, r.action, r.task_definition.as_ref()).await?;
    }
    check_rule_text(&r.name, &r.condition, &r.message)?;
    check_condition(&s, hid, &r.condition).await?;
    let mut tx = s.store.tx_for(hid).await?;
    let version = WeatherRuleRepo::update(&mut tx, &r, expected).await?;
    audit(
        &mut tx,
        &auth.actor,
        "weather_rule.update",
        "weather_rule",
        Some(id.to_string()),
        None,
    )
    .await?;
    enqueue_evaluate(&mut tx, hid).await?;
    tx.commit().await?;
    Ok(([(ETAG, etag(version))], Json(rule_view(&r, version))).into_response())
}

#[utoipa::path(delete, path = "/api/v1/weather-rules/{id}", tag = "weather",
    params(("id" = String, Path, description = "rule id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<WeatherRuleId>,
) -> ApiResult<StatusCode> {
    require_rules(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    WeatherRuleRepo::archive(&mut tx, id).await?;
    audit(
        &mut tx,
        &auth.actor,
        "weather_rule.archive",
        "weather_rule",
        Some(id.to_string()),
        None,
    )
    .await?;
    enqueue_evaluate(&mut tx, hid).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/weather-rules/validate", tag = "weather", request_body = ValidateRuleRequest,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = ValidateRuleResponse), (status = 403, body = Problem)))]
pub async fn validate_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<ValidateRuleRequest>,
) -> ApiResult<Json<ValidateRuleResponse>> {
    require_rules(&auth.actor)?;
    let v = s
        .services
        .weather
        .validate(
            household(&auth.actor)?,
            &req.condition,
            req.message.as_deref(),
        )
        .await?;
    Ok(Json(ValidateRuleResponse {
        ok: v.ok,
        error: v.error.map(|e| RuleError {
            position: u32::try_from(e.position).unwrap_or(u32::MAX),
            message: e.message,
        }),
        preview: v.preview.map(|p| RulePreview {
            matches: p.matches,
            value: p.value,
            when: p.when.map(|(from, _)| from.to_rfc3339()),
        }),
    }))
}

// ---- device settings ----

/// Settings that were never saved (or no longer parse) fall back to the default preset.
fn parse_settings(raw: serde_json::Value) -> DashboardSettings {
    serde_json::from_value(raw).unwrap_or_default()
}

async fn device_view(tx: &mut Tx, id: DeviceId) -> ApiResult<DeviceView> {
    let d = DeviceRepo::get(tx, id).await?.ok_or(ApiError::NotFound)?;
    let settings = parse_settings(DeviceRepo::get_settings(tx, id).await?);
    Ok(DeviceView {
        id: d.value.id,
        kind: d.value.kind,
        name: d.value.name,
        last_seen: d.value.last_seen,
        revoked: d.value.revoked_at.is_some(),
        settings,
        version: d.version,
    })
}

#[utoipa::path(get, path = "/api/v1/devices/{id}", tag = "devices",
    params(("id" = String, Path, description = "device id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = DeviceView), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn get_device(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<DeviceId>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::ManageDeviceSettings,
        &Resource::Device(id),
    )?;
    let mut tx = s.store.tx_for(household(&auth.actor)?).await?;
    let v = device_view(&mut tx, id).await?;
    tx.rollback().await?;
    Ok(([(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/devices/{id}", tag = "devices", request_body = DevicePatch,
    params(("id" = String, Path, description = "device id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = DeviceView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_device(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<DeviceId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<DevicePatch>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::ManageDeviceSettings,
        &Resource::Device(id),
    )?;
    let hid = household(&auth.actor)?;
    let mut version = parse_version(&headers, None)?;
    if let Some(st) = &p.settings {
        st.validate()
            .map_err(|e| ApiError::Unprocessable(e.to_string()))?;
    }
    let name = p
        .name
        .as_deref()
        .map(|n| trim_name(n, "name"))
        .transpose()?;
    let mut tx = s.store.tx_for(hid).await?;
    if let Some(n) = &name {
        version = DeviceRepo::rename(&mut tx, id, n, version).await?;
    }
    if let Some(st) = &p.settings {
        let raw = serde_json::to_value(st).map_err(|e| ApiError::Internal(e.to_string()))?;
        DeviceRepo::set_settings(&mut tx, id, &raw, version).await?;
    }
    let view = device_view(&mut tx, id).await?;
    audit(
        &mut tx,
        &auth.actor,
        "device.update",
        "device",
        Some(id.to_string()),
        None,
    )
    .await?;
    ChangeBus::publish(&mut tx, hid, "device.updated", &json!({ "id": id })).await?;
    tx.commit().await?;
    Ok(([(ETAG, etag(view.version))], Json(view)).into_response())
}

// ---- dashboard ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DashboardQuery {
    /// Days of agenda from today (1 to 14); defaults to the device setting.
    days: Option<u8>,
}

/// `[start of today, start of today + days)` in the household time zone.
fn day_window(tz: &str, days: u8, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let tz: chrono_tz::Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    let midnight = |d: chrono::NaiveDate| {
        tz.from_local_datetime(&d.and_time(NaiveTime::MIN))
            .earliest()
            .or_else(|| tz.from_local_datetime(&d.and_time(NaiveTime::MIN)).latest())
            .map_or_else(
                || d.and_time(NaiveTime::MIN).and_utc(),
                |t| t.with_timezone(&Utc),
            )
    };
    let today = now.with_timezone(&tz).date_naive();
    (
        midnight(today),
        midnight(today + ChronoDuration::days(i64::from(days))),
    )
}

#[utoipa::path(get, path = "/api/v1/dashboard", tag = "dashboard", params(DashboardQuery),
    security(("bearer" = [])),
    responses((status = 200, body = DashboardView), (status = 403, body = Problem)))]
pub async fn get_dashboard(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<DashboardQuery>,
) -> ApiResult<Json<DashboardView>> {
    let (Actor::Device { device_id, .. } | Actor::ElevatedDevice { device_id, .. }) = auth.actor
    else {
        return Err(ApiError::Forbidden(
            "the dashboard is for paired devices".into(),
        ));
    };
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    let settings = parse_settings(DeviceRepo::get_settings(&mut tx, device_id).await?);
    let urls = avatar_urls(&mut tx).await?;
    let members: Vec<MemberPublicDto> = MemberRepo::list(&mut tx, false)
        .await?
        .iter()
        .map(|m| {
            let mut dto = MemberPublicDto::from(&m.value);
            dto.avatar_url = urls.get(&m.value.id).cloned();
            dto
        })
        .collect();
    tx.rollback().await?;

    let days = q.days.unwrap_or(settings.agenda_days);
    if !(1..=fa_domain::MAX_AGENDA_DAYS).contains(&days) {
        return Err(ApiError::BadRequest(format!(
            "days must be 1 to {}",
            fa_domain::MAX_AGENDA_DAYS
        )));
    }
    let (from, to) = day_window(&h.tz, days, Utc::now());
    let events = s
        .services
        .calendar
        .events(
            hid,
            viewer_of(&auth.actor)?,
            HouseholdFlags {
                private_events_as_busy: h.private_events_as_busy,
            },
            from,
            to,
            None,
        )
        .await?;
    let weather = s.services.weather.latest_view(hid).await?;
    let advisories = active_advisories(&s, hid).await?;
    let tasks = crate::tasks::tasks_for_members(
        s.services.tasks.dashboard_tasks(hid, from, to).await?,
        &settings.members,
    );
    let routines = crate::tasks::runs_for_members(
        s.services.tasks.dashboard_routines(hid).await?,
        &settings.members,
    );
    Ok(Json(DashboardView {
        household: DashboardHousehold {
            name: h.name,
            tz: h.tz,
            units: h.units,
            location: h.location,
            now_list_size: h.now_list_size,
            completion_override: h.dashboard_completion_override,
        },
        members,
        events,
        tasks,
        routines,
        task_lists: s.services.tasks.list_task_lists(hid).await?,
        inbox: crate::mail::inbox_counts(&s, hid).await?,
        weather,
        advisories,
        settings,
    }))
}

// ---- status ----

#[utoipa::path(get, path = "/api/v1/status", tag = "status",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = StatusView), (status = 403, body = Problem)))]
pub async fn get_status(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<StatusView>> {
    require(
        &auth.actor,
        Action::ManageHouseholdSettings,
        &Resource::Household,
    )?;
    let hid = household(&auth.actor)?;
    let roles = member_roles(&s, hid).await?;
    let mut tx = s.store.tx_for(hid).await?;
    let sources = CalendarSourceRepo::list(&mut tx, false)
        .await?
        .into_iter()
        .map(|v| v.value)
        .filter(|v| may_manage(&auth.actor, v.owner_member, &roles))
        .map(|v| SourceStatus {
            id: v.id,
            name: v.name,
            kind: v.kind,
            state: v.state,
            last_synced_at: v.last_synced_at,
            last_error: v.last_error,
        })
        .collect();
    let snapshot = WeatherSnapshotRepo::latest(&mut tx).await?;
    tx.rollback().await?;

    let mut sys = s.store.tx_system().await?;
    let weather_status = SystemStatusRepo::get(&mut sys, "weather").await?;
    let backup_status = SystemStatusRepo::get(&mut sys, "backup").await?;
    let counts = fa_store::JobRepo::counts(&mut sys).await?;
    sys.rollback().await?;

    let flag = |v: &Option<(serde_json::Value, DateTime<Utc>)>| {
        v.as_ref()
            .and_then(|(j, _)| j.get("ok"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    };
    let last_backup = backup_status
        .as_ref()
        .and_then(|(j, _)| j.get("mtime"))
        .and_then(serde_json::Value::as_str)
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.with_timezone(&Utc));
    Ok(Json(StatusView {
        sources,
        weather: WeatherStatus {
            fetched_at: snapshot.map(|w| w.fetched_at),
            ok: flag(&weather_status),
        },
        backup: BackupStatus {
            last_at: last_backup,
            ok: flag(&backup_status),
        },
        queue: QueueStatus {
            ready: counts.ready,
            running: counts.running,
            dead: counts.dead,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_to_must_be_a_local_path() {
        for ok in ["/", "/settings/calendars", "/a?b=c&d=e#f"] {
            assert_eq!(safe_return_to(ok), Some(ok), "{ok}");
        }
        for bad in [
            "https://evil.example.com",
            "//evil.example.com",
            "/\\evil.example.com",
            "evil.example.com",
            "javascript:alert(1)",
            "/a\r\nSet-Cookie: x=1",
            "",
        ] {
            assert_eq!(safe_return_to(bad), None, "{bad}");
        }
    }

    #[test]
    fn image_types_come_from_magic_bytes() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(
            sniff_image(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
        assert_eq!(
            sniff_image(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            None
        );
        assert_eq!(sniff_image(b"GIF89a"), None);
    }

    #[test]
    fn day_window_follows_the_household_zone() {
        let now = Utc.with_ymd_and_hms(2026, 3, 28, 23, 30, 0).unwrap();
        // Berlin is already on 29 March, the day the clocks go forward: that day has 23 hours.
        let (from, to) = day_window("Europe/Berlin", 1, now);
        assert_eq!(from, Utc.with_ymd_and_hms(2026, 3, 28, 23, 0, 0).unwrap());
        assert_eq!(to - from, ChronoDuration::hours(23));
        let (from, to) = day_window("not/a-zone", 2, now);
        assert_eq!(from, Utc.with_ymd_and_hms(2026, 3, 28, 0, 0, 0).unwrap());
        assert_eq!(to - from, ChronoDuration::days(2));
    }
}
