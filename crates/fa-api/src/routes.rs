use std::convert::Infallible;

use axum::Json;
use axum::extract::State;
use axum::http::header::{ETAG, IF_MATCH, SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{
    Action, Actor, AuditEntry, AuditEntryId, DeviceDto, DeviceId, HouseholdDto, HouseholdId,
    Member, MemberDto, MemberId, MemberPublicDto, Resource, Role, UserAccount, UserAccountDto,
    UserAccountId, validate_password,
};
use fa_store::{
    AuditRepo, DeviceRepo, HouseholdRepo, MemberRepo, SessionRepo, Tx, UserAccountRepo,
};
use futures::Stream;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::broadcast::error::RecvError;
use utoipa::ToSchema;

use crate::extract::{ApiJson, ApiPath, ApiQuery, ClientIp};
use crate::hub::HubEvent;
use crate::{ApiError, AppState, Auth, SESSION_COOKIE, require, schemas};

type ApiResult<T> = Result<T, ApiError>;

fn household(actor: &Actor) -> ApiResult<HouseholdId> {
    match actor {
        Actor::User { household_id, .. }
        | Actor::Device { household_id, .. }
        | Actor::ElevatedDevice { household_id, .. } => Ok(*household_id),
        Actor::System => Err(ApiError::Forbidden("forbidden".into())),
    }
}

/// The acting member (the adult behind an elevated device) and device, for the audit trail.
fn audit_ids(actor: &Actor) -> (Option<MemberId>, Option<DeviceId>) {
    match actor {
        Actor::User { member_id, .. } => (Some(*member_id), None),
        Actor::ElevatedDevice {
            adult, device_id, ..
        } => (Some(*adult), Some(*device_id)),
        Actor::Device { device_id, .. } => (None, Some(*device_id)),
        Actor::System => (None, None),
    }
}

async fn audit(
    tx: &mut Tx,
    actor: &Actor,
    action: &str,
    subject_type: &str,
    subject_id: Option<String>,
    after: Option<serde_json::Value>,
) -> ApiResult<()> {
    let (actor_member, device) = audit_ids(actor);
    AuditRepo::insert(
        tx,
        &AuditEntry {
            id: AuditEntryId::new(),
            household_id: household(actor)?,
            actor_member,
            device,
            action: action.into(),
            subject_type: subject_type.into(),
            subject_id,
            before: None,
            after,
            at: Utc::now(),
        },
    )
    .await?;
    Ok(())
}

fn member_view(m: &Member, version: i32, has_account: bool) -> serde_json::Value {
    let mut v = serde_json::to_value(MemberDto::from_member(m, has_account)).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.insert("version".into(), version.into());
    }
    v
}

async fn hash_blocking(secret: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || fa_auth::hash_password(&secret))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(Into::into)
}

fn parse_version(headers: &HeaderMap, body: Option<i32>) -> ApiResult<i32> {
    if let Some(h) = headers.get(IF_MATCH) {
        let raw = h.to_str().unwrap_or_default().trim();
        let raw = raw.strip_prefix("W/").unwrap_or(raw).trim_matches('"');
        return raw
            .parse()
            .map_err(|_| ApiError::BadRequest("If-Match must be the record version".into()));
    }
    body.ok_or_else(|| ApiError::PreconditionRequired("send the record version in If-Match".into()))
}

// ---- health ----

#[utoipa::path(get, path = "/healthz", tag = "health", responses((status = 200, description = "alive")))]
pub async fn healthz() -> &'static str {
    "ok"
}

#[utoipa::path(get, path = "/readyz", tag = "health", responses(
    (status = 200, description = "database reachable and migrations current"),
    (status = 503, description = "not ready")))]
