use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use fa_domain::{
    CalendarDisplay, DeviceId, HouseholdId, ListCompletionPolicy, MemberId, PriorityFlag, Task,
    TaskDefinitionId, TaskId, TaskListId, TaskOrigin, TaskStatus, TaskStep, WeatherRuleId,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct TaskRepo;

/// A task to insert. `due_at`/`visible_from` are set by the caller (fa-tasks).
#[derive(Debug, Clone)]
pub struct NewTaskRow {
    pub id: TaskId,
    pub household_id: HouseholdId,
    pub list_id: TaskListId,
    pub definition_id: Option<TaskDefinitionId>,
    pub weather_rule_id: Option<WeatherRuleId>,
    pub title: String,
    pub notes: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub visible_from: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub origin: TaskOrigin,
    pub source_ref: Option<serde_json::Value>,
    pub priority_flag: PriorityFlag,
    pub calendar_display: CalendarDisplay,
    pub created_by: Option<MemberId>,
}

/// Filter of [`TaskRepo::get_view_rows`]. Archived tasks are never returned. `None` fields don't filter.
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub ids: Option<Vec<TaskId>>,
    pub statuses: Option<Vec<TaskStatus>>,
    pub list: Option<TaskListId>,
    /// Assigned to this member.
    pub member: Option<MemberId>,
    /// `due_at >= due_from`.
    pub due_from: Option<DateTime<Utc>>,
    /// `due_at < due_to`.
    pub due_to: Option<DateTime<Utc>>,
    /// `visible_from IS NULL OR visible_from <= visible_before`.
    pub visible_before: Option<DateTime<Utc>>,
    pub calendar_display_not_hidden: bool,
}

/// A task with everything a `TaskView` needs.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub task: Versioned<Task>,
    pub list_name: String,
    pub completion_policy: ListCompletionPolicy,
    pub assignees: Vec<MemberId>,
    pub steps: Vec<TaskStep>,
}

