//! M3 routes: mail sources, messages, mail actions, labelling, organisations, rules, categories,
//! the approval inbox and bills (plan/M3.md, HTTP API v1 additions).
//!
//! Handlers stay thin: load what the policy needs, authorize with `fa_domain::authorize`
//! (`Resource::MailSource{owners, visibility}`), call the fa-mail / fa-calendar / fa-tasks services.
//! A message the viewer cannot read is a 404, so its existence does not leak.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::ETAG;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Duration, Utc};
use fa_domain::{
    Action, Actor, BillId, BillPatch, BillPatchStatus, BillStatus, BillView, CategoryId,
    CategoryInput, CategoryPatch, CategoryView, ContactApproval, ContactSuggestionId, EnrollmentId,
    EnrollmentInput, EnrollmentPatch, EnrollmentView, EvalSampleInput, EventApproval, EventId,
    EventOrigin, EventStatus, HouseholdId, ImapTestRequest, ImapTestResult, InboxKind, InboxView,
    LabelInput, LabelQueueItem, MailActionId, MailActionInput, MailActionView, MailRuleId,
    MailRuleInput, MailRulePatch, MailRuleView, MailSource, MailSourceId, MailSourcePatch,
    MailSourceView, MakeTaskInput, MemberId, Message, MessageDetail, MessageId, MessagePage,
    MessageQuery, NewMailSource, OrgContactId, OrgContactInput, OrgContactPatch, OrgContactView,
    OrgId, OrgInput, OrgPatch, OrgView, PolicyCtx, ProcessingState, Resource, Role,
    RuleTestRequest, RuleTestResult, SignalPatch, SignalView, TaskApproval, TaskId, TaskOrigin,
    TaskPatch, TaskStatus, authorize,
};
use fa_mail::actions::UNDO_DAYS;
use fa_mail::context::Viewer;
use fa_mail::inbox::InboxService;
use fa_mail::labelling::LabelService;
use fa_mail::setup::SetupService;
use fa_mail::sources::SourceService;
use fa_mail::{MailDeps, MailError, actions::ActionService};
use fa_store::{
    BillRepo, ChangeBus, EventRepo, MailActionRepo, MailSourceRepo, MessageRepo, TaskRepo,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::feeds::audit_after;
use crate::routes::{ApiResult, etag, household, parse_version};
use crate::tasks::opt_body;

/// A JSON body whose fields are all optional: an empty body means `{}`.
fn body_or_empty<T: serde::de::DeserializeOwned>(b: &Bytes) -> ApiResult<T> {
    match opt_body(b)? {
        Some(t) => Ok(t),
        None => serde_json::from_value(serde_json::json!({}))
            .map_err(|e| ApiError::Unprocessable(e.to_string())),
    }
}
use crate::{ApiError, AppState, Auth, require};

/// The fa-mail services the M3 routes call; built once by the server.
#[derive(Clone)]
pub struct MailServices {
    pub sources: Arc<SourceService>,
    pub actions: Arc<ActionService>,
    pub inbox: InboxService,
    pub labels: LabelService,
    pub setup: SetupService,
}

impl MailServices {
    pub fn new(deps: Arc<MailDeps>) -> Self {
        Self {
            sources: Arc::new(SourceService::new(deps.clone())),
            actions: Arc::new(ActionService::new(deps.clone())),
            inbox: InboxService::new(deps.store.clone()),
            labels: LabelService::new(deps.store.clone()),
            setup: SetupService::new(deps.store.clone()),
        }
    }
}

fn with_etag<T: serde::Serialize>(version: i32, v: T) -> Response {
    ([(ETAG, etag(version))], Json(v)).into_response()
}

fn viewer_of(actor: &Actor) -> ApiResult<Viewer> {
    match actor {
        Actor::User {
            member_id, role, ..
        } => Ok(Viewer {
            member: Some(*member_id),
            role: Some(*role),
            device: false,
            elevated: false,
        }),
        Actor::ElevatedDevice { adult, .. } => Ok(Viewer {
            member: Some(*adult),
            role: Some(Role::Adult),
            device: true,
            elevated: true,
        }),
        Actor::Device { .. } => Ok(Viewer {
            member: None,
            role: None,
            device: true,
            elevated: false,
        }),
        Actor::System => Err(ApiError::Forbidden("forbidden".into())),
    }
}

/// Signed-in adults only: devices (elevated or not) and children are refused.
fn require_adult(actor: &Actor) -> ApiResult<MemberId> {
    require(actor, Action::ReadMail, &Resource::Household)?;
    match actor {
        Actor::User { member_id, .. } => Ok(*member_id),
        _ => Err(ApiError::Forbidden("forbidden".into())),
    }
}

fn require_setup(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ManageMailSetup, &Resource::Household)
}

struct SourceCtx {
    source: MailSource,
    owners: Vec<MemberId>,
}