pub async fn readyz(State(s): State<AppState>) -> Response {
    let ready = sqlx_ping(&s).await && s.store.migrations_applied().await.unwrap_or(false);
    if ready {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}

async fn sqlx_ping(s: &AppState) -> bool {
    match s.store.tx_system().await {
        Ok(tx) => tx.rollback().await.is_ok(),
        Err(_) => false,
    }
}

// ---- auth ----

#[derive(Deserialize, ToSchema)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResp {
    #[schema(value_type = String, format = Uuid)]
    pub member_id: MemberId,
    #[schema(value_type = String)]
    pub role: Role,
    pub expires_at: DateTime<Utc>,
}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", request_body = LoginReq,
    responses((status = 200, body = LoginResp, description = "sets the fa_session cookie"),
              (status = 401, body = schemas::Problem), (status = 423, body = schemas::Problem),
              (status = 429, body = schemas::Problem)))]
pub async fn login(
    State(s): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiJson(req): ApiJson<LoginReq>,
) -> ApiResult<Response> {
    let ua = headers.get(USER_AGENT).and_then(|v| v.to_str().ok());
    let out = s.auth.login(&req.username, &req.password, ip, ua).await?;
    let max_age = s.auth.config().session_ttl(out.role).as_secs();
    let mut res = Json(LoginResp {
        member_id: out.member_id,
        role: out.role,
        expires_at: out.expires_at,
    })
    .into_response();
    set_cookie(&mut res, &out.session_id, max_age, s.cfg.cookie_secure);
    Ok(res)
}

fn set_cookie(res: &mut Response, value: &str, max_age: u64, secure: bool) {
    let secure = if secure { "; Secure" } else { "" };
    let c = format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    );
    if let Ok(v) = HeaderValue::from_str(&c) {
        res.headers_mut().append(SET_COOKIE, v);
    }
}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth",
    security(("cookie" = [])), responses((status = 204), (status = 401, body = schemas::Problem)))]
pub async fn logout(State(s): State<AppState>, auth: Auth) -> ApiResult<Response> {
    if matches!(auth.actor, Actor::User { .. })
        && let Some(id) = &auth.session_id
    {
        s.auth.logout(id).await?;
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut res, "", 0, s.cfg.cookie_secure);
    Ok(res)
}

#[derive(Serialize, ToSchema)]
pub struct ActorView {
    /// `user`, `device` or `elevated_device`.
    pub kind: &'static str,
    #[schema(value_type = Option<Object>)]
    pub member: Option<serde_json::Value>,
    #[schema(value_type = Option<schemas::DeviceDto>)]
    pub device: Option<DeviceDto>,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub actor: ActorView,
    #[schema(value_type = schemas::HouseholdDto)]
    pub household: HouseholdDto,
}

#[utoipa::path(get, path = "/api/v1/me", tag = "auth", security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MeResponse), (status = 401, body = schemas::Problem)))]
pub async fn me(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<MeResponse>> {
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let view = match &auth.actor {
        Actor::User { member_id, .. } => {
            let m = MemberRepo::get(&mut tx, *member_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let has_account = UserAccountRepo::get_by_member(&mut tx, *member_id)
                .await?
                .is_some();
            ActorView {
                kind: "user",
                member: serde_json::to_value(MemberDto::from_member(&m.value, has_account)).ok(),
                device: None,
            }
        }
        Actor::Device { device_id, .. } | Actor::ElevatedDevice { device_id, .. } => {
            let d = DeviceRepo::get(&mut tx, *device_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let (kind, member) = match &auth.actor {
                Actor::ElevatedDevice { adult, .. } => {
                    let m = MemberRepo::get(&mut tx, *adult).await?;
                    (
                        "elevated_device",
                        m.and_then(|m| serde_json::to_value(MemberPublicDto::from(&m.value)).ok()),
                    )
                }
                _ => ("device", None),
            };
            ActorView {
                kind,
                member,
                device: Some(DeviceDto::from(&d.value)),
            }
        }
        Actor::System => return Err(ApiError::Forbidden("forbidden".into())),
    };
    tx.rollback().await?;
    Ok(Json(MeResponse {
        actor: view,
        household: HouseholdDto::from(&h.value),
    }))
}

// ---- members ----

#[utoipa::path(get, path = "/api/v1/members", tag = "members",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, description = "adults: full members; children and devices: public fields",
                body = Vec<schemas::MemberDto>),
              (status = 401, body = schemas::Problem)))]
