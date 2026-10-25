use chrono::{DateTime, Utc};
use fa_domain::tasks_api::TaskListPatch;
use fa_domain::{TaskList, TaskListId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct TaskListRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    name: String,
    kind: String,
    completion_policy: String,
    child_can_add: String,
    dashboard_add: bool,
    calendar_default: String,
    claim_ttl_minutes: Option<i32>,
    visible_on_dashboard: bool,
    color: Option<String>,
    sort: i32,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<TaskList> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: TaskList {
                id: r.id.into(),
                household_id: r.household_id.into(),
                name: r.name,
                kind: parse_enum(&r.kind)?,
                completion_policy: parse_enum(&r.completion_policy)?,
                child_can_add: parse_enum(&r.child_can_add)?,
                dashboard_add: r.dashboard_add,
                calendar_default: parse_enum(&r.calendar_default)?,
                claim_ttl_minutes: r.claim_ttl_minutes,
                visible_on_dashboard: r.visible_on_dashboard,
                color: r.color,
                sort: r.sort,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl TaskListRepo {
    pub async fn insert(conn: &mut PgConnection, l: &TaskList) -> Result<Versioned<TaskList>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO task_list (id, household_id, name, kind, completion_policy, child_can_add,
                                    dashboard_add, calendar_default, claim_ttl_minutes,
                                    visible_on_dashboard, color, sort)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id, household_id, name, kind, completion_policy, child_can_add, dashboard_add,
                       calendar_default, claim_ttl_minutes, visible_on_dashboard, color, sort,
                       archived_at, version, created_at, updated_at",
            l.id.as_uuid(),
            l.household_id.as_uuid(),
            l.name,
            enum_str(&l.kind),
            enum_str(&l.completion_policy),
            enum_str(&l.child_can_add),
            l.dashboard_add,
            enum_str(&l.calendar_default),
            l.claim_ttl_minutes,
            l.visible_on_dashboard,
            l.color,
            l.sort
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: TaskListId,
    ) -> Result<Option<Versioned<TaskList>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, kind, completion_policy, child_can_add, dashboard_add,
                    calendar_default, claim_ttl_minutes, visible_on_dashboard, color, sort,
                    archived_at, version, created_at, updated_at
             FROM task_list WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<Versioned<TaskList>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, kind, completion_policy, child_can_add, dashboard_add,
                    calendar_default, claim_ttl_minutes, visible_on_dashboard, color, sort,
                    archived_at, version, created_at, updated_at
             FROM task_list WHERE $1 OR archived_at IS NULL ORDER BY sort, created_at, id",
            include_archived
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// A non-archived list by exact name.
    pub async fn get_by_name(
        conn: &mut PgConnection,
        name: &str,
    ) -> Result<Option<Versioned<TaskList>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, kind, completion_policy, child_can_add, dashboard_add,
                    calendar_default, claim_ttl_minutes, visible_on_dashboard, color, sort,
                    archived_at, version, created_at, updated_at
             FROM task_list WHERE name = $1 AND archived_at IS NULL ORDER BY created_at, id LIMIT 1",
            name
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Applies the set fields of a patch. Returns the new version, or `Conflict` if stale.
    pub async fn update(
        conn: &mut PgConnection,
        id: TaskListId,
        p: &TaskListPatch,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE task_list SET
                    name = COALESCE($2, name), kind = COALESCE($3, kind),
                    completion_policy = COALESCE($4, completion_policy),
                    child_can_add = COALESCE($5, child_can_add),
                    dashboard_add = COALESCE($6, dashboard_add),
                    calendar_default = COALESCE($7, calendar_default),
                    claim_ttl_minutes = CASE WHEN $8 THEN $9 ELSE claim_ttl_minutes END,
                    visible_on_dashboard = COALESCE($10, visible_on_dashboard),
                    color = CASE WHEN $11 THEN $12 ELSE color END,
                    sort = COALESCE($13, sort),
                    version = version + 1
             WHERE id = $1 AND version = $14
             RETURNING version",
            id.as_uuid(),
            p.name,
            p.kind.as_ref().map(enum_str),
            p.completion_policy.as_ref().map(enum_str),
            p.child_can_add.as_ref().map(enum_str),
            p.dashboard_add,
            p.calendar_default.as_ref().map(enum_str),
            p.claim_ttl_minutes.is_some(),
            p.claim_ttl_minutes.flatten(),
            p.visible_on_dashboard,
            p.color.is_some(),
            p.color.clone().flatten(),
            p.sort,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "task_list", id.as_uuid()).await),
        }
    }

    pub async fn archive(conn: &mut PgConnection, id: TaskListId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task_list SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}
