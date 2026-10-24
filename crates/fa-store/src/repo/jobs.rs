use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, JobId};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::Result;

/// System-wide queue tables (no RLS); use a system tx.
pub struct JobRepo;
pub struct ScheduleRepo;

#[derive(Debug, Clone)]
pub struct NewJob {
    pub kind: String,
    pub payload: serde_json::Value,
    pub household_id: Option<HouseholdId>,
    pub run_at: Option<DateTime<Utc>>,
    pub priority: i32,
    pub max_attempts: i32,
    pub idempotency_key: Option<String>,
}

impl NewJob {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            payload: serde_json::json!({}),
            household_id: None,
            run_at: None,
            priority: 0,
            max_attempts: 5,
            idempotency_key: None,
        }
    }
}

/// Queue depth for the status page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobCounts {
    pub ready: i64,
    pub running: i64,
    pub dead: i64,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub household_id: Option<HouseholdId>,
    pub kind: String,
    pub payload: serde_json::Value,
    /// `queued`, `running`, `done` or `dead`.
    pub status: String,
    pub run_at: DateTime<Utc>,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub locked_by: Option<String>,
    pub locked_until: Option<DateTime<Utc>>,
    pub idempotency_key: Option<String>,
    pub last_error: Option<String>,
}

struct Row {
    id: Uuid,
    household_id: Option<Uuid>,
    kind: String,
    payload: serde_json::Value,
    status: String,
    run_at: DateTime<Utc>,
    priority: i32,
    attempts: i32,
    max_attempts: i32,
    locked_by: Option<String>,
    locked_until: Option<DateTime<Utc>>,
    idempotency_key: Option<String>,
    last_error: Option<String>,
}

impl From<Row> for Job {
    fn from(r: Row) -> Self {
        Job {
            id: r.id.into(),
            household_id: r.household_id.map(Into::into),
            kind: r.kind,
            payload: r.payload,
            status: r.status,
            run_at: r.run_at,
            priority: r.priority,
            attempts: r.attempts,
            max_attempts: r.max_attempts,
            locked_by: r.locked_by,
            locked_until: r.locked_until,
            idempotency_key: r.idempotency_key,
            last_error: r.last_error,
        }
    }
}

impl JobRepo {
    /// Enqueue. With an idempotency key that already exists, nothing is inserted and `None` is returned.
    pub async fn enqueue(conn: &mut PgConnection, j: &NewJob) -> Result<Option<JobId>> {
        let id = sqlx::query_scalar!(
            "INSERT INTO job (id, household_id, kind, payload, run_at, priority, max_attempts, idempotency_key)
             VALUES ($1, $2, $3, $4, COALESCE($5, now()), $6, $7, $8)
             ON CONFLICT (idempotency_key) DO NOTHING
             RETURNING id",
            JobId::new().as_uuid(),
            j.household_id.map(|h| h.as_uuid()),
            j.kind,
            j.payload,
            j.run_at,
            j.priority,
            j.max_attempts,
            j.idempotency_key
        )
        .fetch_optional(conn)
        .await?;
        Ok(id.map(Into::into))
    }

    pub async fn get(conn: &mut PgConnection, id: JobId) -> Result<Option<Job>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, payload, status, run_at, priority, attempts, max_attempts,
                    locked_by, locked_until, idempotency_key, last_error
             FROM job WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Claims the next due job (`FOR UPDATE SKIP LOCKED`). Also reclaims running jobs whose lock expired.
    pub async fn claim(
        conn: &mut PgConnection,
        worker: &str,
        lock_secs: i64,
    ) -> Result<Option<Job>> {
        let row = sqlx::query_as!(
            Row,
            "UPDATE job SET status = 'running', locked_by = $1, attempts = attempts + 1,
                    locked_until = now() + make_interval(secs => $2)
             WHERE id = (
                 SELECT id FROM job
                 WHERE (status = 'queued' AND run_at <= now())
                    OR (status = 'running' AND locked_until < now())
                 ORDER BY priority DESC, run_at
                 FOR UPDATE SKIP LOCKED LIMIT 1)
             RETURNING id, household_id, kind, payload, status, run_at, priority, attempts, max_attempts,
                       locked_by, locked_until, idempotency_key, last_error",
            worker,
            lock_secs as f64
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn complete(conn: &mut PgConnection, id: JobId) -> Result<()> {
        sqlx::query!(
            "UPDATE job SET status = 'done', locked_by = NULL, locked_until = NULL WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Records a failure: retries with exponential backoff (30s * 2^attempts), or marks `dead`.
    pub async fn fail(conn: &mut PgConnection, id: JobId, error: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE job SET last_error = $2, locked_by = NULL, locked_until = NULL,
                    status = CASE WHEN attempts >= max_attempts THEN 'dead' ELSE 'queued' END,
                    run_at = now() + make_interval(secs => 30.0 * power(2, attempts))
             WHERE id = $1",
            id.as_uuid(),
            error
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn counts(conn: &mut PgConnection) -> Result<JobCounts> {
        let r = sqlx::query!(
            r#"SELECT
                 count(*) FILTER (WHERE status = 'queued' AND run_at <= now()) AS "ready!",
                 count(*) FILTER (WHERE status = 'running') AS "running!",
                 count(*) FILTER (WHERE status = 'dead') AS "dead!"
               FROM job"#
        )
        .fetch_one(conn)
        .await?;
        Ok(JobCounts {
            ready: r.ready,
            running: r.running,
            dead: r.dead,
        })
    }

    pub async fn list_dead(conn: &mut PgConnection) -> Result<Vec<Job>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, payload, status, run_at, priority, attempts, max_attempts,
                    locked_by, locked_until, idempotency_key, last_error
             FROM job WHERE status = 'dead' ORDER BY updated_at DESC"
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub id: Uuid,
    pub name: String,
    pub cron: String,
    pub job_kind: String,
    pub payload: serde_json::Value,
    pub enabled: bool,
    pub last_enqueued_at: Option<DateTime<Utc>>,
}

impl ScheduleRepo {
    /// Insert or update by `name` (keeps `last_enqueued_at`).
    pub async fn upsert(
        conn: &mut PgConnection,
        name: &str,
        cron: &str,
        job_kind: &str,
        payload: &serde_json::Value,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO schedule (id, name, cron, job_kind, payload, enabled) VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (name) DO UPDATE SET cron = EXCLUDED.cron, job_kind = EXCLUDED.job_kind,
                 payload = EXCLUDED.payload, enabled = EXCLUDED.enabled",
            Uuid::now_v7(),
            name,
            cron,
            job_kind,
            payload,
            enabled
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn list(conn: &mut PgConnection, only_enabled: bool) -> Result<Vec<Schedule>> {
        let rows = sqlx::query_as!(
            Schedule,
            "SELECT id, name, cron, job_kind, payload, enabled, last_enqueued_at
             FROM schedule WHERE $1 = false OR enabled ORDER BY name",
            only_enabled
        )
        .fetch_all(conn)
        .await?;
        Ok(rows)
    }

    pub async fn mark_enqueued(conn: &mut PgConnection, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        sqlx::query!(
            "UPDATE schedule SET last_enqueued_at = $2 WHERE id = $1",
            id,
            at
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}