pub async fn list_members(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<serde_json::Value>> {
    let hid = household(&auth.actor)?;
    let full = require(&auth.actor, Action::ReadMemberFull, &Resource::Household).is_ok();
    let mut tx = s.store.tx_for(hid).await?;
    let members = MemberRepo::list(&mut tx, false).await?;
    let out = if full {
        let accounts: std::collections::HashSet<MemberId> = UserAccountRepo::list(&mut tx)
            .await?
            .into_iter()
            .map(|a| a.value.member_id)
            .collect();
        members
            .iter()
            .map(|m| member_view(&m.value, m.version, accounts.contains(&m.value.id)))
            .collect::<Vec<_>>()
    } else {
        members
            .iter()
            .filter_map(|m| serde_json::to_value(MemberPublicDto::from(&m.value)).ok())
            .collect()
    };
    tx.rollback().await?;
    Ok(Json(serde_json::Value::Array(out)))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateMemberReq {
    pub display_name: String,
    #[schema(value_type = String)]
    pub role: Role,
    pub avatar: Option<String>,
    pub color: Option<String>,
    #[schema(value_type = Option<String>, format = Date)]
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub hobbies: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
}

fn clean_name(name: &str) -> ApiResult<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 100 {
        return Err(ApiError::Unprocessable(
            "display_name must be 1 to 100 characters".into(),
        ));
    }
    Ok(n.to_owned())
}

#[utoipa::path(post, path = "/api/v1/members", tag = "members", request_body = CreateMemberReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = schemas::MemberDto), (status = 403, body = schemas::Problem),
              (status = 422, body = schemas::Problem)))]
pub async fn create_member(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<CreateMemberReq>,
) -> ApiResult<Response> {
    require(&auth.actor, Action::ManageMember, &Resource::Household)?;
    let hid = household(&auth.actor)?;
    let now = Utc::now();
    let m = Member {
        id: MemberId::new(),
        household_id: hid,
        display_name: clean_name(&req.display_name)?,
        role: req.role,
        avatar: req.avatar,
        color: req.color,
        birthdate: req.birthdate,
        grade: req.grade,
        interests: req.interests,
        hobbies: req.hobbies,
        responsibilities: req.responsibilities,
        pin_hash: None,
        archived_at: None,
        created_at: now,
        updated_at: now,
    };
    let (by, _) = audit_ids(&auth.actor);
    let mut tx = s.store.tx_for(hid).await?;
    let stored = MemberRepo::insert(&mut tx, &m, None, by).await?;
    audit(
        &mut tx,
        &auth.actor,
        "member.create",
        "member",
        Some(m.id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    s.hub
        .publish(hid, "member.updated", serde_json::json!({ "id": m.id }));
    let body = member_view(&stored.value, stored.version, false);
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(stored.version))],
        Json(body),
    )
        .into_response())
}

fn etag(version: i32) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{version}\"")).unwrap_or(HeaderValue::from_static("\"0\""))
}

/// Distinguishes an absent field from an explicit `null` (which clears the value).
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Deserialize, ToSchema)]
pub struct PatchMemberReq {
    pub display_name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub avatar: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub color: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub birthdate: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub grade: Option<Option<String>>,
    pub interests: Option<Vec<String>>,
    pub hobbies: Option<Vec<String>>,
    pub responsibilities: Option<Vec<String>>,
    /// Alternative to the `If-Match` header.
    pub version: Option<i32>,
}

#[utoipa::path(patch, path = "/api/v1/members/{id}", tag = "members", request_body = PatchMemberReq,
    params(("id" = String, Path, description = "member id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = schemas::MemberDto), (status = 403, body = schemas::Problem),
              (status = 404, body = schemas::Problem), (status = 409, body = schemas::Problem),
              (status = 428, body = schemas::Problem)))]
