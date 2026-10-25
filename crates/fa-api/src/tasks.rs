//! M2 routes: task lists, tasks, definitions, routines and absences (plan/M2.md, HTTP API v1).
//!
//! Handlers stay thin: load what the policy needs, authorize, call the `TaskService`.

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::ETAG;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use fa_domain::{
    Action, Actor, CompleteTask, DefinitionError, Household, Member, MemberAwayId, MemberAwayView,
    MemberId, NewAway, NewTask, PolicyCtx, Resource, Role, RoutineId, RoutineInput, RoutinePatch,
    RoutineRunId, RoutineRunView, RoutineStepId, RoutineView, Schedule, StepTick, TaskDefinitionId,
    TaskDefinitionInput, TaskDefinitionPatch, TaskDefinitionView, TaskId, TaskListId,
    TaskListInput, TaskListPatch, TaskListView, TaskPatch, TaskStatus, TaskStepId, TaskView,
    ValidateDefinitionRequest, ValidateDefinitionResponse,
};
use fa_store::{HouseholdRepo, MemberRepo, TaskDefinitionRepo, TaskFilter, TaskListRepo, TaskRepo};
use fa_tasks::{StatusFilter, TaskQuery, TaskViewMode, Viewer};
use serde::{Deserialize, de::DeserializeOwned};
use utoipa::{IntoParams, ToSchema};

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery, require_ctx};
use crate::routes::{ApiResult, etag, household, parse_version};
use crate::{ApiError, AppState, Auth, require};

fn field_err(field: &str, message: impl Into<String>) -> ApiError {
    ApiError::FieldInvalid {
        field: field.into(),
        position: None,
        message: message.into(),
    }
}

fn with_etag<T: serde::Serialize>(version: i32, v: T) -> Response {
    ([(ETAG, etag(version))], Json(v)).into_response()
}

pub(crate) fn device_of(actor: &Actor) -> Option<fa_domain::DeviceId> {
    match actor {
        Actor::Device { device_id, .. } | Actor::ElevatedDevice { device_id, .. } => {
            Some(*device_id)
        }
        _ => None,
    }
}

/// The member behind the request: the signed-in user or the adult who elevated the device.
pub(crate) fn actor_member(actor: &Actor) -> Option<MemberId> {
    match actor {
        Actor::User { member_id, .. } => Some(*member_id),
        Actor::ElevatedDevice { adult, .. } => Some(*adult),
        _ => None,
    }
}

fn viewer_of(actor: &Actor) -> Viewer {
    match actor {
        Actor::User {
            member_id, role, ..
        } => Viewer {
            member: Some(*member_id),
            role: Some(*role),
            device: false,
        },
        Actor::ElevatedDevice { adult, .. } => Viewer {
            member: Some(*adult),
            role: Some(Role::Adult),
            device: true,
        },
        _ => Viewer {
            member: None,
            role: None,
            device: true,
        },
    }
}

fn is_child_user(actor: &Actor) -> Option<MemberId> {
    match actor {
        Actor::User {
            member_id,
            role: Role::Child,
            ..
        } => Some(*member_id),
        _ => None,
    }
}

fn require_read(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ReadOwnData, &Resource::Household)
}

fn require_manage(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ManageTasks, &Resource::Household)
}

/// Reads a member of the household (404 when it is absent or archived).
pub(crate) async fn load_member(
    s: &AppState,
    hid: fa_domain::HouseholdId,
    id: MemberId,
) -> ApiResult<Member> {
    let mut tx = s.store.tx_for(hid).await?;
    let m = MemberRepo::get(&mut tx, id).await?;
    tx.rollback().await?;
    m.map(|m| m.value)
        .filter(|m| m.archived_at.is_none())
        .ok_or(ApiError::NotFound)
}