impl SourceCtx {
    fn resource(&self) -> Resource {
        Resource::MailSource {
            owners: self.owners.clone(),
            visibility: self.source.visibility,
        }
    }

    /// Reading is checked first and answers 404, so a hidden source is indistinguishable from none.
    fn can_read(&self, actor: &Actor) -> ApiResult<()> {
        authorize(
            actor,
            Action::ReadMail,
            &self.resource(),
            &PolicyCtx::default(),
        )
        .map_err(|_| ApiError::NotFound)
    }

    fn require(&self, actor: &Actor, action: Action) -> ApiResult<()> {
        require(actor, action, &self.resource())
    }
}

async fn load_source(s: &AppState, hid: HouseholdId, id: MailSourceId) -> ApiResult<SourceCtx> {
    let mut tx = s.store.tx_for(hid).await?;
    let src = MailSourceRepo::get(&mut tx, id)
        .await?
        .filter(|v| v.value.archived_at.is_none())
        .ok_or(ApiError::NotFound)?;
    let owners = MailSourceRepo::owners(&mut tx, id).await?;
    tx.rollback().await?;
    Ok(SourceCtx {
        source: src.value,
        owners,
    })
}

async fn load_message(
    s: &AppState,
    hid: HouseholdId,
    id: MessageId,
) -> ApiResult<(Message, SourceCtx)> {
    let msg = {
        let mut tx = s.store.tx_for(hid).await?;
        let m = MessageRepo::get(&mut tx, id).await?;
        tx.rollback().await?;
        m.ok_or(ApiError::NotFound)?
    };
    let src = load_source(s, hid, msg.source_id).await?;
    Ok((msg, src))
}

fn is_rule_error(e: MailError) -> ApiError {
    match e {
        MailError::Invalid(message) => {
            let m = message.as_str();
            let field = if m.starts_with("a rule needs a name") {
                "name"
            } else if m.starts_with("confidence") {
                "confidence"
            } else if m.starts_with("unknown category")
                || m.starts_with("the value")
                || m.starts_with("mail rules cannot")
            {
                "value"
            } else {
                "conditions"
            };
            ApiError::FieldInvalid {
                field: field.into(),
                position: None,
                message,
            }
        }
        other => other.into(),
    }
}

// ---- mail sources ----

#[utoipa::path(get, path = "/api/v1/mail-sources", tag = "mail",
    security(("cookie" = [])),
    responses((status = 200, body = Vec<MailSourceView>), (status = 403, body = Problem)))]
pub async fn list_mail_sources(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<MailSourceView>>> {
    let me = require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    Ok(Json(s.services.mail.sources.list(hid, me).await?))
}

#[utoipa::path(post, path = "/api/v1/mail-sources", tag = "mail", request_body = NewMailSource,
    security(("cookie" = [])),
    responses((status = 201, body = MailSourceView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn create_mail_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(new): ApiJson<NewMailSource>,
) -> ApiResult<Response> {
    let me = require_adult(&auth.actor)?;
    require(&auth.actor, Action::ManageMailSource, &Resource::Household)?;
    let hid = household(&auth.actor)?;
    let view = s.services.mail.sources.create(hid, new, me).await?;
    audit_after(
        &s,
        &auth.actor,
        "mail_source.create",
        "mail_source",
        view.id.to_string(),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(view.version))],
        Json(view),
    )
        .into_response())
}

#[utoipa::path(patch, path = "/api/v1/mail-sources/{id}", tag = "mail", request_body = MailSourcePatch,
    params(("id" = String, Path, description = "source id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = MailSourceView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_mail_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailSourceId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<MailSourcePatch>,
) -> ApiResult<Response> {
    let me = require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let ctx = load_source(&s, hid, id).await?;
    ctx.can_read(&auth.actor)?;
    ctx.require(&auth.actor, Action::ManageMailSource)?;
    let expected = parse_version(&headers, None)?;
    let view = s
        .services
        .mail
        .sources
        .update(hid, me, id, &patch, expected)
        .await?;
    audit_after(
        &s,
        &auth.actor,
        "mail_source.update",
        "mail_source",
        id.to_string(),
    )
    .await;
    Ok(with_etag(view.version, view))
}

#[utoipa::path(delete, path = "/api/v1/mail-sources/{id}", tag = "mail",
    params(("id" = String, Path, description = "source id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_mail_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailSourceId>,
) -> ApiResult<StatusCode> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let ctx = load_source(&s, hid, id).await?;
    ctx.can_read(&auth.actor)?;
    ctx.require(&auth.actor, Action::ManageMailSource)?;
    s.services.mail.sources.archive(hid, id).await?;
    audit_after(
        &s,
        &auth.actor,
        "mail_source.archive",
        "mail_source",
        id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/mail-sources/{id}/sync", tag = "mail",
    params(("id" = String, Path, description = "source id")),
    security(("cookie" = [])),
    responses((status = 202, description = "a sync job is queued"), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn sync_mail_source(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailSourceId>,
) -> ApiResult<StatusCode> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let ctx = load_source(&s, hid, id).await?;
    ctx.can_read(&auth.actor)?;
    ctx.require(&auth.actor, Action::ManageMailSource)?;
    s.services.mail.sources.enqueue_sync(hid, id).await?;
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(post, path = "/api/v1/mail-sources/imap/test", tag = "mail", request_body = ImapTestRequest,
    security(("cookie" = [])),
    responses((status = 200, body = ImapTestResult), (status = 403, body = Problem)))]
pub async fn test_imap(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<ImapTestRequest>,
) -> ApiResult<Json<ImapTestResult>> {
    require_adult(&auth.actor)?;
    Ok(Json(s.services.mail.sources.test_imap(req).await))
}

// ---- messages ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MessageListQuery {
    /// A `category.key`.
    category: Option<String>,
    #[param(value_type = Option<String>)]
    member: Option<MemberId>,
    #[param(value_type = Option<String>)]
    source: Option<MailSourceId>,
    state: Option<ProcessingState>,
    /// Free text over sender and subject.
    q: Option<String>,
    /// Include categories whose behaviour is `ignore`.
    include_ignored: Option<bool>,
    /// RFC 3339 cursor: the `next_before` of the previous page.
    before: Option<DateTime<Utc>>,
    /// At most 100, default 50.
    limit: Option<u32>,
}

#[utoipa::path(get, path = "/api/v1/messages", tag = "mail", params(MessageListQuery),
    security(("cookie" = [])),
    responses((status = 200, body = MessagePage), (status = 403, body = Problem)))]
