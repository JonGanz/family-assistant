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
    Member, MemberDto, MemberId, MemberPublicDto, MemberView, Resource, Role, UserAccount,
    UserAccountDto, UserAccountId, system_api::OwnPasswordChange, validate_password,
};
use fa_store::{
    AuditRepo, ChangeBus, DeviceRepo, HouseholdRepo, MemberAvatarRepo, MemberRepo, SessionRepo, Tx,
    UserAccountRepo,
};
use futures::Stream;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::broadcast::error::RecvError;
use utoipa::ToSchema;

use crate::conn::Conn;
use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery, ClientIp};
use crate::hub::HubEvent;
use crate::{ApiError, AppState, Auth, require};

pub(crate) type ApiResult<T> = Result<T, ApiError>;

pub(crate) fn household(actor: &Actor) -> ApiResult<HouseholdId> {
    match actor {
        Actor::User { household_id, .. }
        | Actor::Device { household_id, .. }
        | Actor::ElevatedDevice { household_id, .. } => Ok(*household_id),
        Actor::System => Err(ApiError::Forbidden("forbidden".into())),
    }
}

/// The acting member (the adult behind an elevated device) and device, for the audit trail.
pub(crate) fn audit_ids(actor: &Actor) -> (Option<MemberId>, Option<DeviceId>) {
    match actor {
        Actor::User { member_id, .. } => (Some(*member_id), None),
        Actor::ElevatedDevice {
            adult, device_id, ..
        } => (Some(*adult), Some(*device_id)),
        Actor::Device { device_id, .. } => (None, Some(*device_id)),
        Actor::System => (None, None),
    }
}

pub(crate) async fn audit(
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

/// `/api/v1/members/{id}/avatar?v=<sha256 prefix>`, the cache-busting URL clients render.
pub(crate) fn avatar_url(id: MemberId, sha256: &str) -> String {
    let v: String = sha256.chars().take(12).collect();
    format!("/api/v1/members/{id}/avatar?v={v}")
}

pub(crate) async fn avatar_urls(
    tx: &mut Tx,
) -> ApiResult<std::collections::HashMap<MemberId, String>> {
    Ok(MemberAvatarRepo::hashes(tx)
        .await?
        .into_iter()
        .map(|(m, sha)| (m, avatar_url(m, &sha)))
        .collect())
}

/// The authorization resource for acting on a member: an adult other than the actor who has not
/// claimed their account (no account, or `initial_password`) is `UnclaimedAdult` (Q-077).
pub(crate) async fn member_resource(
    conn: &mut sqlx::PgConnection,
    actor: &Actor,
    id: MemberId,
    role: Role,
) -> ApiResult<Resource> {
    let own = matches!(actor, Actor::User { member_id, .. } if *member_id == id);
    if role == Role::Adult && !own {
        let unclaimed = UserAccountRepo::get_by_member(conn, id)
            .await?
            .is_none_or(|a| a.value.initial_password);
        if unclaimed {
            return Ok(Resource::UnclaimedAdult(id));
        }
    }
    Ok(Resource::Member(id, role))
}

/// True when the member's account still has the password another adult chose.
pub(crate) async fn must_change_password(
    s: &AppState,
    household: HouseholdId,
    member: MemberId,
) -> ApiResult<bool> {
    let mut tx = s.store.tx_for(household).await?;
    let flag = UserAccountRepo::get_by_member(&mut tx, member)
        .await?
        .is_some_and(|a| a.value.initial_password);
    tx.rollback().await?;
    Ok(flag)
}

fn member_view(
    m: &Member,
    version: i32,
    account: Option<bool>,
    avatar_url: Option<String>,
) -> serde_json::Value {
    let mut dto = MemberDto::from_member(m, account.is_some());
    dto.initial_password = account == Some(true);
    dto.avatar_url = avatar_url;
    dto.version = version;
    serde_json::to_value(dto).unwrap_or_default()
}

async fn hash_blocking(secret: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || fa_auth::hash_password(&secret))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(Into::into)
}

pub(crate) fn parse_version(headers: &HeaderMap, body: Option<i32>) -> ApiResult<i32> {
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
    /// True while the password was set by another adult; the client sends the user to the
    /// Account page to choose their own.
    pub must_change_password: bool,
}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", request_body = LoginReq,
    responses((status = 200, body = LoginResp, description = "sets the fa_session cookie"),
              (status = 401, body = Problem), (status = 423, body = Problem),
              (status = 429, body = Problem)))]