/// The member who acts: the user, the elevating adult, or (plain devices) the one the request names.
async fn resolve_member(
    s: &AppState,
    actor: &Actor,
    supplied: Option<MemberId>,
) -> ApiResult<MemberId> {
    let hid = household(actor)?;
    let id = match (actor, supplied) {
        (Actor::User { member_id, .. }, _) => return Ok(*member_id),
        (Actor::ElevatedDevice { adult, .. }, None) => return Ok(*adult),
        (_, Some(m)) => m,
        (_, None) => return Err(field_err("member", "a device must say which member acts")),
    };
    match load_member(s, hid, id).await {
        Ok(_) => Ok(id),
        Err(ApiError::NotFound) => Err(field_err("member", "not a member of this household")),
        Err(e) => Err(e),
    }
}

/// An optional JSON body: empty means "all defaults".
fn opt_body<T: DeserializeOwned>(b: &Bytes) -> ApiResult<Option<T>> {
    if b.iter().all(u8::is_ascii_whitespace) {
        return Ok(None);
    }
    serde_json::from_slice(b)
        .map(Some)
        .map_err(|e| ApiError::Unprocessable(e.to_string()))
}

struct Loaded {
    row: fa_store::TaskRow,
    household: Household,
}

impl Loaded {
    fn resource(&self) -> Resource {
        let t = &self.row.task.value;
        Resource::Task {
            assignees: self.row.assignees.clone(),
            created_by: t.created_by,
            definition: t.definition_id.is_some(),
        }
    }

    fn ctx(&self) -> PolicyCtx {
        PolicyCtx {
            dashboard_completion_override: self.household.dashboard_completion_override,
            list_completion_policy: self.row.completion_policy,
        }
    }

    /// Children see only tasks assigned to them, or unassigned ones; never suggestions.
    fn visible_to(&self, actor: &Actor) -> bool {
        match is_child_user(actor) {
            Some(me) => {
                self.row.task.value.status != TaskStatus::Suggested
                    && (self.row.assignees.is_empty() || self.row.assignees.contains(&me))
            }
            None => true,
        }
    }
}

async fn load_task(s: &AppState, hid: fa_domain::HouseholdId, id: TaskId) -> ApiResult<Loaded> {
    let mut tx = s.store.tx_for(hid).await?;
    let row = TaskRepo::get_view_rows(
        &mut tx,
        &TaskFilter {
            ids: Some(vec![id]),
            ..Default::default()
        },
    )
    .await?
    .into_iter()
    .next()
    .ok_or(ApiError::NotFound)?;
    let household = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    tx.rollback().await?;
    Ok(Loaded { row, household })
}

async fn load_list(
    s: &AppState,
    hid: fa_domain::HouseholdId,
    id: TaskListId,
) -> ApiResult<fa_domain::TaskList> {
    let mut tx = s.store.tx_for(hid).await?;
    let l = TaskListRepo::get(&mut tx, id).await?;
    tx.rollback().await?;
    l.map(|l| l.value)
        .filter(|l| l.archived_at.is_none())
        .ok_or_else(|| field_err("list", "unknown task list"))
}

async fn load_household(s: &AppState, hid: fa_domain::HouseholdId) -> ApiResult<Household> {
    let mut tx = s.store.tx_for(hid).await?;
    let h = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?;
    tx.rollback().await?;
    Ok(h.value)
}

// ---- tasks ----

#[derive(Deserialize, ToSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ViewParam {
    Now,
    Mine,
    All,
}