pub async fn list_messages(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<MessageListQuery>,
) -> ApiResult<Json<MessagePage>> {
    require_adult(&auth.actor)?;
    let page = s
        .services
        .mail
        .setup
        .list_messages(
            household(&auth.actor)?,
            viewer_of(&auth.actor)?,
            MessageQuery {
                category: q.category,
                member: q.member,
                source: q.source,
                state: q.state,
                q: q.q,
                include_ignored: q.include_ignored,
                before: q.before,
                limit: q.limit,
            },
        )
        .await?;
    Ok(Json(page))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberQuery {
    /// Fills the relevance of each section for this member.
    #[param(value_type = Option<String>)]
    member: Option<MemberId>,
}

#[utoipa::path(get, path = "/api/v1/messages/{id}", tag = "mail", params(MemberQuery, ("id" = String, Path, description = "message id")),
    security(("cookie" = [])),
    responses((status = 200, body = MessageDetail), (status = 404, body = Problem)))]
pub async fn get_message(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MessageId>,
    ApiQuery(q): ApiQuery<MemberQuery>,
) -> ApiResult<Json<MessageDetail>> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let (_, src) = load_message(&s, hid, id).await?;
    src.can_read(&auth.actor)?;
    let d = s
        .services
        .mail
        .setup
        .message_detail(hid, viewer_of(&auth.actor)?, id, q.member)
        .await?;
    Ok(Json(d))
}

#[utoipa::path(post, path = "/api/v1/messages/{id}/labels", tag = "mail", request_body = LabelInput,
    params(("id" = String, Path, description = "message id")),
    security(("cookie" = [])),
    responses((status = 200, body = MessageDetail), (status = 404, body = Problem),
              (status = 422, body = Problem)))]
pub async fn label_message(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MessageId>,
    ApiJson(input): ApiJson<LabelInput>,
) -> ApiResult<Json<MessageDetail>> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let (_, src) = load_message(&s, hid, id).await?;
    src.can_read(&auth.actor)?;
    let d = s
        .services
        .mail
        .labels
        .label(hid, viewer_of(&auth.actor)?, id, input)
        .await?;
    Ok(Json(d))
}

#[utoipa::path(post, path = "/api/v1/messages/{id}/eval-sample", tag = "mail", request_body = EvalSampleInput,
    params(("id" = String, Path, description = "message id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 404, body = Problem)))]
pub async fn set_eval_sample(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MessageId>,
    ApiJson(input): ApiJson<EvalSampleInput>,
) -> ApiResult<StatusCode> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let (_, src) = load_message(&s, hid, id).await?;
    src.can_read(&auth.actor)?;
    s.services
        .mail
        .labels
        .set_eval_sample(hid, viewer_of(&auth.actor)?, id, input.in_sample)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/messages/{id}/actions", tag = "mail", request_body = MailActionInput,
    params(("id" = String, Path, description = "message id")),
    security(("cookie" = [])),
    responses((status = 200, body = MailActionView), (status = 403, body = Problem),
              (status = 404, body = Problem),
              (status = 409, body = Problem, description = "the source is read-only or needs reconnecting"),
              (status = 422, body = Problem)))]