pub async fn login(
    State(s): State<AppState>,
    ClientIp(ip): ClientIp,
    conn: Conn,
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
        must_change_password: must_change_password(&s, out.household_id, out.member_id).await?,
    })
    .into_response();
    set_cookie(&mut res, &conn, &s.cfg, &out.session_id, max_age);
    Ok(res)
}

pub(crate) fn set_cookie(
    res: &mut Response,
    conn: &Conn,
    cfg: &crate::ApiConfig,
    value: &str,
    max_age: u64,
) {
    if let Ok(v) = HeaderValue::from_str(&conn.session_cookie(cfg, value, max_age)) {
        res.headers_mut().append(SET_COOKIE, v);
    }
}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth",
    security(("cookie" = [])), responses((status = 204), (status = 401, body = Problem)))]
pub async fn logout(State(s): State<AppState>, conn: Conn, auth: Auth) -> ApiResult<Response> {
    if matches!(auth.actor, Actor::User { .. })
        && let Some(id) = &auth.session_id
    {
        s.auth.logout(id).await?;
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut res, &conn, &s.cfg, "", 0);
    Ok(res)
}

#[derive(Serialize, ToSchema)]
pub struct ActorView {
    /// `user`, `device` or `elevated_device`.
    pub kind: &'static str,
    #[schema(value_type = Option<MemberView>)]
    pub member: Option<serde_json::Value>,
    #[schema(value_type = Option<DeviceDto>)]
    pub device: Option<DeviceDto>,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub actor: ActorView,
    #[schema(value_type = HouseholdDto)]
    pub household: HouseholdDto,
    /// True while the signed-in account still has the password another adult set.
    pub must_change_password: bool,
}

#[utoipa::path(get, path = "/api/v1/me", tag = "auth", security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MeResponse), (status = 401, body = Problem)))]
pub async fn me(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<MeResponse>> {
    Ok(Json(me_for(&s, &auth.actor).await?))
}

pub(crate) async fn me_for(s: &AppState, actor: &Actor) -> ApiResult<MeResponse> {
    let hid = household(actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?;
    let mut must_change = false;
    let view = match actor {
        Actor::User { member_id, .. } => {
            let m = MemberRepo::get(&mut tx, *member_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let account = UserAccountRepo::get_by_member(&mut tx, *member_id).await?;
            must_change = account.as_ref().is_some_and(|a| a.value.initial_password);
            let has_account = account.is_some();
            let mut dto = MemberDto::from_member(&m.value, has_account);
            dto.avatar_url = avatar_urls(&mut tx).await?.remove(member_id);
            ActorView {
                kind: "user",
                member: serde_json::to_value(dto).ok(),
                device: None,
            }
        }
        Actor::Device { device_id, .. } | Actor::ElevatedDevice { device_id, .. } => {
            let d = DeviceRepo::get(&mut tx, *device_id)
                .await?
                .ok_or(ApiError::NotFound)?;
            let (kind, member) = match actor {
                Actor::ElevatedDevice { adult, .. } => {
                    let m = MemberRepo::get(&mut tx, *adult).await?;
                    let mut urls = avatar_urls(&mut tx).await?;
                    (
                        "elevated_device",
                        m.and_then(|m| {
                            let mut dto = MemberPublicDto::from(&m.value);
                            dto.avatar_url = urls.remove(&m.value.id);
                            serde_json::to_value(dto).ok()
                        }),
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
    Ok(MeResponse {
        actor: view,
        household: HouseholdDto::from(&h.value),
        must_change_password: must_change,
    })
}

// ---- members ----

#[utoipa::path(get, path = "/api/v1/members", tag = "members",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, description = "adults: full members; children and devices: public fields",
                body = Vec<MemberView>),
              (status = 401, body = Problem)))]
pub async fn list_members(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<serde_json::Value>> {
    let hid = household(&auth.actor)?;
    let full = require(&auth.actor, Action::ReadMemberFull, &Resource::Household).is_ok();
    let mut tx = s.store.tx_for(hid).await?;
    let members = MemberRepo::list(&mut tx, false).await?;
    let urls = avatar_urls(&mut tx).await?;
    let out = if full {
        let accounts: std::collections::HashMap<MemberId, bool> = UserAccountRepo::list(&mut tx)
            .await?
            .into_iter()
            .map(|a| (a.value.member_id, a.value.initial_password))
            .collect();
        members
            .iter()
            .map(|m| {
                member_view(
                    &m.value,
                    m.version,
                    accounts.get(&m.value.id).copied(),
                    urls.get(&m.value.id).cloned(),
                )
            })
            .collect::<Vec<_>>()
    } else {
        members
            .iter()
            .filter_map(|m| {
                let mut dto = MemberPublicDto::from(&m.value);
                dto.avatar_url = urls.get(&m.value.id).cloned();
                serde_json::to_value(dto).ok()
            })
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
    pub color: Option<fa_domain::theme::Hue>,
    #[schema(value_type = Option<String>, format = Date)]
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub hobbies: Vec<String>,
    #[serde(default)]
    pub responsibilities: Vec<String>,
    /// Nicknames matched in mail (M3).
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Optional; used only for mail relevance (D-055).
    #[schema(value_type = Option<String>)]
    pub gender: Option<fa_domain::Gender>,
    /// Free-form relevance tags (D-055).
    #[serde(default)]
    pub relevance_tags: Vec<String>,
}

/// Trimmed, non-empty, de-duplicated (case-insensitive), first spelling kept.
fn clean_list(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        let s = s.trim().to_string();
        if !s.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&s)) {
            out.push(s);
        }
    }
    out
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
    responses((status = 201, body = MemberView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
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
        aliases: clean_list(req.aliases),
        gender: req.gender,
        relevance_tags: clean_list(req.relevance_tags),
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
    ChangeBus::publish(
        &mut tx,
        hid,
        "member.updated",
        &serde_json::json!({ "id": m.id }),
    )
    .await?;
    tx.commit().await?;
    let body = member_view(&stored.value, stored.version, None, None);
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(stored.version))],
        Json(body),
    )
        .into_response())
}

pub(crate) fn etag(version: i32) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{version}\"")).unwrap_or(HeaderValue::from_static("\"0\""))
}