pub async fn patch_member(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<PatchMemberReq>,
) -> ApiResult<Response> {
    let hid = household(&auth.actor)?;
    let expected = parse_version(&headers, req.version)?;
    let mut tx = s.store.tx_for(hid).await?;
    let current = MemberRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    require(
        &auth.actor,
        Action::ManageMember,
        &Resource::Member(id, current.value.role),
    )?;
    let mut m = current.value;
    if let Some(n) = req.display_name {
        m.display_name = clean_name(&n)?;
    }
    if let Some(v) = req.avatar {
        m.avatar = v;
    }
    if let Some(v) = req.color {
        m.color = v;
    }
    if let Some(v) = req.birthdate {
        m.birthdate = v;
    }
    if let Some(v) = req.grade {
        m.grade = v;
    }
    if let Some(v) = req.interests {
        m.interests = v;
    }
    if let Some(v) = req.hobbies {
        m.hobbies = v;
    }
    if let Some(v) = req.responsibilities {
        m.responsibilities = v;
    }
    let version = MemberRepo::update(&mut tx, &m, expected).await?;
    audit(
        &mut tx,
        &auth.actor,
        "member.update",
        "member",
        Some(id.to_string()),
        None,
    )
    .await?;
    let has_account = UserAccountRepo::get_by_member(&mut tx, id).await?.is_some();
    tx.commit().await?;
    s.hub
        .publish(hid, "member.updated", serde_json::json!({ "id": id }));
    Ok((
        [(ETAG, etag(version))],
        Json(member_view(&m, version, has_account)),
    )
        .into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccountReq {
    pub username: String,
    pub password: String,
}

#[utoipa::path(post, path = "/api/v1/members/{id}/account", tag = "members",
    request_body = CreateAccountReq, params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = schemas::UserAccountDto), (status = 403, body = schemas::Problem),
              (status = 409, body = schemas::Problem), (status = 422, body = schemas::Problem)))]
pub async fn create_account(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    ApiJson(req): ApiJson<CreateAccountReq>,
) -> ApiResult<Response> {
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let target = MemberRepo::get(&mut tx, id)
        .await?
        .map(|m| m.value)
        .filter(|m| m.archived_at.is_none())
        .ok_or(ApiError::NotFound)?;
    let resource = Resource::Member(id, target.role);
    match target.role {
        Role::Child => require(&auth.actor, Action::ManageChildAccount, &resource)?,
        Role::Adult => require(&auth.actor, Action::ManageMember, &resource)?,
    }
    let username = req.username.trim().to_owned();
    if username.is_empty() || username.chars().count() > 64 {
        return Err(ApiError::Unprocessable(
            "username must be 1 to 64 characters".into(),
        ));
    }
    validate_password(target.role, &req.password)
        .map_err(|e| ApiError::Unprocessable(e.to_string()))?;
    let password_hash = hash_blocking(req.password).await?;
    let account = UserAccount {
        id: UserAccountId::new(),
        household_id: hid,
        member_id: id,
        username,
        password_hash,
        disabled_at: None,
        created_at: Utc::now(),
    };
    let stored = UserAccountRepo::insert(&mut tx, &account).await?;
    audit(
        &mut tx,
        &auth.actor,
        "account.create",
        "user_account",
        Some(account.id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    s.hub
        .publish(hid, "member.updated", serde_json::json!({ "id": id }));
    Ok((
        StatusCode::CREATED,
        Json(UserAccountDto::from(&stored.value)),
    )
        .into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct PasswordReq {
    pub password: String,
}

#[utoipa::path(post, path = "/api/v1/members/{id}/account/password", tag = "members",
    request_body = PasswordReq, params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = schemas::Problem), (status = 404, body = schemas::Problem),
              (status = 422, body = schemas::Problem)))]