pub async fn message_action(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MessageId>,
    ApiJson(input): ApiJson<MailActionInput>,
) -> ApiResult<Json<MailActionView>> {
    let me = require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let (_, src) = load_message(&s, hid, id).await?;
    src.can_read(&auth.actor)?;
    src.require(&auth.actor, Action::GmailAction)?;
    if src.source.read_only {
        return Err(ApiError::Conflict("the source is read-only".into()));
    }
    let kind = input.action;
    let view = s
        .services
        .mail
        .actions
        .apply(hid, id, input, Some(me), false)
        .await?;
    audit_after(
        &s,
        &auth.actor,
        &format!("mail.action.{}", kind.as_str()),
        "message",
        id.to_string(),
    )
    .await;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/api/v1/mail-actions/{id}/undo", tag = "mail",
    params(("id" = String, Path, description = "mail action id")),
    security(("cookie" = [])),
    responses((status = 200, body = MailActionView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 410, body = Problem, description = "more than 30 days ago")))]
pub async fn undo_mail_action(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailActionId>,
) -> ApiResult<Json<MailActionView>> {
    let me = require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let action = {
        let mut tx = s.store.tx_for(hid).await?;
        let a = MailActionRepo::get(&mut tx, id).await?;
        tx.rollback().await?;
        a.ok_or(ApiError::NotFound)?
    };
    let (_, src) = load_message(&s, hid, action.message_id).await?;
    src.can_read(&auth.actor)?;
    src.require(&auth.actor, Action::GmailAction)?;
    if action.undone_at.is_some() {
        return Err(ApiError::Conflict("the action was already undone".into()));
    }
    if Utc::now() > action.done_at + Duration::days(UNDO_DAYS) {
        return Err(ApiError::Gone(format!(
            "an action can be undone for {UNDO_DAYS} days"
        )));
    }
    let view = s.services.mail.actions.undo(hid, id, Some(me)).await?;
    audit_after(&s, &auth.actor, "mail.undo", "mail_action", id.to_string()).await;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/api/v1/messages/{id}/task", tag = "mail", request_body = MakeTaskInput,
    params(("id" = String, Path, description = "message id")),
    security(("cookie" = [])),
    responses((status = 202, description = "a task is queued"), (status = 404, body = Problem)))]
pub async fn message_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MessageId>,
    body: Bytes,
) -> ApiResult<StatusCode> {
    require_adult(&auth.actor)?;
    let input: MakeTaskInput = body_or_empty(&body)?;
    let hid = household(&auth.actor)?;
    let (_, src) = load_message(&s, hid, id).await?;
    src.can_read(&auth.actor)?;
    s.services
        .mail
        .setup
        .make_task(hid, viewer_of(&auth.actor)?, id, input)
        .await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LabelNextQuery {
    /// Skip this message (the one just skipped or labelled).
    #[param(value_type = Option<String>)]
    after: Option<MessageId>,
}

#[utoipa::path(get, path = "/api/v1/labelling/next", tag = "mail", params(LabelNextQuery),
    security(("cookie" = [])),
    responses((status = 200, body = LabelQueueItem), (status = 204, description = "the queue is empty")))]
pub async fn labelling_next(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<LabelNextQuery>,
) -> ApiResult<Response> {
    require_adult(&auth.actor)?;
    let item = s
        .services
        .mail
        .labels
        .next(household(&auth.actor)?, viewer_of(&auth.actor)?, q.after)
        .await?;
    Ok(match item {
        Some(i) => Json(i).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}

// ---- organisations, contacts, enrollments ----

#[utoipa::path(get, path = "/api/v1/orgs", tag = "mail",
    security(("cookie" = [])),
    responses((status = 200, body = Vec<OrgView>), (status = 403, body = Problem)))]