#[derive(Deserialize, ToSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StatusParam {
    Open,
    Done,
    Skipped,
    Suggested,
    Any,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TaskListQuery {
    view: Option<ViewParam>,
    #[param(value_type = Option<String>)]
    member: Option<MemberId>,
    #[param(value_type = Option<String>)]
    list: Option<TaskListId>,
    status: Option<StatusParam>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

#[utoipa::path(get, path = "/api/v1/tasks", tag = "tasks", params(TaskListQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<TaskView>), (status = 403, body = Problem)))]
pub async fn list_tasks(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<TaskListQuery>,
) -> ApiResult<Json<Vec<TaskView>>> {
    require_read(&auth.actor)?;
    let query = TaskQuery {
        view: match q.view {
            Some(ViewParam::Now) => TaskViewMode::Now,
            Some(ViewParam::Mine) => TaskViewMode::Mine,
            Some(ViewParam::All) | None => TaskViewMode::All,
        },
        member: q.member,
        list: q.list,
        status: q.status.map(|s| match s {
            StatusParam::Open => StatusFilter::Open,
            StatusParam::Done => StatusFilter::Done,
            StatusParam::Skipped => StatusFilter::Skipped,
            StatusParam::Suggested => StatusFilter::Suggested,
            StatusParam::Any => StatusFilter::Any,
        }),
        from: q.from,
        to: q.to,
    };
    let list = s
        .services
        .tasks
        .list_tasks(household(&auth.actor)?, &query, viewer_of(&auth.actor))
        .await?;
    Ok(Json(list))
}

#[utoipa::path(post, path = "/api/v1/tasks", tag = "tasks", request_body = NewTask,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = TaskView), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "`{field, message}`")))]
pub async fn create_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(mut new): ApiJson<NewTask>,
) -> ApiResult<Response> {
    require_read(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let list = load_list(&s, hid, new.list).await?;
    require_ctx(
        &auth.actor,
        Action::AddTask,
        &Resource::TaskList {
            child_can_add: list.child_can_add,
            dashboard_add: list.dashboard_add,
        },
        &PolicyCtx::default(),
    )?;
    let member = resolve_member(&s, &auth.actor, new.member).await?;
    if let Some(me) = is_child_user(&auth.actor) {
        // A child adds tasks for themself only.
        match new.assignees.as_slice() {
            [] => new.assignees = vec![me],
            [only] if *only == me => {}
            _ => return Err(ApiError::Forbidden("forbidden".into())),
        }
    }
    let view = s
        .services
        .tasks
        .create_task(hid, new, Some(member), device_of(&auth.actor))
        .await?;
    Ok((
        StatusCode::CREATED,
        [(ETAG, etag(view.version))],
        Json(view),
    )
        .into_response())
}

#[utoipa::path(get, path = "/api/v1/tasks/{id}", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 404, body = Problem)))]
pub async fn get_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Response> {
    require_read(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let loaded = load_task(&s, hid, id).await?;
    if !loaded.visible_to(&auth.actor) {
        return Err(ApiError::NotFound);
    }
    let view = s.services.tasks.get_task(hid, id).await?;
    Ok(with_etag(view.version, view))
}

#[utoipa::path(patch, path = "/api/v1/tasks/{id}", tag = "tasks", request_body = TaskPatch,
    params(("id" = String, Path, description = "task id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<TaskPatch>,
) -> ApiResult<Response> {
    let hid = household(&auth.actor)?;
    let loaded = load_task(&s, hid, id).await?;
    require_ctx(
        &auth.actor,
        Action::EditTask,
        &loaded.resource(),
        &loaded.ctx(),
    )?;
    let expected = parse_version(&headers, None)?;
    let view = s.services.tasks.update_task(hid, id, p, expected).await?;
    Ok(with_etag(view.version, view))
}

#[utoipa::path(delete, path = "/api/v1/tasks/{id}", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<StatusCode> {
    let hid = household(&auth.actor)?;
    let loaded = load_task(&s, hid, id).await?;
    require_ctx(
        &auth.actor,
        Action::EditTask,
        &loaded.resource(),
        &loaded.ctx(),
    )?;
    s.services.tasks.archive_task(hid, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Loads the task and runs the `CompleteTask` policy on it.
async fn authorize_complete(s: &AppState, actor: &Actor, id: TaskId) -> ApiResult<Loaded> {
    let loaded = load_task(s, household(actor)?, id).await?;
    require_ctx(
        actor,
        Action::CompleteTask,
        &loaded.resource(),
        &loaded.ctx(),
    )?;
    Ok(loaded)
}

/// Audit trail for task state changes (D-013: who completed it, from which device).
async fn audit_task(
    s: &AppState,
    actor: &Actor,
    action: &str,
    id: TaskId,
    after: serde_json::Value,
) -> ApiResult<()> {
    let mut tx = s.store.tx_for(household(actor)?).await?;
    crate::routes::audit(
        &mut tx,
        actor,
        action,
        "task",
        Some(id.to_string()),
        Some(after),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/complete", tag = "tasks", request_body = CompleteTask,
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem, description = "a device did not send `member`")))]
pub async fn complete_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
    body: Bytes,
) -> ApiResult<Json<TaskView>> {
    let supplied = opt_body::<CompleteTask>(&body)?.and_then(|b| b.member);
    authorize_complete(&s, &auth.actor, id).await?;
    let member = resolve_member(&s, &auth.actor, supplied).await?;
    let view = s
        .services
        .tasks
        .complete(
            household(&auth.actor)?,
            id,
            Some(member),
            device_of(&auth.actor),
        )
        .await?;
    audit_task(
        &s,
        &auth.actor,
        "task.complete",
        id,
        serde_json::json!({ "member": member }),
    )
    .await?;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/reopen", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem)))]
pub async fn reopen_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Json<TaskView>> {
    authorize_complete(&s, &auth.actor, id).await?;
    let view = s.services.tasks.reopen(household(&auth.actor)?, id).await?;
    audit_task(&s, &auth.actor, "task.reopen", id, serde_json::json!({})).await?;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/skip", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem)))]
pub async fn skip_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Json<TaskView>> {
    require_manage(&auth.actor)?;
    let view = s
        .services
        .tasks
        .skip(household(&auth.actor)?, id, actor_member(&auth.actor))
        .await?;
    audit_task(&s, &auth.actor, "task.skip", id, serde_json::json!({})).await?;
    Ok(Json(view))
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/accept", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem)))]
pub async fn accept_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Json<TaskView>> {
    require_manage(&auth.actor)?;
    Ok(Json(
        s.services.tasks.accept(household(&auth.actor)?, id).await?,
    ))
}

async fn authorize_claim(s: &AppState, actor: &Actor, id: TaskId) -> ApiResult<MemberId> {
    let loaded = load_task(s, household(actor)?, id).await?;
    require_ctx(actor, Action::ClaimTask, &loaded.resource(), &loaded.ctx())?;
    actor_member(actor).ok_or_else(|| ApiError::Forbidden("forbidden".into()))
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/claim", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem),
              (status = 409, body = Problem, description = "claimed by someone else or not open"),
              (status = 422, body = Problem, description = "not a pool task or not a candidate")))]
pub async fn claim_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Json<TaskView>> {
    let me = authorize_claim(&s, &auth.actor, id).await?;
    Ok(Json(
        s.services
            .tasks
            .claim(household(&auth.actor)?, id, me)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/tasks/{id}/release", tag = "tasks",
    params(("id" = String, Path, description = "task id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem)))]
pub async fn release_task(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskId>,
) -> ApiResult<Json<TaskView>> {
    let me = authorize_claim(&s, &auth.actor, id).await?;
    Ok(Json(
        s.services
            .tasks
            .release(household(&auth.actor)?, id, me)
            .await?,
    ))
}

#[utoipa::path(patch, path = "/api/v1/tasks/{id}/steps/{step_id}", tag = "tasks", request_body = StepTick,
    params(("id" = String, Path, description = "task id"),
           ("step_id" = String, Path, description = "step id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem, description = "a device did not send `member`")))]
pub async fn tick_task_step(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath((id, step)): ApiPath<(TaskId, TaskStepId)>,
    ApiJson(tick): ApiJson<StepTick>,
) -> ApiResult<Json<TaskView>> {
    authorize_complete(&s, &auth.actor, id).await?;
    let member = resolve_member(&s, &auth.actor, tick.member).await?;
    Ok(Json(
        s.services
            .tasks
            .tick_step(
                household(&auth.actor)?,
                id,
                step,
                tick.done,
                Some(member),
                device_of(&auth.actor),
            )
            .await?,
    ))
}

// ---- task lists ----

#[utoipa::path(get, path = "/api/v1/task-lists", tag = "tasks",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<TaskListView>), (status = 403, body = Problem)))]
pub async fn list_task_lists(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<TaskListView>>> {
    require_read(&auth.actor)?;
    Ok(Json(
        s.services
            .tasks
            .list_task_lists(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/task-lists", tag = "tasks", request_body = TaskListInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = TaskListView), (status = 403, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem)))]
pub async fn create_task_list(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(i): ApiJson<TaskListInput>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let v = s
        .services
        .tasks
        .create_task_list(household(&auth.actor)?, i)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/task-lists/{id}", tag = "tasks", request_body = TaskListPatch,
    params(("id" = String, Path, description = "list id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskListView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_task_list(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskListId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<TaskListPatch>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .tasks
        .update_task_list(household(&auth.actor)?, id, p, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/task-lists/{id}", tag = "tasks",
    params(("id" = String, Path, description = "list id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_task_list(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskListId>,
) -> ApiResult<StatusCode> {
    require_manage(&auth.actor)?;
    s.services
        .tasks
        .archive_task_list(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- task definitions ----

/// A bad RRULE becomes a 422 with the field name.
fn check_schedule(schedule: &Schedule) -> ApiResult<()> {
    if let Schedule::Rrule { rrule } = schedule {
        fa_tasks::validate_rrule(rrule).map_err(|m| field_err("rrule", m))?;
    }
    Ok(())
}

/// A bad season condition becomes a 422 carrying `{field, position, message}`.
fn check_condition(condition: &str) -> ApiResult<()> {
    fa_weather::validate_condition(condition).map_err(|e| ApiError::FieldInvalid {
        field: "season_condition".into(),
        position: u32::try_from(e.position).ok(),
        message: e.message,
    })
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DefinitionQuery {
    #[param(value_type = Option<String>)]
    list: Option<TaskListId>,
}

#[utoipa::path(get, path = "/api/v1/task-definitions", tag = "tasks", params(DefinitionQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<TaskDefinitionView>), (status = 403, body = Problem)))]
pub async fn list_definitions(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<DefinitionQuery>,
) -> ApiResult<Json<Vec<TaskDefinitionView>>> {
    require_manage(&auth.actor)?;
    Ok(Json(
        s.services
            .tasks
            .list_definitions(household(&auth.actor)?, q.list)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/task-definitions", tag = "tasks", request_body = TaskDefinitionInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = TaskDefinitionView), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "invalid RRULE or `season_condition`: `{field, position?, message}`")))]
pub async fn create_definition(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(i): ApiJson<TaskDefinitionInput>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    check_schedule(&i.schedule)?;
    if let Some(c) = &i.season_condition {
        check_condition(c)?;
    }
    let v = s
        .services
        .tasks
        .create_definition(household(&auth.actor)?, i)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/task-definitions/{id}", tag = "tasks", request_body = TaskDefinitionPatch,
    params(("id" = String, Path, description = "definition id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TaskDefinitionView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_definition(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskDefinitionId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<TaskDefinitionPatch>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    if let Some(sch) = &p.schedule {
        check_schedule(sch)?;
    }
    if let Some(Some(c)) = &p.season_condition {
        check_condition(c)?;
    }
    let v = s
        .services
        .tasks
        .update_definition(household(&auth.actor)?, id, p, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/task-definitions/{id}", tag = "tasks",
    params(("id" = String, Path, description = "definition id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_definition(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<TaskDefinitionId>,
) -> ApiResult<StatusCode> {
    require_manage(&auth.actor)?;
    s.services
        .tasks
        .archive_definition(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/task-definitions/validate", tag = "tasks", request_body = ValidateDefinitionRequest,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = ValidateDefinitionResponse), (status = 403, body = Problem)))]
pub async fn validate_definition(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<ValidateDefinitionRequest>,
) -> ApiResult<Json<ValidateDefinitionResponse>> {
    require_manage(&auth.actor)?;
    let h = load_household(&s, household(&auth.actor)?).await?;
    let tz: chrono_tz::Tz = h.tz.parse().unwrap_or(chrono_tz::UTC);
    let now = Utc::now();
    let mut errors = Vec::new();
    let mut next_dates = Vec::new();
    if let Some(rrule) = &req.rrule {
        match fa_tasks::validate_rrule(rrule) {
            Ok(()) => {
                let start = req
                    .start_date
                    .unwrap_or_else(|| now.with_timezone(&tz).date_naive());
                next_dates = fa_tasks::preview_dates(
                    &Schedule::Rrule {
                        rrule: rrule.clone(),
                    },
                    start,
                    None,
                    tz,
                    now,
                    5,
                );
            }
            Err(message) => errors.push(DefinitionError {
                field: "rrule".into(),
                position: None,
                message,
            }),
        }
    }
    if let Some(c) = &req.season_condition
        && let Err(e) = fa_weather::validate_condition(c)
    {
        errors.push(DefinitionError {
            field: "season_condition".into(),
            position: i32::try_from(e.position).ok(),
            message: e.message,
        });
    }
    Ok(Json(ValidateDefinitionResponse {
        ok: errors.is_empty(),
        errors,
        next_dates,
    }))
}

// ---- routines ----

#[utoipa::path(get, path = "/api/v1/routines", tag = "tasks",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<RoutineView>), (status = 403, body = Problem)))]
pub async fn list_routines(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<RoutineView>>> {
    require_read(&auth.actor)?;
    Ok(Json(
        s.services
            .tasks
            .list_routines(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/routines", tag = "tasks", request_body = RoutineInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = RoutineView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn create_routine(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(i): ApiJson<RoutineInput>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let v = s
        .services
        .tasks
        .create_routine(household(&auth.actor)?, i)
        .await?;
    Ok((StatusCode::CREATED, [(ETAG, etag(v.version))], Json(v)).into_response())
}

#[utoipa::path(patch, path = "/api/v1/routines/{id}", tag = "tasks", request_body = RoutinePatch,
    params(("id" = String, Path, description = "routine id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RoutineView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn patch_routine(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RoutineId>,
    headers: HeaderMap,
    ApiJson(p): ApiJson<RoutinePatch>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .tasks
        .update_routine(household(&auth.actor)?, id, p, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/routines/{id}", tag = "tasks",
    params(("id" = String, Path, description = "routine id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_routine(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RoutineId>,
) -> ApiResult<StatusCode> {
    require_manage(&auth.actor)?;
    s.services
        .tasks
        .archive_routine(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TodayQuery {
    #[param(value_type = Option<String>)]
    member: Option<MemberId>,
}

#[utoipa::path(get, path = "/api/v1/routines/today", tag = "tasks", params(TodayQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<RoutineRunView>), (status = 403, body = Problem)))]
pub async fn routines_today(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<TodayQuery>,
) -> ApiResult<Json<Vec<RoutineRunView>>> {
    require_read(&auth.actor)?;
    // Children only ever see their own runs.
    let member = is_child_user(&auth.actor).or(q.member);
    Ok(Json(
        s.services
            .tasks
            .routines_today(household(&auth.actor)?, member)
            .await?,
    ))
}

#[utoipa::path(patch, path = "/api/v1/routine-runs/{id}/steps/{step_id}", tag = "tasks", request_body = StepTick,
    params(("id" = String, Path, description = "routine run id"),
           ("step_id" = String, Path, description = "routine step id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RoutineRunView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem, description = "a device did not send `member`")))]
pub async fn tick_routine_step(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath((run, step)): ApiPath<(RoutineRunId, RoutineStepId)>,
    ApiJson(tick): ApiJson<StepTick>,
) -> ApiResult<Json<RoutineRunView>> {
    let hid = household(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let found = fa_store::RoutineRunRepo::get(&mut tx, run)
        .await?
        .ok_or(ApiError::NotFound)?;
    let owner = MemberRepo::get(&mut tx, found.run.member_id)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    let household = HouseholdRepo::get(&mut tx, hid)
        .await?
        .ok_or(ApiError::NotFound)?
        .value;
    tx.rollback().await?;
    require_ctx(
        &auth.actor,
        Action::TickRoutineStep,
        &Resource::Member(owner.id, owner.role),
        &PolicyCtx {
            dashboard_completion_override: household.dashboard_completion_override,
            ..PolicyCtx::default()
        },
    )?;
    let member = resolve_member(&s, &auth.actor, tick.member).await?;
    Ok(Json(
        s.services
            .tasks
            .tick_routine_step(
                hid,
                run,
                step,
                tick.done,
                Some(member),
                device_of(&auth.actor),
            )
            .await?,
    ))
}

// ---- absences ----

#[utoipa::path(get, path = "/api/v1/members/{id}/away", tag = "tasks",
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<MemberAwayView>), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn list_away(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
) -> ApiResult<Json<Vec<MemberAwayView>>> {
    require_manage(&auth.actor)?;
    let hid = household(&auth.actor)?;
    load_member(&s, hid, id).await?;
    Ok(Json(s.services.tasks.list_away(hid, Some(id)).await?))
}

#[utoipa::path(post, path = "/api/v1/members/{id}/away", tag = "tasks", request_body = NewAway,
    params(("id" = String, Path, description = "member id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = MemberAwayView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn add_away(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberId>,
    ApiJson(a): ApiJson<NewAway>,
) -> ApiResult<Response> {
    require_manage(&auth.actor)?;
    let hid = household(&auth.actor)?;
    load_member(&s, hid, id).await?;
    let v = s.services.tasks.add_away(hid, id, a).await?;
    Ok((StatusCode::CREATED, Json(v)).into_response())
}

#[utoipa::path(delete, path = "/api/v1/away/{id}", tag = "tasks",
    params(("id" = String, Path, description = "absence id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_away(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MemberAwayId>,
) -> ApiResult<StatusCode> {
    require_manage(&auth.actor)?;
    s.services
        .tasks
        .delete_away(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- weather rules and dashboard support ----

/// A task rule must name an existing definition (`{"definition": id}`); an alert must name none.
pub(crate) async fn check_rule_task(
    s: &AppState,
    hid: fa_domain::HouseholdId,
    action: fa_domain::RuleAction,
    def: Option<&serde_json::Value>,
) -> ApiResult<()> {
    let given = def.filter(|v| !v.is_null());
    match action {
        fa_domain::RuleAction::Alert => match given {
            None => Ok(()),
            Some(_) => Err(field_err(
                "task_definition",
                "must be empty for an alert rule",
            )),
        },
        fa_domain::RuleAction::Task => {
            let id = given
                .and_then(|v| v.get("definition"))
                .and_then(serde_json::Value::as_str)
                .and_then(|s| s.parse::<TaskDefinitionId>().ok())
                .ok_or_else(|| {
                    field_err(
                        "task_definition",
                        "a task rule needs {\"definition\": <task definition id>}",
                    )
                })?;
            let mut tx = s.store.tx_for(hid).await?;
            let found = TaskDefinitionRepo::get(&mut tx, id).await?;
            tx.rollback().await?;
            match found {
                Some(d) if d.value.archived_at.is_none() => Ok(()),
                _ => Err(field_err("task_definition", "unknown task definition")),
            }
        }
    }
}

/// Dashboard tasks the device's `members` setting lets through: unassigned, or for a shown member.
pub(crate) fn tasks_for_members(
    tasks: Vec<TaskView>,
    sel: &fa_domain::MemberSelection,
) -> Vec<TaskView> {
    match sel {
        fa_domain::MemberSelection::All(_) => tasks,
        fa_domain::MemberSelection::Ids(ids) => tasks
            .into_iter()
            .filter(|t| t.assignees.is_empty() || t.assignees.iter().any(|a| ids.contains(a)))
            .collect(),
    }
}

pub(crate) fn runs_for_members(
    runs: Vec<RoutineRunView>,
    sel: &fa_domain::MemberSelection,
) -> Vec<RoutineRunView> {
    match sel {
        fa_domain::MemberSelection::All(_) => runs,
        fa_domain::MemberSelection::Ids(ids) => runs
            .into_iter()
            .filter(|r| ids.contains(&r.member))
            .collect(),
    }
}
