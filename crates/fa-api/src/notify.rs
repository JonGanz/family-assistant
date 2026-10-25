//! M2 notification-preference routes (plan/M2.md, HTTP API v1).

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::{IntoResponse, Response};
use fa_domain::{Action, Actor, Member, MemberId, NotifyPrefPatch, NotifyPrefView, Resource, Role};

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath};
use crate::routes::{ApiResult, etag, household, parse_version};
use crate::tasks::load_member;
use crate::{ApiError, AppState, Auth, require};

/// Authorizes `ManageNotifyPrefs` on the member and returns it.
async fn authorize(s: &AppState, actor: &Actor, id: MemberId) -> ApiResult<Member> {
    let m = load_member(s, household(actor)?, id).await?;
    require(
        actor,
        Action::ManageNotifyPrefs,
        &Resource::Member(m.id, m.role),
    )?;
    Ok(m)
}

/// The topic is a secret; only that member and adults see it.
fn redact(actor: &Actor, member: MemberId, mut v: NotifyPrefView) -> NotifyPrefView {
    let allowed = match actor {
        Actor::User {
            member_id, role, ..
        } => *member_id == member || *role == Role::Adult,
        _ => false,
    };
    if !allowed {
        v.topic = None;
        v.subscribe_url = None;
    }
    v
}

fn respond(actor: &Actor, member: MemberId, v: NotifyPrefView) -> Response {
    let v = redact(actor, member, v);
    ([(ETAG, etag(v.version))], Json(v)).into_response()
}

#[utoipa::path(get, path = "/api/v1/members/{id}/notify-prefs", tag = "notify",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = NotifyPrefView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn get_prefs(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
) -> ApiResult<Response> {
    authorize(&s, &auth.actor, id).await?;
    let v = s
        .services
        .notify
        .get_prefs(household(&auth.actor)?, id)
        .await?;
    Ok(respond(&auth.actor, id, v))
}

#[utoipa::path(put, path = "/api/v1/members/{id}/notify-prefs", tag = "notify", request_body = NotifyPrefPatch,
    params(("id" = String, Path, description = "member id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = NotifyPrefView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn put_prefs(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<NotifyPrefPatch>,
) -> ApiResult<Response> {
    authorize(&s, &auth.actor, id).await?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .notify
        .put_prefs(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(respond(&auth.actor, id, v))
}

#[utoipa::path(post, path = "/api/v1/members/{id}/notify-prefs/test", tag = "notify",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204, description = "published"), (status = 403, body = Problem),
              (status = 404, body = Problem),
              (status = 502, body = Problem, description = "ntfy rejected the message"),
              (status = 503, body = Problem, description = "ntfy is not configured")))]
pub async fn test_prefs(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
) -> ApiResult<StatusCode> {
    authorize(&s, &auth.actor, id).await?;
    s.services
        .notify
        .send_test(household(&auth.actor)?, id)
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}