pub async fn list_orgs(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<Vec<OrgView>>> {
    require_adult(&auth.actor)?;
    Ok(Json(
        s.services
            .mail
            .setup
            .list_orgs(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/orgs", tag = "mail", request_body = OrgInput,
    security(("cookie" = [])),
    responses((status = 201, body = OrgView), (status = 403, body = Problem), (status = 422, body = Problem)))]
pub async fn create_org(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<OrgInput>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .create_org(household(&auth.actor)?, input)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/orgs/{id}", tag = "mail", request_body = OrgPatch,
    params(("id" = String, Path, description = "org id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = OrgView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_org(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<OrgId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<OrgPatch>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .mail
        .setup
        .update_org(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/orgs/{id}", tag = "mail",
    params(("id" = String, Path, description = "org id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_org(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<OrgId>,
) -> ApiResult<StatusCode> {
    require_setup(&auth.actor)?;
    s.services
        .mail
        .setup
        .delete_org(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/orgs/{id}/contacts", tag = "mail", request_body = OrgContactInput,
    params(("id" = String, Path, description = "org id")),
    security(("cookie" = [])),
    responses((status = 201, body = OrgContactView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn add_org_contact(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<OrgId>,
    ApiJson(input): ApiJson<OrgContactInput>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .add_contact(household(&auth.actor)?, id, input)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/org-contacts/{id}", tag = "mail", request_body = OrgContactPatch,
    params(("id" = String, Path, description = "contact id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = OrgContactView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_org_contact(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<OrgContactId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<OrgContactPatch>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .mail
        .setup
        .update_contact(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/org-contacts/{id}", tag = "mail",
    params(("id" = String, Path, description = "contact id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_org_contact(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<OrgContactId>,
) -> ApiResult<StatusCode> {
    require_setup(&auth.actor)?;
    s.services
        .mail
        .setup
        .delete_contact(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/members/{id}/enrollments", tag = "mail",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = [])),
    responses((status = 200, body = Vec<EnrollmentView>), (status = 403, body = Problem)))]
pub async fn list_enrollments(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(member): ApiPath<MemberId>,
) -> ApiResult<Json<Vec<EnrollmentView>>> {
    require_adult(&auth.actor)?;
    Ok(Json(
        s.services
            .mail
            .setup
            .list_enrollments(household(&auth.actor)?, Some(member))
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/members/{id}/enrollments", tag = "mail", request_body = EnrollmentInput,
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = [])),
    responses((status = 201, body = EnrollmentView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn create_enrollment(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(member): ApiPath<MemberId>,
    ApiJson(input): ApiJson<EnrollmentInput>,
) -> ApiResult<Response> {
    require_adult(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .create_enrollment(household(&auth.actor)?, member, input)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/enrollments/{id}", tag = "mail", request_body = EnrollmentPatch,
    params(("id" = String, Path, description = "enrollment id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = EnrollmentView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_enrollment(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<EnrollmentId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<EnrollmentPatch>,
) -> ApiResult<Response> {
    require_adult(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .mail
        .setup
        .update_enrollment(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/enrollments/{id}", tag = "mail",
    params(("id" = String, Path, description = "enrollment id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_enrollment(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<EnrollmentId>,
) -> ApiResult<StatusCode> {
    require_adult(&auth.actor)?;
    s.services
        .mail
        .setup
        .delete_enrollment(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- rules, signals, categories ----

#[utoipa::path(get, path = "/api/v1/mail-rules", tag = "mail",
    security(("cookie" = [])),
    responses((status = 200, body = Vec<MailRuleView>), (status = 403, body = Problem)))]
pub async fn list_mail_rules(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<MailRuleView>>> {
    require_adult(&auth.actor)?;
    Ok(Json(
        s.services
            .mail
            .setup
            .list_rules(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/mail-rules", tag = "mail", request_body = MailRuleInput,
    security(("cookie" = [])),
    responses((status = 201, body = MailRuleView), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "`{field, message}`")))]
pub async fn create_mail_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<MailRuleInput>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .create_rule(household(&auth.actor)?, input)
        .await
        .map_err(is_rule_error)?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/mail-rules/{id}", tag = "mail", request_body = MailRulePatch,
    params(("id" = String, Path, description = "rule id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = MailRuleView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_mail_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailRuleId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<MailRulePatch>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .mail
        .setup
        .update_rule(household(&auth.actor)?, id, patch, expected)
        .await
        .map_err(is_rule_error)?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/mail-rules/{id}", tag = "mail",
    params(("id" = String, Path, description = "rule id")),
    security(("cookie" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_mail_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MailRuleId>,
) -> ApiResult<StatusCode> {
    require_setup(&auth.actor)?;
    s.services
        .mail
        .setup
        .delete_rule(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/mail-rules/test", tag = "mail", request_body = RuleTestRequest,
    security(("cookie" = [])),
    responses((status = 200, body = RuleTestResult), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "`{field, message}`")))]
pub async fn test_mail_rule(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<RuleTestRequest>,
) -> ApiResult<Json<RuleTestResult>> {
    require_setup(&auth.actor)?;
    let r = s
        .services
        .mail
        .setup
        .test_rule(household(&auth.actor)?, viewer_of(&auth.actor)?, req)
        .await
        .map_err(is_rule_error)?;
    Ok(Json(r))
}

#[utoipa::path(get, path = "/api/v1/mail-signals", tag = "mail",
    security(("cookie" = [])),
    responses((status = 200, body = Vec<SignalView>), (status = 403, body = Problem)))]
pub async fn list_mail_signals(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<SignalView>>> {
    require_adult(&auth.actor)?;
    Ok(Json(
        s.services
            .mail
            .setup
            .list_signals(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(patch, path = "/api/v1/mail-signals/{key}", tag = "mail", request_body = SignalPatch,
    params(("key" = String, Path, description = "signal key")),
    security(("cookie" = [])),
    responses((status = 200, body = SignalView), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn patch_mail_signal(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(key): ApiPath<String>,
    ApiJson(patch): ApiJson<SignalPatch>,
) -> ApiResult<Json<SignalView>> {
    require_setup(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .set_signal(household(&auth.actor)?, &key, patch.enabled)
        .await?;
    Ok(Json(v))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CategoryListQuery {
    /// Include archived categories.
    archived: Option<bool>,
}

#[utoipa::path(get, path = "/api/v1/categories", tag = "mail", params(CategoryListQuery),
    security(("cookie" = [])),
    responses((status = 200, body = Vec<CategoryView>), (status = 403, body = Problem)))]
pub async fn list_categories(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<CategoryListQuery>,
) -> ApiResult<Json<Vec<CategoryView>>> {
    require_adult(&auth.actor)?;
    Ok(Json(
        s.services
            .mail
            .setup
            .list_categories(household(&auth.actor)?, q.archived.unwrap_or(false))
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/categories", tag = "mail", request_body = CategoryInput,
    security(("cookie" = [])),
    responses((status = 201, body = CategoryView), (status = 403, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem)))]
pub async fn create_category(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<CategoryInput>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let v = s
        .services
        .mail
        .setup
        .create_category(household(&auth.actor)?, input)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/categories/{id}", tag = "mail", request_body = CategoryPatch,
    params(("id" = String, Path, description = "category id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = CategoryView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_category(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<CategoryId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<CategoryPatch>,
) -> ApiResult<Response> {
    require_setup(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .mail
        .setup
        .update_category(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

// ---- inbox ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct InboxQuery {
    /// `event`, `bill`, `task` or `contact`; default all.
    #[param(value_type = Option<String>)]
    kind: Option<InboxKind>,
}

#[utoipa::path(get, path = "/api/v1/inbox", tag = "inbox", params(InboxQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = InboxView, description = "a device without elevation gets `items: []` and the counts"),
              (status = 403, body = Problem)))]
pub async fn get_inbox(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<InboxQuery>,
) -> ApiResult<Json<InboxView>> {
    require(&auth.actor, Action::ReadInbox, &Resource::Household)?;
    let hid = household(&auth.actor)?;
    let viewer = viewer_of(&auth.actor)?;
    let v = s.services.mail.inbox.list_kind(hid, viewer, q.kind).await?;
    Ok(Json(v))
}

async fn publish_inbox(s: &AppState, hid: HouseholdId) {
    let res = async {
        let mut tx = s.store.tx_for(hid).await?;
        ChangeBus::publish(&mut tx, hid, "inbox.updated", &serde_json::json!({})).await?;
        tx.commit().await?;
        Ok::<_, ApiError>(())
    }
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, "inbox.updated publish failed");
    }
}

fn approver(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ApproveEvent, &Resource::Household)
}

/// D-078: accepting or skipping a task that came from email needs access to its source mailbox.
/// Other tasks are not scoped.
pub(crate) async fn ensure_mail_task_scope(
    s: &AppState,
    actor: &Actor,
    id: TaskId,
) -> ApiResult<()> {
    let hid = household(actor)?;
    let v = s.services.tasks.get_task(hid, id).await?;
    if v.origin == TaskOrigin::Email {
        s.services
            .mail
            .inbox
            .ensure_task(hid, viewer_of(actor)?, id)
            .await?;
    }
    Ok(())
}

#[utoipa::path(post, path = "/api/v1/inbox/{kind}/{id}/approve", tag = "inbox",
    params(("kind" = String, Path, description = "event, bill, task or contact"),
           ("id" = String, Path, description = "item id")),
    request_body(content = serde_json::Value, description = "`EventApproval`, `BillPatch`, `TaskApproval` or `ContactApproval` by kind; may be empty"),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem, description = "the item is not awaiting a decision"),
              (status = 422, body = Problem)))]
pub async fn approve_inbox(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath((kind, id)): ApiPath<(InboxKind, Uuid)>,
    body: Bytes,
) -> ApiResult<StatusCode> {
    approver(&auth.actor)?;
    let viewer = viewer_of(&auth.actor)?;
    let hid = household(&auth.actor)?;
    match kind {
        InboxKind::Event => {
            let a: EventApproval = body_or_empty(&body)?;
            let id = EventId::from(id);
            s.services.mail.inbox.ensure_event(hid, viewer, id).await?;
            let by = viewer.member;
            let cancel = pending_cancel(&s, hid, id).await?;
            if cancel {
                s.services.calendar.approve_cancel(hid, id, by).await?;
            } else {
                s.services.calendar.approve_event(hid, id, a, by).await?;
            }
        }
        InboxKind::Bill => {
            let p: BillPatch = body_or_empty(&body)?;
            s.services
                .mail
                .inbox
                .approve_bill(hid, BillId::from(id), p, viewer)
                .await?;
        }
        InboxKind::Task => {
            let a: TaskApproval = body_or_empty(&body)?;
            s.services
                .mail
                .inbox
                .ensure_task(hid, viewer, TaskId::from(id))
                .await?;
            approve_task(&s, hid, TaskId::from(id), a).await?;
        }
        InboxKind::Contact => {
            let a: ContactApproval = body_or_empty(&body)?;
            s.services
                .mail
                .inbox
                .approve_contact(hid, ContactSuggestionId::from(id), a, viewer)
                .await?;
        }
    }
    audit_after(
        &s,
        &auth.actor,
        "inbox.approve",
        kind.as_str(),
        id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/inbox/{kind}/{id}/reject", tag = "inbox",
    params(("kind" = String, Path, description = "event, bill, task or contact"),
           ("id" = String, Path, description = "item id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem, description = "the item is not awaiting a decision")))]
pub async fn reject_inbox(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath((kind, id)): ApiPath<(InboxKind, Uuid)>,
) -> ApiResult<StatusCode> {
    approver(&auth.actor)?;
    let viewer = viewer_of(&auth.actor)?;
    let by = viewer.member;
    let hid = household(&auth.actor)?;
    match kind {
        InboxKind::Event => {
            let id = EventId::from(id);
            s.services.mail.inbox.ensure_event(hid, viewer, id).await?;
            if pending_cancel(&s, hid, id).await? {
                s.services.calendar.reject_cancel(hid, id, by).await?;
            } else {
                s.services.calendar.reject_event(hid, id, by).await?;
            }
        }
        InboxKind::Bill => {
            s.services
                .mail
                .inbox
                .reject_bill(hid, BillId::from(id), viewer)
                .await?;
        }
        InboxKind::Task => {
            let id = TaskId::from(id);
            s.services.mail.inbox.ensure_task(hid, viewer, id).await?;
            suggested_mail_task(&s, hid, id).await?;
            s.services.tasks.skip(hid, id, by).await?;
            publish_inbox(&s, hid).await;
        }
        InboxKind::Contact => {
            s.services
                .mail
                .inbox
                .reject_contact(hid, ContactSuggestionId::from(id), viewer)
                .await?;
        }
    }
    audit_after(
        &s,
        &auth.actor,
        "inbox.reject",
        kind.as_str(),
        id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Whether the extracted event is a proposal to delete an event we wrote.
async fn pending_cancel(s: &AppState, hid: HouseholdId, id: EventId) -> ApiResult<bool> {
    let mut tx = s.store.tx_for(hid).await?;
    let e = EventRepo::get_full(&mut tx, id).await?;
    tx.rollback().await?;
    let e = e
        .filter(|e| e.event.origin == EventOrigin::Extracted)
        .ok_or(ApiError::NotFound)?;
    Ok(e.pending_cancel && e.event.status == Some(EventStatus::Written))
}

/// The task view, when it is a suggestion that came from email.
async fn suggested_mail_task(
    s: &AppState,
    hid: HouseholdId,
    id: TaskId,
) -> ApiResult<fa_domain::TaskView> {
    let v = s.services.tasks.get_task(hid, id).await?;
    if v.origin != TaskOrigin::Email {
        return Err(ApiError::NotFound);
    }
    if v.status != TaskStatus::Suggested {
        return Err(ApiError::Conflict(
            "the task is not awaiting a decision".into(),
        ));
    }
    Ok(v)
}

async fn approve_task(
    s: &AppState,
    hid: HouseholdId,
    id: TaskId,
    a: TaskApproval,
) -> ApiResult<()> {
    let v = suggested_mail_task(s, hid, id).await?;
    if a.title.is_some() || a.assignees.is_some() || a.due_at.is_some() {
        s.services
            .tasks
            .update_task(
                hid,
                id,
                TaskPatch {
                    title: a.title,
                    assignees: a.assignees,
                    due_at: a.due_at.map(Some),
                    notes: None,
                    priority_flag: None,
                    calendar_display: None,
                    steps: None,
                },
                v.version,
            )
            .await?;
    }
    s.services.tasks.accept(hid, id).await?;
    publish_inbox(s, hid).await;
    Ok(())
}

// ---- bills ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct BillListQuery {
    /// `proposed`, `open`, `paid` or `dismissed`; default all.
    #[param(value_type = Option<String>)]
    status: Option<BillStatus>,
}

/// Bill views with the message link only when the viewer may read the message's source.
async fn bill_views(
    s: &AppState,
    hid: HouseholdId,
    actor: &Actor,
    bills: Vec<fa_store::Versioned<fa_domain::Bill>>,
) -> ApiResult<Vec<BillView>> {
    let mut tx = s.store.tx_for(hid).await?;
    let mut out = Vec::with_capacity(bills.len());
    let mut readable: std::collections::HashMap<MailSourceId, bool> = Default::default();
    for b in bills {
        let v = &b.value;
        let task_done = match v.task_id {
            Some(t) => TaskRepo::get(&mut tx, t)
                .await?
                .is_some_and(|t| t.value.status == TaskStatus::Done),
            None => false,
        };
        let mut message = None;
        if let Some(m) = v.message_id
            && let Some(msg) = MessageRepo::get(&mut tx, m).await?
        {
            let ok = match readable.get(&msg.source_id) {
                Some(ok) => *ok,
                None => {
                    let owners = MailSourceRepo::owners(&mut tx, msg.source_id).await?;
                    let vis = MailSourceRepo::get(&mut tx, msg.source_id)
                        .await?
                        .map(|s| s.value.visibility);
                    let ok = vis.is_some_and(|visibility| {
                        authorize(
                            actor,
                            Action::ReadMail,
                            &Resource::MailSource { owners, visibility },
                            &PolicyCtx::default(),
                        )
                        .is_ok()
                    });
                    readable.insert(msg.source_id, ok);
                    ok
                }
            };
            if ok {
                message = Some(m);
            }
        }
        let external_url = message.and_then(|_| {
            v.source_ref
                .get("external_url")
                .and_then(|u| u.as_str())
                .map(str::to_owned)
        });
        out.push(BillView {
            id: v.id,
            payee: v.payee.clone(),
            amount_minor: v.amount_minor,
            currency: v.currency.clone(),
            due_on: v.due_on,
            account_hint: v.account_hint.clone(),
            status: v.status,
            confidence: v.confidence,
            task: v.task_id,
            task_done,
            message,
            external_url,
            version: b.version,
        });
    }
    tx.rollback().await?;
    Ok(out)
}

#[utoipa::path(get, path = "/api/v1/bills", tag = "bills", params(BillListQuery),
    security(("cookie" = [])),
    responses((status = 200, body = Vec<BillView>), (status = 403, body = Problem)))]
pub async fn list_bills(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<BillListQuery>,
) -> ApiResult<Json<Vec<BillView>>> {
    require_adult(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let bills = {
        let mut tx = s.store.tx_for(hid).await?;
        let b = BillRepo::list(&mut tx, q.status).await?;
        tx.rollback().await?;
        b
    };
    Ok(Json(bill_views(&s, hid, &auth.actor, bills).await?))
}

#[utoipa::path(patch, path = "/api/v1/bills/{id}", tag = "bills", request_body = BillPatch,
    params(("id" = String, Path, description = "bill id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = [])),
    responses((status = 200, body = BillView), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_bill(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<BillId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<BillPatch>,
) -> ApiResult<Response> {
    require(&auth.actor, Action::ManageBills, &Resource::Household)?;
    let expected = parse_version(&headers, None)?;
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let cur = BillRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if cur.version != expected {
        return Err(ApiError::Conflict(
            "the record was changed by someone else".into(),
        ));
    }
    if cur.value.status == BillStatus::Proposed && p.status == Some(BillPatchStatus::Dismissed) {
        // Dismissing a proposal also teaches the pipeline that the message is not a bill.
        tx.rollback().await?;
        s.services
            .mail
            .inbox
            .reject_bill(hid, id, viewer_of(&auth.actor)?)
            .await?;
        let mut tx = s.store.tx_for(hid).await?;
        let b = BillRepo::get(&mut tx, id)
            .await?
            .ok_or(ApiError::NotFound)?;
        tx.rollback().await?;
        return one_bill(&s, hid, &auth.actor, b).await;
    }
    let mut b = cur.value.clone();
    if let Some(payee) = p.payee {
        if payee.trim().is_empty() {
            return Err(field("payee", "the payee is empty"));
        }
        b.payee = payee.trim().to_owned();
    }
    if let Some(a) = p.amount_minor {
        if a <= 0 {
            return Err(field("amount_minor", "the amount must be positive"));
        }
        b.amount_minor = a;
    }
    if let Some(c) = p.currency {
        let c = c.trim().to_uppercase();
        if c.len() != 3 || !c.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return Err(field(
                "currency",
                "currency must be a 3-letter ISO 4217 code",
            ));
        }
        b.currency = c;
    }
    if let Some(d) = p.due_on {
        b.due_on = d;
    }
    if let Some(a) = p.account_hint {
        b.account_hint = Some(a.trim().to_owned()).filter(|a| !a.is_empty());
    }
    match p.status {
        Some(BillPatchStatus::Dismissed) => b.status = BillStatus::Dismissed,
        Some(BillPatchStatus::Open) if b.status == BillStatus::Dismissed => {
            b.status = BillStatus::Open;
        }
        _ => {}
    }
    BillRepo::update(&mut tx, &b, expected).await?;
    ChangeBus::publish(&mut tx, hid, "bills.updated", &serde_json::json!({})).await?;
    let updated = BillRepo::get(&mut tx, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    audit_after(&s, &auth.actor, "bill.update", "bill", id.to_string()).await;
    one_bill(&s, hid, &auth.actor, updated).await
}

async fn one_bill(
    s: &AppState,
    hid: HouseholdId,
    actor: &Actor,
    b: fa_store::Versioned<fa_domain::Bill>,
) -> ApiResult<Response> {
    let v = bill_views(s, hid, actor, vec![b])
        .await?
        .pop()
        .ok_or(ApiError::NotFound)?;
    Ok(with_etag(v.version, v))
}

fn field(name: &str, message: &str) -> ApiError {
    ApiError::FieldInvalid {
        field: name.into(),
        position: None,
        message: message.into(),
    }
}
