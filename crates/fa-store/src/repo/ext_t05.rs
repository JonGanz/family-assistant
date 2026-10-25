//! Repository extensions added by M2 t05 (see plan/M2).

use chrono::{DateTime, Utc};
use fa_domain::{
    MemberId, RoutineRun, RoutineRunId, TaskDefinition, TaskDefinitionId, TaskId, WeatherRuleId,
};
use sqlx::PgConnection;

use super::{RunRow, RunStepRow, TaskDefinitionRepo, TaskRepo, parse_enum};
use crate::Result;

impl TaskRepo {
    /// Overrides the completion stamp (the service clock instead of the database clock).
    pub async fn set_completed_at(
        conn: &mut PgConnection,
        id: TaskId,
        at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE task SET completed_at = $2 WHERE id = $1",
            id.as_uuid(),
            at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Overrides the creation stamp (the service clock instead of the database clock).
    pub async fn set_created_at(
        conn: &mut PgConnection,
        id: TaskId,
        at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE task SET created_at = $2 WHERE id = $1",
            id.as_uuid(),
            at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Open, non-archived tasks past due with assignees: `(task, title, assignees)`.
    pub async fn overdue_titled(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(TaskId, String, Vec<MemberId>)>> {
        let rows = sqlx::query!(
            r#"SELECT t.id, t.title,
                      ARRAY(SELECT member_id FROM task_assignee WHERE task_id = t.id
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
                    r.title,
                    r.assignees.into_iter().map(Into::into).collect(),
                )
            })
            .collect())
    }

    /// Has the weather rule already made a task due at `due_at`? Serialises concurrent callers.
    pub async fn weather_task_exists(
        conn: &mut PgConnection,
        rule: WeatherRuleId,
        due_at: DateTime<Utc>,
    ) -> Result<bool> {
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            format!("weather:{}:{}", rule.as_uuid(), due_at.timestamp())
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM task WHERE weather_rule_id = $1 AND due_at = $2) AS "e!""#,
            rule.as_uuid(),
            due_at
        )
        .fetch_one(conn)
        .await?)
    }
}

impl TaskDefinitionRepo {
    /// Locks the row, waiting for a concurrent holder (`FOR UPDATE`).
    pub async fn lock_blocking(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
    ) -> Result<Option<TaskDefinition>> {
        let found = sqlx::query_scalar!(
            "SELECT id FROM task_definition WHERE id = $1 FOR UPDATE",
            id.as_uuid()
        )
        .fetch_optional(&mut *conn)
        .await?;
        match found {
            Some(_) => Ok(Self::get(conn, id).await?.map(|v| v.value)),
            None => Ok(None),
        }
    }
}

impl super::RoutineRunRepo {
    /// One run with its routine data and steps.
    pub async fn get(conn: &mut PgConnection, id: RoutineRunId) -> Result<Option<RunRow>> {
        let Some(b) = sqlx::query!(
            "SELECT r.id, r.household_id, r.routine_id, r.member_id, r.run_date, r.status,
                    r.completed_at, ro.name AS routine_name, ro.starts_at, ro.done_by,
                    ro.show_on_dashboard
             FROM routine_run r JOIN routine ro ON ro.id = r.routine_id WHERE r.id = $1",
            id.as_uuid()
        )
        .fetch_optional(&mut *conn)
        .await?
        else {
            return Ok(None);
        };
        let steps = sqlx::query!(
            "SELECT rs.step_id, s.ord, s.text, s.target_time, rs.done_at, rs.done_by,
                    rs.done_via_device
             FROM routine_run_step rs JOIN routine_step s ON s.id = rs.step_id
             WHERE rs.run_id = $1 ORDER BY s.ord",
            id.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(Some(RunRow {
            run: RoutineRun {
                id: b.id.into(),
                household_id: b.household_id.into(),
                routine_id: b.routine_id.into(),
                member_id: b.member_id.into(),
                run_date: b.run_date,
                status: parse_enum(&b.status)?,
                completed_at: b.completed_at,
            },
            routine_name: b.routine_name,
            starts_at: b.starts_at,
            done_by: b.done_by,
            show_on_dashboard: b.show_on_dashboard,
            steps: steps
                .into_iter()
                .map(|s| RunStepRow {
                    step_id: s.step_id.into(),
                    ord: s.ord,
                    text: s.text,
                    target_time: s.target_time,
                    done_at: s.done_at,
                    done_by: s.done_by.map(Into::into),
                    done_via_device: s.done_via_device.map(Into::into),
                })
                .collect(),
        }))
    }
}
