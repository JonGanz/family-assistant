//! fa-jobs: Postgres-backed job queue, cron scheduler and the M0 job handlers.

mod handlers;
mod scheduler;
mod worker;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use fa_store::{JobRepo, NewJob, Store};

pub use handlers::{BackupVerify, SessionCleanup, ensure_default_schedules};
pub use scheduler::{Scheduler, tick as scheduler_tick};
pub use worker::{Worker, WorkerConfig};

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
    #[error("{0}")]
    Handler(String),
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Shared context handed to every handler.
#[derive(Clone)]
pub struct JobContext {
    pub store: Store,
    /// Directory holding database dumps (`backup.verify`); `None` when not configured.
    pub backup_dir: Option<PathBuf>,
}

pub trait JobHandler: Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>>;
}

/// Enqueues a job. A duplicate idempotency key is a no-op (returns `false`).
pub async fn enqueue(
    store: &Store,
    kind: &str,
    payload: serde_json::Value,
    run_at: Option<DateTime<Utc>>,
    priority: i32,
    idempotency_key: Option<String>,
) -> Result<bool, JobError> {
    let mut job = NewJob::new(kind);
    job.payload = payload;
    job.run_at = run_at;
    job.priority = priority;
    job.idempotency_key = idempotency_key;
    let mut tx = store.tx_system().await?;
    let id = JobRepo::enqueue(&mut tx, &job).await?;
    tx.commit().await?;
    Ok(id.is_some())
}