pub async fn reset_password(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    ApiJson(req): ApiJson<PasswordReq>,
) -> ApiResult<StatusCode> {
    let hid = household(&auth.actor)?;
    let own = matches!(&auth.actor, Actor::User { member_id, .. } if *member_id == id);
    let mut tx = s.store.tx_for(hid).await?;
    let target = MemberRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    if !own {
        require(
            &auth.actor,
            Action::ManageChildAccount,
            &Resource::Member(id, target.role),
        )?;
    }
    validate_password(target.role, &req.password)
        .map_err(|e| ApiError::Unprocessable(e.to_string()))?;
    let account = UserAccountRepo::get_by_member(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let hash = hash_blocking(req.password).await?;
    UserAccountRepo::update_password(&mut tx, account.value.id, &hash, account.version).await?;
    if !own {
        SessionRepo::delete_for_member(&mut tx, id).await?;
    }
    audit(
        &mut tx,
        &auth.actor,
        "account.reset_password",
        "user_account",
        Some(account.value.id.to_string()),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, ToSchema)]
pub struct PinReq {
    pub pin: String,
}

#[utoipa::path(put, path = "/api/v1/members/{id}/pin", tag = "members", request_body = PinReq,
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = schemas::Problem), (status = 422, body = schemas::Problem)))]
pub async fn set_pin(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    ApiJson(req): ApiJson<PinReq>,
) -> ApiResult<StatusCode> {
    s.auth.set_pin(&auth.actor, id, &req.pin).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- devices ----

#[utoipa::path(get, path = "/api/v1/devices", tag = "devices", security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<schemas::DeviceDto>), (status = 403, body = schemas::Problem)))]
pub async fn list_devices(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<DeviceDto>>> {
    require(&auth.actor, Action::ManageDevices, &Resource::Household)?;
    let mut tx = s.store.tx_for(household(&auth.actor)?).await?;
    let devices = DeviceRepo::list(&mut tx, false).await?;
    tx.rollback().await?;
    Ok(Json(
        devices.iter().map(|d| DeviceDto::from(&d.value)).collect(),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PairDeviceReq {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct PairDeviceResp {
    #[schema(value_type = String, format = Uuid)]
    pub id: DeviceId,
    /// Shown once.
    pub token: String,
    pub pair_url: String,
}

#[utoipa::path(post, path = "/api/v1/devices", tag = "devices", request_body = PairDeviceReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = PairDeviceResp), (status = 403, body = schemas::Problem)))]
pub async fn pair_device(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<PairDeviceReq>,
) -> ApiResult<Response> {
    if req.name.trim().is_empty() || req.name.chars().count() > 100 {
        return Err(ApiError::Unprocessable(
            "name must be 1 to 100 characters".into(),
        ));
    }
    let paired = s.auth.pair_device(&auth.actor, &req.name).await?;
    Ok((
        StatusCode::CREATED,
        Json(PairDeviceResp {
            id: paired.device.id,
            token: paired.token,
            pair_url: paired.pair_url,
        }),
    )
        .into_response())
}

#[utoipa::path(delete, path = "/api/v1/devices/{id}", tag = "devices",
    params(("id" = String, Path, description = "device id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = schemas::Problem), (status = 404, body = schemas::Problem)))]
pub async fn revoke_device(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<DeviceId>,
) -> ApiResult<StatusCode> {
    s.auth.revoke_device(&auth.actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, ToSchema)]
pub struct ElevateReq {
    #[schema(value_type = String, format = Uuid)]
    pub member_id: MemberId,
    pub pin: String,
}

#[derive(Serialize, ToSchema)]
pub struct ElevateResp {
    pub expires_at: DateTime<Utc>,
}

#[utoipa::path(post, path = "/api/v1/device/elevate", tag = "devices", request_body = ElevateReq,
    security(("bearer" = [])),
    responses((status = 200, body = ElevateResp), (status = 401, body = schemas::Problem),
              (status = 403, body = schemas::Problem), (status = 423, body = schemas::Problem)))]
pub async fn elevate(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<ElevateReq>,
) -> ApiResult<Json<ElevateResp>> {
    let (Actor::Device {
        device_id,
        household_id,
    }
    | Actor::ElevatedDevice {
        device_id,
        household_id,
        ..
    }) = auth.actor
    else {
        return Err(ApiError::Forbidden("only devices can elevate".into()));
    };
    let mut tx = s.store.tx_for(household_id).await?;
    let device = DeviceRepo::get(&mut tx, device_id)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    tx.rollback().await?;
    let e = s.auth.elevate(&device, req.member_id, &req.pin).await?;
    Ok(Json(ElevateResp {
        expires_at: e.expires_at,
    }))
}

// ---- sessions ----

#[derive(Deserialize)]
pub struct SessionsQuery {
    member: Option<MemberId>,
}

#[derive(Serialize, ToSchema)]
pub struct SessionView {
    /// One-way handle, not the cookie value.
    pub id: String,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

fn session_owner(actor: &Actor, q: Option<MemberId>) -> ApiResult<MemberId> {
    match (q, actor) {
        (Some(m), _) => Ok(m),
        (None, Actor::User { member_id, .. }) => Ok(*member_id),
        _ => Err(ApiError::BadRequest(
            "member query parameter required".into(),
        )),
    }
}

#[utoipa::path(get, path = "/api/v1/sessions", tag = "sessions",
    params(("member" = Option<String>, Query, description = "member id; defaults to yourself")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<SessionView>), (status = 403, body = schemas::Problem)))]
pub async fn list_sessions(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<SessionsQuery>,
) -> ApiResult<Json<Vec<SessionView>>> {
    let owner = session_owner(&auth.actor, q.member)?;
    let list = s.auth.list_sessions(&auth.actor, owner).await?;
    Ok(Json(
        list.into_iter()
            .map(|i| SessionView {
                id: i.handle,
                user_agent: i.user_agent,
                created_at: i.created_at,
                last_seen: i.last_seen,
                expires_at: i.expires_at,
            })
            .collect(),
    ))
}

#[utoipa::path(delete, path = "/api/v1/sessions/{id}", tag = "sessions",
    params(("id" = String, Path, description = "session handle"),
           ("member" = Option<String>, Query, description = "owner member id; defaults to yourself")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = schemas::Problem), (status = 404, body = schemas::Problem)))]
pub async fn revoke_session(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(handle): ApiPath<String>,
    ApiQuery(q): ApiQuery<SessionsQuery>,
) -> ApiResult<StatusCode> {
    let owner = session_owner(&auth.actor, q.member)?;
    s.auth.revoke_session(&auth.actor, owner, &handle).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- stream ----

fn to_event(e: &HubEvent) -> Event {
    Event::default()
        .id(e.id.to_string())
        .event(e.kind.clone())
        .data(e.data.to_string())
}

#[utoipa::path(get, path = "/api/v1/stream", tag = "stream", security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, content_type = "text/event-stream",
               description = "`heartbeat` every 15 s and `member.updated`; honours Last-Event-ID")))]
pub async fn stream(
    State(s): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let hid = household(&auth.actor)?;
    let last = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok());
    let (replay, mut rx) = s.hub.subscribe(hid, last);
    let shutdown = s.shutdown.clone();
    let period = s.cfg.heartbeat_interval;
    let stream = async_stream::stream! {
        for e in &replay {
            yield Ok(to_event(e));
        }
        let mut tick = tokio::time::interval(period);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = tick.tick() => {
                    yield Ok(Event::default().event("heartbeat").data(Utc::now().to_rfc3339()));
                }
                r = rx.recv() => match r {
                    Ok(e) if e.household == hid => yield Ok(to_event(&e)),
                    Ok(_) | Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
            }
        }
    };
    Ok(Sse::new(stream))
}

#[utoipa::path(get, path = "/api/v1/openapi.json", tag = "health",
    responses((status = 200, description = "this document")))]
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::openapi())
}
