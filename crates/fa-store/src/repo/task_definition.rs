use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use fa_domain::{
    HouseholdId, MemberId, Schedule, ScheduleKind, TaskDefinition, TaskDefinitionId, TaskListId,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct TaskDefinitionRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    list_id: Uuid,
    title: String,
    notes: Option<String>,
    steps_template: Vec<String>,
    schedule_kind: String,
    interval_days: Option<i32>,
    rrule: Option<String>,
    start_date: NaiveDate,
    due_time: Option<NaiveTime>,
    lead_days: i32,
    on_missed: String,
    strategy: String,
    advance_on: String,
    skip_advances: bool,
    rotation_cursor: i32,
    priority_flag: String,
    season_from: Option<i32>,
    season_until: Option<i32>,
    season_condition: Option<String>,
    condition_ok: Option<bool>,
    condition_checked_at: Option<DateTime<Utc>>,
    last_done_at: Option<DateTime<Utc>>,
    next_due_at: Option<DateTime<Utc>>,
    paused: bool,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// `SELECT <all definition columns> <tail>`; the tail starts with `FROM`.
macro_rules! select_def {
    ($tail:literal $(, $arg:expr)* $(,)?) => {
        sqlx::query_as!(
            Row,
            "SELECT id, household_id, list_id, title, notes, steps_template, schedule_kind,
                    interval_days, rrule, start_date, due_time, lead_days, on_missed, strategy,
                    advance_on, skip_advances, rotation_cursor, priority_flag, season_from,
                    season_until, season_condition, condition_ok, condition_checked_at,
                    last_done_at, next_due_at, paused, archived_at, version, created_at, updated_at "
                + $tail
            $(, $arg)*
        )
    };
}

impl TryFrom<Row> for Versioned<TaskDefinition> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        let schedule = match parse_enum::<ScheduleKind>(&r.schedule_kind)? {
            ScheduleKind::AfterDone => Schedule::AfterDone {
                interval_days: r
                    .interval_days
                    .ok_or_else(|| StoreError::Corrupt("after_done without interval".into()))?,
            },
            ScheduleKind::Rrule => Schedule::Rrule {
                rrule: r
                    .rrule
                    .ok_or_else(|| StoreError::Corrupt("rrule schedule without rrule".into()))?,
            },
        };
        Ok(Versioned {
            version: r.version,
            value: TaskDefinition {
                id: r.id.into(),
                household_id: r.household_id.into(),
                list_id: r.list_id.into(),
                title: r.title,
                notes: r.notes,
                steps_template: r.steps_template,
                schedule,
                start_date: r.start_date,
                due_time: r.due_time,
                lead_days: r.lead_days,
                on_missed: parse_enum(&r.on_missed)?,
                strategy: parse_enum(&r.strategy)?,
                advance_on: parse_enum(&r.advance_on)?,
                skip_advances: r.skip_advances,
                rotation_cursor: r.rotation_cursor,
                priority_flag: parse_enum(&r.priority_flag)?,
                season: r.season_from.zip(r.season_until),
                season_condition: r.season_condition,
                condition_ok: r.condition_ok,
                condition_checked_at: r.condition_checked_at,
                last_done_at: r.last_done_at,
                next_due_at: r.next_due_at,
                paused: r.paused,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

fn schedule_cols(s: &Schedule) -> (&'static str, Option<i32>, Option<&str>) {
    match s {
        Schedule::AfterDone { interval_days } => ("after_done", Some(*interval_days), None),
        Schedule::Rrule { rrule } => ("rrule", None, Some(rrule.as_str())),
    }
}

async fn replace_candidates(
    conn: &mut PgConnection,
    household: HouseholdId,
    def: TaskDefinitionId,
    members: &[MemberId],
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM task_definition_candidate WHERE definition_id = $1",
        def.as_uuid()
    )
    .execute(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = members.iter().map(|m| m.as_uuid()).collect();
    sqlx::query!(
        "INSERT INTO task_definition_candidate (definition_id, member_id, household_id, ord)
         SELECT $1, t.member_id, $2, (t.ord - 1)::int
         FROM unnest($3::uuid[]) WITH ORDINALITY AS t(member_id, ord)",
        def.as_uuid(),
        household.as_uuid(),
        &ids
    )
    .execute(conn)
    .await?;
    Ok(())
}

impl TaskDefinitionRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        d: &TaskDefinition,
        candidates: &[MemberId],
    ) -> Result<Versioned<TaskDefinition>> {
        let (kind, interval, rrule) = schedule_cols(&d.schedule);
        let (sf, su) = d.season.unzip();
        sqlx::query!(
            "INSERT INTO task_definition (id, household_id, list_id, title, notes, steps_template,
                    schedule_kind, interval_days, rrule, start_date, due_time, lead_days, on_missed,
                    strategy, advance_on, skip_advances, rotation_cursor, priority_flag, season_from,
                    season_until, season_condition, paused, next_due_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                     $19, $20, $21, $22, $23)",
            d.id.as_uuid(),
            d.household_id.as_uuid(),
            d.list_id.as_uuid(),
            d.title,
            d.notes,
            &d.steps_template,
            kind,
            interval,
            rrule,
            d.start_date,
            d.due_time,
            d.lead_days,
            enum_str(&d.on_missed),
            enum_str(&d.strategy),
            enum_str(&d.advance_on),
            d.skip_advances,
            d.rotation_cursor,
            enum_str(&d.priority_flag),
            sf,
            su,
            d.season_condition,
            d.paused,
            d.next_due_at
        )
        .execute(&mut *conn)
        .await?;
        replace_candidates(conn, d.household_id, d.id, candidates).await?;
        Self::get(conn, d.id).await?.ok_or(StoreError::NotFound)
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
    ) -> Result<Option<Versioned<TaskDefinition>>> {
        let row = select_def!("FROM task_definition WHERE id = $1", id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        list: Option<TaskListId>,
        include_archived: bool,
    ) -> Result<Vec<Versioned<TaskDefinition>>> {
        let rows = select_def!(
            "FROM task_definition
             WHERE ($1::uuid IS NULL OR list_id = $1) AND ($2 OR archived_at IS NULL)
             ORDER BY created_at, id",
            list.map(|l| l.as_uuid()),
            include_archived
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// A non-archived definition by exact title within a list.
    pub async fn get_by_title(
        conn: &mut PgConnection,
        list: TaskListId,
        title: &str,
    ) -> Result<Option<Versioned<TaskDefinition>>> {
        let row = select_def!(
            "FROM task_definition WHERE list_id = $1 AND title = $2 AND archived_at IS NULL
             ORDER BY created_at, id LIMIT 1",
            list.as_uuid(),
            title
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Replaces the editable fields, including `next_due_at` (callers reset it to `None` on a
    /// schedule change). The system state (`rotation_cursor`, `last_done_at`, condition result) is
    /// untouched. `candidates: Some` replaces the ordered candidate list. Returns the new version.
    pub async fn update(
        conn: &mut PgConnection,
        d: &TaskDefinition,
        candidates: Option<&[MemberId]>,
        expected_version: i32,
    ) -> Result<i32> {
        let (kind, interval, rrule) = schedule_cols(&d.schedule);
        let (sf, su) = d.season.unzip();
        let v = sqlx::query_scalar!(
            "UPDATE task_definition SET list_id = $2, title = $3, notes = $4, steps_template = $5,
                    schedule_kind = $6, interval_days = $7, rrule = $8, start_date = $9, due_time = $10,
                    lead_days = $11, on_missed = $12, strategy = $13, advance_on = $14,
                    skip_advances = $15, priority_flag = $16, season_from = $17, season_until = $18,
                    season_condition = $19, paused = $20, next_due_at = $21, version = version + 1
             WHERE id = $1 AND version = $22
             RETURNING version",
            d.id.as_uuid(),
            d.list_id.as_uuid(),
            d.title,
            d.notes,
            &d.steps_template,
            kind,
            interval,
            rrule,
            d.start_date,
            d.due_time,
            d.lead_days,
            enum_str(&d.on_missed),
            enum_str(&d.strategy),
            enum_str(&d.advance_on),
            d.skip_advances,
            enum_str(&d.priority_flag),
            sf,
            su,
            d.season_condition,
            d.paused,
            d.next_due_at,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        let Some(v) = v else {
            return Err(conflict_or_missing(conn, "task_definition", d.id.as_uuid()).await);
        };
        if let Some(c) = candidates {
            replace_candidates(conn, d.household_id, d.id, c).await?;
        }
        Ok(v)
    }

    pub async fn archive(conn: &mut PgConnection, id: TaskDefinitionId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task_definition SET archived_at = COALESCE(archived_at, now()), version = version + 1
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

    /// The ordered candidates `(member, ord)`.
    pub async fn candidates(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
    ) -> Result<Vec<(MemberId, i32)>> {
        let rows = sqlx::query!(
            "SELECT member_id, ord FROM task_definition_candidate WHERE definition_id = $1 ORDER BY ord",
            id.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.member_id.into(), r.ord))
            .collect())
    }

    /// Active, unpaused definitions that might need a task now: `next_due_at` unset, or its
    /// visibility start (at most `lead_days + 1` days before it, to allow for any tz) has passed.
    /// fa-tasks checks precisely. Use a system tx.
    pub async fn due_for_materialize(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(HouseholdId, TaskDefinitionId)>> {
        let rows = sqlx::query!(
            "SELECT household_id, id FROM task_definition
             WHERE archived_at IS NULL AND NOT paused
               AND (next_due_at IS NULL OR next_due_at - make_interval(days => lead_days + 1) <= $1)
             ORDER BY next_due_at NULLS FIRST, id",
            now
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.household_id.into(), r.id.into()))
            .collect())
    }

    /// Locks the row (`FOR UPDATE SKIP LOCKED`); `None` if missing or locked by someone else.
    pub async fn lock(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
    ) -> Result<Option<TaskDefinition>> {
        let row = select_def!(
            "FROM task_definition WHERE id = $1 FOR UPDATE SKIP LOCKED",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row
            .map(Versioned::<TaskDefinition>::try_from)
            .transpose()?
            .map(|v| v.value))
    }

    /// Writes the scheduling state. System state: no version bump.
    pub async fn set_schedule_state(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
        rotation_cursor: i32,
        last_done_at: Option<DateTime<Utc>>,
        next_due_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task_definition SET rotation_cursor = $2, last_done_at = $3, next_due_at = $4,
                    updated_at = now()
             WHERE id = $1",
            id.as_uuid(),
            rotation_cursor,
            last_done_at,
            next_due_at
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

    /// Non-archived definitions with a `season_condition` (all households in a system tx).
    pub async fn with_condition(
        conn: &mut PgConnection,
    ) -> Result<Vec<(TaskDefinitionId, String)>> {
        let rows = sqlx::query!(
            r#"SELECT id, season_condition AS "season_condition!" FROM task_definition
               WHERE archived_at IS NULL AND season_condition IS NOT NULL ORDER BY created_at, id"#
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.id.into(), r.season_condition))
            .collect())
    }

    /// Stores the weather evaluation result. System state: no version bump.
    pub async fn set_condition_result(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
        ok: bool,
        checked_at: DateTime<Utc>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE task_definition SET condition_ok = $2, condition_checked_at = $3, updated_at = now()
             WHERE id = $1",
            id.as_uuid(),
            ok,
            checked_at
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