/// Distinguishes an absent field from an explicit `null` (which clears the value).
pub(crate) fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
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
    #[schema(value_type = Option<fa_domain::theme::Hue>)]
    pub color: Option<Option<fa_domain::theme::Hue>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, format = Date)]
    pub birthdate: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub grade: Option<Option<String>>,
    pub interests: Option<Vec<String>>,
    pub hobbies: Option<Vec<String>>,
    pub responsibilities: Option<Vec<String>>,
    pub aliases: Option<Vec<String>>,
    /// `null` clears it.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub gender: Option<Option<fa_domain::Gender>>,
    pub relevance_tags: Option<Vec<String>>,
    /// Alternative to the `If-Match` header.
    pub version: Option<i32>,
}

#[utoipa::path(patch, path = "/api/v1/members/{id}", tag = "members", request_body = PatchMemberReq,
    params(("id" = String, Path, description = "member id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MemberView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 428, body = Problem)))]
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
    let resource = member_resource(&mut tx, &auth.actor, id, current.value.role).await?;
    require(&auth.actor, Action::ManageMember, &resource)?;
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
    if let Some(v) = req.aliases {
        m.aliases = clean_list(v);
    }
    if let Some(v) = req.gender {
        m.gender = v;
    }
    if let Some(v) = req.relevance_tags {
        m.relevance_tags = clean_list(v);
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
    let has_account = UserAccountRepo::get_by_member(&mut tx, id)
        .await?
        .map(|a| a.value.initial_password);
    let url = avatar_urls(&mut tx).await?.remove(&id);
    ChangeBus::publish(
        &mut tx,
        hid,
        "member.updated",
        &serde_json::json!({ "id": id }),
    )
    .await?;
    tx.commit().await?;
    Ok((
        [(ETAG, etag(version))],
        Json(member_view(&m, version, has_account, url)),
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
    responses((status = 201, body = UserAccountDto), (status = 403, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem)))]
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
    let resource = member_resource(&mut tx, &auth.actor, id, target.role).await?;
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
        initial_password: target.role == Role::Adult,
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
    ChangeBus::publish(
        &mut tx,
        hid,
        "member.updated",
        &serde_json::json!({ "id": id }),
    )
    .await?;
    tx.commit().await?;
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

/// An adult resetting a child's password sends `PasswordReq`; a member changing their own sends
/// `OwnPasswordChange` (C-03).
#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum PasswordBody {
    Own(OwnPasswordChange),
    Reset(PasswordReq),
}

#[utoipa::path(post, path = "/api/v1/members/{id}/account/password", tag = "members",
    request_body = PasswordBody, params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204, description = "an own change ends the member's other sessions"),
              (status = 401, body = Problem, description = "wrong current password"),
              (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 422, body = Problem), (status = 423, body = Problem),
              (status = 429, body = Problem)))]
