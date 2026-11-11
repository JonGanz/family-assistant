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

    /// Makes a household's queued jobs of `kind` runnable now (skips a retry backoff). Running,
    /// done and dead jobs and other households are untouched. Returns how many jobs were queued
    /// (including those already runnable).
    pub async fn release_queued(
        conn: &mut PgConnection,
        household: fa_domain::HouseholdId,
        kind: &str,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE job SET run_at = now()
             WHERE household_id = $1 AND kind = $2 AND status = 'queued' AND run_at > now()",
        )
        .bind(household.as_uuid())
        .bind(kind)
        .execute(conn)
        .await?
        .rows_affected())
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

/// A failed or retrying job for the status page (no payload content beyond id-like keys).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProblemJob {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub run_at: DateTime<Utc>,
    /// Only string/number values of keys ending in `id`, as text.
    pub refs: serde_json::Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProblemGroup {
    pub kind: String,
    pub status: String,
    pub error: Option<String>,
    pub count: i64,
    pub last_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LlmLastFailure {
    pub purpose: String,
    pub model: String,
    pub error_kind: Option<String>,
    pub at: DateTime<Utc>,
    pub failed_24h: i64,
    pub calls_24h: i64,
}

const PROBLEM: &str = "household_id = $1 AND (status = 'dead' OR (status = 'queued' AND attempts > 0 AND last_error IS NOT NULL))";

impl JobRepo {
    /// Recent dead jobs and retrying queued jobs (with an error) of a household, newest first.
    pub async fn list_problems(
        conn: &mut PgConnection,
        household: HouseholdId,
        limit: i64,
    ) -> Result<Vec<ProblemJob>> {
        Ok(sqlx::query_as::<_, ProblemJob>(&format!(
            "SELECT id, kind, status, attempts, max_attempts, last_error, created_at, updated_at, run_at,
                    COALESCE((SELECT jsonb_object_agg(e.key, e.value #>> '{{}}')
                              FROM jsonb_each(CASE WHEN jsonb_typeof(payload) = 'object' THEN payload ELSE '{{}}'::jsonb END) e
                              WHERE e.key ~* 'id$' AND jsonb_typeof(e.value) IN ('string', 'number')
                                AND length(e.value #>> '{{}}') <= 80), '{{}}'::jsonb) AS refs
             FROM job WHERE {PROBLEM} ORDER BY updated_at DESC LIMIT $2"
        ))
        .bind(household.as_uuid())
        .bind(limit)
        .fetch_all(conn)
        .await?)
    }

    /// Problem jobs grouped by kind, status and the first line of the error.
    pub async fn problem_groups(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<Vec<ProblemGroup>> {
        Ok(sqlx::query_as::<_, ProblemGroup>(&format!(
            "SELECT kind, status, split_part(last_error, E'\n', 1) AS error,
                    count(*) AS count, max(updated_at) AS last_at
             FROM job WHERE {PROBLEM}
             GROUP BY kind, status, split_part(last_error, E'\n', 1)
             ORDER BY count(*) DESC, max(updated_at) DESC"
        ))
        .bind(household.as_uuid())
        .fetch_all(conn)
        .await?)
    }

    /// Requeues a household's dead jobs of `kind` (fresh attempts, runnable now).
    pub async fn requeue_dead(
        conn: &mut PgConnection,
        household: HouseholdId,
        kind: &str,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE job SET status = 'queued', attempts = 0, run_at = now(), locked_by = NULL, locked_until = NULL
             WHERE household_id = $1 AND kind = $2 AND status = 'dead'",
        )
        .bind(household.as_uuid())
        .bind(kind)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// The latest failed AI call of a household with 24 h call and failure counts.
    pub async fn llm_last_failure(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<Option<LlmLastFailure>> {
        Ok(sqlx::query_as::<_, LlmLastFailure>(
            "SELECT purpose, model, error_kind, started_at AS at,
                    (SELECT count(*) FROM llm_call WHERE household_id = $1 AND NOT ok
                       AND started_at > now() - interval '24 hours') AS failed_24h,
                    (SELECT count(*) FROM llm_call WHERE household_id = $1
                       AND started_at > now() - interval '24 hours') AS calls_24h
             FROM llm_call WHERE household_id = $1 AND NOT ok ORDER BY started_at DESC LIMIT 1",
        )
        .bind(household.as_uuid())
        .fetch_optional(conn)
        .await?)
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