struct Row {
    id: Uuid,
    household_id: Uuid,
    list_id: Uuid,
    definition_id: Option<Uuid>,
    weather_rule_id: Option<Uuid>,
    title: String,
    notes: Option<String>,
    due_at: Option<DateTime<Utc>>,
    visible_from: Option<DateTime<Utc>>,
    status: String,
    origin: String,
    source_ref: Option<serde_json::Value>,
    priority_flag: String,
    calendar_display: String,
    created_by: Option<Uuid>,
    completed_at: Option<DateTime<Utc>>,
    completed_by: Option<Uuid>,
    completed_via_device: Option<Uuid>,
    skipped_by: Option<Uuid>,
    claimed_by: Option<Uuid>,
    claimed_until: Option<DateTime<Utc>>,
    completion_undo: Option<serde_json::Value>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// `SELECT <task columns of alias t> <tail>`; the tail starts with `FROM task t`.
macro_rules! select_task {
    ($tail:literal $(, $arg:expr)* $(,)?) => {
        sqlx::query_as!(
            Row,
            "SELECT t.id, t.household_id, t.list_id, t.definition_id, t.weather_rule_id, t.title,
                    t.notes, t.due_at, t.visible_from, t.status, t.origin, t.source_ref,
                    t.priority_flag, t.calendar_display, t.created_by, t.completed_at,
                    t.completed_by, t.completed_via_device, t.skipped_by, t.claimed_by,
                    t.claimed_until, t.completion_undo, t.archived_at, t.version, t.created_at,
                    t.updated_at "
                + $tail
            $(, $arg)*
        )
    };
}

impl TryFrom<Row> for Versioned<Task> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Task {
                id: r.id.into(),
                household_id: r.household_id.into(),
                list_id: r.list_id.into(),
                definition_id: r.definition_id.map(Into::into),
                weather_rule_id: r.weather_rule_id.map(Into::into),
                title: r.title,
                notes: r.notes,
                due_at: r.due_at,
                visible_from: r.visible_from,
                status: parse_enum(&r.status)?,
                origin: parse_enum(&r.origin)?,
                source_ref: r.source_ref,
                priority_flag: parse_enum(&r.priority_flag)?,
                calendar_display: parse_enum(&r.calendar_display)?,
                created_by: r.created_by.map(Into::into),
                completed_at: r.completed_at,
                completed_by: r.completed_by.map(Into::into),
                completed_via_device: r.completed_via_device.map(Into::into),
                skipped_by: r.skipped_by.map(Into::into),
                claimed_by: r.claimed_by.map(Into::into),
                claimed_until: r.claimed_until,
                completion_undo: r.completion_undo,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

struct StepRow {
    id: Uuid,
    household_id: Uuid,
    task_id: Uuid,
    ord: i32,
    text: String,
    done_at: Option<DateTime<Utc>>,
    done_by: Option<Uuid>,
}

impl From<StepRow> for TaskStep {
    fn from(r: StepRow) -> Self {
        TaskStep {
            id: r.id.into(),
            household_id: r.household_id.into(),
            task_id: r.task_id.into(),
            ord: r.ord,
            text: r.text,
            done_at: r.done_at,
            done_by: r.done_by.map(Into::into),
        }
    }
}

async fn assignees_of(
    conn: &mut PgConnection,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<MemberId>>> {
    let rows = sqlx::query!(
        "SELECT task_id, member_id FROM task_assignee WHERE task_id = ANY($1) ORDER BY member_id",
        ids
    )
    .fetch_all(conn)
    .await?;
    let mut map: HashMap<Uuid, Vec<MemberId>> = HashMap::new();
    for r in rows {
        map.entry(r.task_id).or_default().push(r.member_id.into());
    }
    Ok(map)
}

async fn steps_of(conn: &mut PgConnection, ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<TaskStep>>> {
    let rows = sqlx::query_as!(
        StepRow,
        "SELECT id, household_id, task_id, ord, text, done_at, done_by FROM task_step
         WHERE task_id = ANY($1) ORDER BY task_id, ord",
        ids
    )
    .fetch_all(conn)
    .await?;
    let mut map: HashMap<Uuid, Vec<TaskStep>> = HashMap::new();
    for r in rows {
        map.entry(r.task_id).or_default().push(r.into());
    }
    Ok(map)
}

async fn replace_assignees(
    conn: &mut PgConnection,
    household: HouseholdId,
    task: TaskId,
    members: &[MemberId],
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM task_assignee WHERE task_id = $1",
        task.as_uuid()
    )
    .execute(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = members.iter().map(|m| m.as_uuid()).collect();
    sqlx::query!(
        "INSERT INTO task_assignee (task_id, member_id, household_id)
         SELECT $1, m, $2 FROM unnest($3::uuid[]) AS m ON CONFLICT DO NOTHING",
        task.as_uuid(),
        household.as_uuid(),
        &ids
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Replaces the steps by text: a new step reuses the first unused existing step with the same
/// text (keeping its tick); the others are deleted or inserted.
async fn replace_steps(
    conn: &mut PgConnection,
    household: HouseholdId,
    task: TaskId,
    texts: &[String],
) -> Result<()> {
    let existing = steps_of(conn, &[task.as_uuid()])
        .await?
        .remove(&task.as_uuid())
        .unwrap_or_default();
    let mut unused: Vec<Option<TaskStep>> = existing.into_iter().map(Some).collect();
    let mut plan: Vec<(usize, Option<Uuid>)> = Vec::with_capacity(texts.len());
    for (i, text) in texts.iter().enumerate() {
        let hit = unused
            .iter_mut()
            .find(|s| s.as_ref().is_some_and(|s| &s.text == text))
            .and_then(Option::take);
        plan.push((i, hit.map(|s| s.id.as_uuid())));
    }
    let drop: Vec<Uuid> = unused
        .into_iter()
        .flatten()
        .map(|s| s.id.as_uuid())
        .collect();
    sqlx::query!("DELETE FROM task_step WHERE id = ANY($1)", &drop)
        .execute(&mut *conn)
        .await?;
    // Park the kept steps on negative ords so renumbering cannot collide.
    sqlx::query!(
        "UPDATE task_step SET ord = -(ord + 1) WHERE task_id = $1",
        task.as_uuid()
    )
    .execute(&mut *conn)
    .await?;
    for (i, kept) in plan {
        let ord = i as i32;
        match kept {
            Some(id) => {
                sqlx::query!("UPDATE task_step SET ord = $2 WHERE id = $1", id, ord)
                    .execute(&mut *conn)
                    .await?;
            }
            None => {
                sqlx::query!(
                    "INSERT INTO task_step (id, household_id, task_id, ord, text)
                     VALUES ($1, $2, $3, $4, $5)",
                    Uuid::now_v7(),
                    household.as_uuid(),
                    task.as_uuid(),
                    ord,
                    texts[i]
                )
                .execute(&mut *conn)
                .await?;
            }
        }
    }
    Ok(())
}

impl TaskRepo {
    /// Inserts a task with its assignees and steps. `None` when the `(definition, origin, due_at)`
    /// key already exists (nothing is written then).
    pub async fn insert(
        conn: &mut PgConnection,
        t: &NewTaskRow,
        assignees: &[MemberId],
        steps: &[String],
    ) -> Result<Option<Versioned<Task>>> {
        let inserted = sqlx::query_scalar!(
            "INSERT INTO task (id, household_id, list_id, definition_id, weather_rule_id, title, notes,
                               due_at, visible_from, status, origin, source_ref, priority_flag,
                               calendar_display, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (definition_id, origin, due_at) DO NOTHING
             RETURNING id",
            t.id.as_uuid(),
            t.household_id.as_uuid(),
            t.list_id.as_uuid(),
            t.definition_id.map(|d| d.as_uuid()),
            t.weather_rule_id.map(|w| w.as_uuid()),
            t.title,
            t.notes,
            t.due_at,
            t.visible_from,
            enum_str(&t.status),
            enum_str(&t.origin),
            t.source_ref,
            enum_str(&t.priority_flag),
            enum_str(&t.calendar_display),
            t.created_by.map(|m| m.as_uuid())
        )
        .fetch_optional(&mut *conn)
        .await?;
        if inserted.is_none() {
            return Ok(None);
        }
        replace_assignees(conn, t.household_id, t.id, assignees).await?;
        replace_steps(conn, t.household_id, t.id, steps).await?;
        Self::get(conn, t.id).await
    }

    pub async fn get(conn: &mut PgConnection, id: TaskId) -> Result<Option<Versioned<Task>>> {
        let row = select_task!("FROM task t WHERE t.id = $1", id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn get_view_rows(conn: &mut PgConnection, f: &TaskFilter) -> Result<Vec<TaskRow>> {
        let ids: Option<Vec<Uuid>> = f
            .ids
            .as_ref()
            .map(|v| v.iter().map(|i| i.as_uuid()).collect());
        let statuses: Option<Vec<String>> = f
            .statuses
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str().to_owned()).collect());
        let rows = sqlx::query!(
            "SELECT t.id, t.household_id, t.list_id, t.definition_id, t.weather_rule_id, t.title,
                    t.notes, t.due_at, t.visible_from, t.status, t.origin, t.source_ref,
                    t.priority_flag, t.calendar_display, t.created_by, t.completed_at,
                    t.completed_by, t.completed_via_device, t.skipped_by, t.claimed_by,
                    t.claimed_until, t.completion_undo, t.archived_at, t.version, t.created_at,
                    t.updated_at, l.name AS list_name, l.completion_policy
             FROM task t JOIN task_list l ON l.id = t.list_id
             WHERE t.archived_at IS NULL
               AND ($1::uuid[] IS NULL OR t.id = ANY($1))
               AND ($2::text[] IS NULL OR t.status = ANY($2))
               AND ($3::uuid IS NULL OR t.list_id = $3)
               AND ($4::uuid IS NULL OR EXISTS (
                        SELECT 1 FROM task_assignee a WHERE a.task_id = t.id AND a.member_id = $4))
               AND ($5::timestamptz IS NULL OR t.due_at >= $5)
               AND ($6::timestamptz IS NULL OR t.due_at < $6)
               AND ($7::timestamptz IS NULL OR t.visible_from IS NULL OR t.visible_from <= $7)
               AND (NOT $8 OR t.calendar_display <> 'hidden')
             ORDER BY t.due_at NULLS LAST, t.created_at, t.id",
            ids.as_deref(),
            statuses.as_deref(),
            f.list.map(|l| l.as_uuid()),
            f.member.map(|m| m.as_uuid()),
            f.due_from,
            f.due_to,
            f.visible_before,
            f.calendar_display_not_hidden
        )
        .fetch_all(&mut *conn)
        .await?;
        let task_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mut assignees = assignees_of(conn, &task_ids).await?;
        let mut steps = steps_of(conn, &task_ids).await?;
        rows.into_iter()
            .map(|r| {
                let id = r.id;
                let task = Row {
                    id: r.id,
                    household_id: r.household_id,
                    list_id: r.list_id,
                    definition_id: r.definition_id,
                    weather_rule_id: r.weather_rule_id,
                    title: r.title,
                    notes: r.notes,
                    due_at: r.due_at,
                    visible_from: r.visible_from,
                    status: r.status,
                    origin: r.origin,
                    source_ref: r.source_ref,
                    priority_flag: r.priority_flag,
                    calendar_display: r.calendar_display,
                    created_by: r.created_by,
                    completed_at: r.completed_at,
                    completed_by: r.completed_by,
                    completed_via_device: r.completed_via_device,
                    skipped_by: r.skipped_by,
                    claimed_by: r.claimed_by,
                    claimed_until: r.claimed_until,
                    completion_undo: r.completion_undo,
                    archived_at: r.archived_at,
                    version: r.version,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                };
                Ok(TaskRow {
                    task: task.try_into()?,
                    list_name: r.list_name,
                    completion_policy: parse_enum(&r.completion_policy)?,
                    assignees: assignees.remove(&id).unwrap_or_default(),
                    steps: steps.remove(&id).unwrap_or_default(),
                })
            })
            .collect()
    }

    /// Open or suggested, non-archived tasks of a definition.
    pub async fn open_for_definition(
        conn: &mut PgConnection,
        def: TaskDefinitionId,
    ) -> Result<Vec<Task>> {
        let rows = select_task!(
            "FROM task t WHERE t.definition_id = $1 AND t.status IN ('open', 'suggested')
               AND t.archived_at IS NULL ORDER BY t.due_at NULLS LAST, t.created_at, t.id",
            def.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|r| Versioned::<Task>::try_from(r).map(|v| v.value))
            .collect()
    }

    /// Replaces title, notes, due date, priority and calendar display. `assignees` and `steps`
    /// replace the stored sets when `Some` (a step with unchanged text keeps its tick). Returns the
    /// new version.
    pub async fn update(
        conn: &mut PgConnection,
        t: &Task,
        assignees: Option<&[MemberId]>,
        steps: Option<&[String]>,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE task SET title = $2, notes = $3, due_at = $4, priority_flag = $5,
                    calendar_display = $6, version = version + 1
             WHERE id = $1 AND version = $7
             RETURNING version",
            t.id.as_uuid(),
            t.title,
            t.notes,
            t.due_at,
            enum_str(&t.priority_flag),
            enum_str(&t.calendar_display),
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        let Some(v) = v else {
            return Err(conflict_or_missing(conn, "task", t.id.as_uuid()).await);
        };
        if let Some(a) = assignees {
            replace_assignees(conn, t.household_id, t.id, a).await?;
        }
        if let Some(s) = steps {
            replace_steps(conn, t.household_id, t.id, s).await?;
        }
        Ok(v)
    }

    /// Sets the status. `Done` stamps `completed_*` (now), clears the claim and stores
    /// `completion_undo`; `Skipped` sets `skipped_by` (`None` = the system); `Open` and `Suggested`
    /// clear all of it. Bumps the version.
    pub async fn set_status(
        conn: &mut PgConnection,
        id: TaskId,
        status: TaskStatus,
        by: Option<MemberId>,
        device: Option<DeviceId>,
        completion_undo: Option<serde_json::Value>,
    ) -> Result<()> {
        let done = status == TaskStatus::Done;
        let skipped = status == TaskStatus::Skipped;
        let n = sqlx::query!(
            "UPDATE task SET status = $2,
                    completed_at = CASE WHEN $3 THEN now() END,
                    completed_by = CASE WHEN $3 THEN $5::uuid END,
                    completed_via_device = CASE WHEN $3 THEN $6::uuid END,
                    skipped_by = CASE WHEN $4 THEN $5::uuid END,
                    completion_undo = CASE WHEN $3 OR $4 THEN $7::jsonb END,
                    claimed_by = CASE WHEN $3 THEN NULL ELSE claimed_by END,
                    claimed_until = CASE WHEN $3 THEN NULL ELSE claimed_until END,
                    version = version + 1
             WHERE id = $1",
            id.as_uuid(),
            status.as_str(),
            done,
            skipped,
            by.map(|m| m.as_uuid()),
            device.map(|d| d.as_uuid()),
            completion_undo
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

    /// Back to `open`, clearing completion, skip and undo data.
    pub async fn reopen(conn: &mut PgConnection, id: TaskId) -> Result<()> {
        Self::set_status(conn, id, TaskStatus::Open, None, None, None).await
    }

    /// Deletes the open tasks of a definition created after `after` that have no ticked step
    /// (the occurrences made since a completion that is being undone).
    pub async fn delete_created_after(
        conn: &mut PgConnection,
        def: TaskDefinitionId,
        after: DateTime<Utc>,
    ) -> Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM task t
             WHERE t.definition_id = $1 AND t.status = 'open' AND t.created_at > $2
               AND NOT EXISTS (SELECT 1 FROM task_step s WHERE s.task_id = t.id AND s.done_at IS NOT NULL)",
            def.as_uuid(),
            after
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Sets (or with `None`, releases) the claim. Bumps the version.
    pub async fn set_claim(
        conn: &mut PgConnection,
        id: TaskId,
        member: Option<MemberId>,
        until: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task SET claimed_by = $2, claimed_until = $3, version = version + 1 WHERE id = $1",
            id.as_uuid(),
            member.map(|m| m.as_uuid()),
            until
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

    pub async fn clear_expired_claims(conn: &mut PgConnection, now: DateTime<Utc>) -> Result<u64> {
        Ok(sqlx::query!(
            "UPDATE task SET claimed_by = NULL, claimed_until = NULL, version = version + 1
             WHERE claimed_until IS NOT NULL AND claimed_until <= $1",
            now
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Ticks or unticks a step (`done = false` clears it). Doesn't bump the task version.
    pub async fn tick_step(
        conn: &mut PgConnection,
        task: TaskId,
        step: fa_domain::TaskStepId,
        done: bool,
        by: Option<MemberId>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task_step SET done_at = CASE WHEN $3 THEN now() END,
                    done_by = CASE WHEN $3 THEN $4::uuid END
             WHERE id = $1 AND task_id = $2",
            step.as_uuid(),
            task.as_uuid(),
            done,
            by.map(|m| m.as_uuid())
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

    pub async fn archive(conn: &mut PgConnection, id: TaskId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task SET archived_at = COALESCE(archived_at, now()), version = version + 1 WHERE id = $1",
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

    /// Open, non-archived tasks past due that have assignees, with those assignees.
    pub async fn overdue_open(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(TaskId, Vec<MemberId>)>> {
        let rows = sqlx::query!(
            r#"SELECT t.id, ARRAY(SELECT member_id FROM task_assignee WHERE task_id = t.id
                                  ORDER BY member_id) AS "assignees!"
               FROM task t
               WHERE t.status = 'open' AND t.archived_at IS NULL AND t.due_at < $1
                 AND EXISTS (SELECT 1 FROM task_assignee WHERE task_id = t.id)
               ORDER BY t.due_at, t.id"#,
            now
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id.into(),
                    r.assignees.into_iter().map(Into::into).collect(),
                )
            })
            .collect())
    }

    /// Open tasks with assignees due within `(now, now + horizon]`: `(task, due_at, assignees, title)`.
    pub async fn due_soon_candidates(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
        horizon: Duration,
    ) -> Result<Vec<(TaskId, DateTime<Utc>, Vec<MemberId>, String)>> {
        let rows = sqlx::query!(
            r#"SELECT t.id, t.due_at AS "due_at!", t.title,
                      ARRAY(SELECT member_id FROM task_assignee WHERE task_id = t.id
                            ORDER BY member_id) AS "assignees!"
               FROM task t
               WHERE t.status = 'open' AND t.archived_at IS NULL
                 AND t.due_at > $1 AND t.due_at <= $2
                 AND EXISTS (SELECT 1 FROM task_assignee WHERE task_id = t.id)
               ORDER BY t.due_at, t.id"#,
            now,
            now + horizon
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id.into(),
                    r.due_at,
                    r.assignees.into_iter().map(Into::into).collect(),
                    r.title,
                )
            })
            .collect())
    }
}