pub async fn reset_password(
    State(s): State<AppState>,
    auth: Auth,
    ClientIp(ip): ClientIp,
    ApiPath(id): ApiPath<MemberId>,
    ApiJson(req): ApiJson<PasswordBody>,
) -> ApiResult<StatusCode> {
    let hid = household(&auth.actor)?;
    let own = matches!(&auth.actor, Actor::User { member_id, .. } if *member_id == id);
    if own {
        let PasswordBody::Own(change) = req else {
            return Err(ApiError::Unprocessable(
                "send current_password and new_password".into(),
            ));
        };
        s.auth
            .change_own_password(
                &auth.actor,
                &change.current_password,
                &change.new_password,
                auth.session_id.as_deref(),
                ip,
            )
            .await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let PasswordBody::Reset(req) = req else {
        return Err(ApiError::Unprocessable("send password".into()));
    };
    let mut tx = s.store.tx_for(hid).await?;
    let target = MemberRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    let resource = member_resource(&mut tx, &auth.actor, id, target.role).await?;
    match target.role {
        Role::Child => require(&auth.actor, Action::ManageChildAccount, &resource)?,
        Role::Adult => require(&auth.actor, Action::ManageMember, &resource)?,
    }
    validate_password(target.role, &req.password)
        .map_err(|e| ApiError::Unprocessable(e.to_string()))?;
    let account = UserAccountRepo::get_by_member(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let hash = hash_blocking(req.password).await?;
    UserAccountRepo::update_password(&mut tx, account.value.id, &hash, account.version).await?;
    if target.role == Role::Adult {
        UserAccountRepo::set_initial_password(&mut tx, account.value.id, true).await?;
    }
    SessionRepo::delete_for_member(&mut tx, id).await?;
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
    responses((status = 204), (status = 403, body = Problem), (status = 422, body = Problem)))]
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
    responses((status = 200, body = Vec<DeviceDto>), (status = 403, body = Problem)))]
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
    /// The same link on the home-network dashboard address, when that is on (D-064).
    pub lan_pair_url: Option<String>,
}

#[utoipa::path(post, path = "/api/v1/devices", tag = "devices", request_body = PairDeviceReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = PairDeviceResp), (status = 403, body = Problem)))]
pub async fn pair_device(
    State(s): State<AppState>,
    conn: Conn,
    auth: Auth,
    ApiJson(req): ApiJson<PairDeviceReq>,
) -> ApiResult<Response> {
    if req.name.trim().is_empty() || req.name.chars().count() > 100 {
        return Err(ApiError::Unprocessable(
            "name must be 1 to 100 characters".into(),
        ));
    }
    let paired = s.auth.pair_device(&auth.actor, &req.name).await?;
    // On a secure request the link uses the tailnet address, when one is set.
    let pair_url = match (&s.cfg.tailnet_base_url, conn.secure) {
        (Some(base), true) => format!(
            "{}/dashboard?device={}",
            base.trim_end_matches('/'),
            paired.token
        ),
        _ => paired.pair_url,
    };
    Ok((
        StatusCode::CREATED,
        Json(PairDeviceResp {
            id: paired.device.id,
            lan_pair_url: s.cfg.lan_base_url.as_deref().map(|b| {
                format!(
                    "{}/dashboard?device={}",
                    b.trim_end_matches('/'),
                    paired.token
                )
            }),
            token: paired.token,
            pair_url,
        }),
    )
        .into_response())
}

#[utoipa::path(delete, path = "/api/v1/devices/{id}", tag = "devices",
    params(("id" = String, Path, description = "device id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
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
    responses((status = 200, body = ElevateResp), (status = 401, body = Problem),
              (status = 403, body = Problem), (status = 423, body = Problem)))]
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
    responses((status = 200, body = Vec<SessionView>), (status = 403, body = Problem)))]
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
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
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
               description = "`heartbeat` every 15 s plus the change kinds (`member.updated`, `calendar.updated`, `weather.updated`, `advisory.updated`, `household.updated`, `device.updated`, `source.updated`, `tasks.updated` `{}`, `routines.updated` `{date}`, `mail.updated` `{source?}`, `inbox.updated` `{}`, `bills.updated` `{}`, `resync`), bridged from `fa_change`; honours Last-Event-ID")))]
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
                    Ok(_) => {}
                    Err(RecvError::Lagged(_)) => yield Ok(Event::default().event("resync").data("{}")),
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
