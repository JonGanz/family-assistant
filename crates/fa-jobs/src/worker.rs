use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use fa_store::{Job, JobRepo};
use tokio_util::sync::CancellationToken;

use crate::{JobContext, JobError, JobHandler};

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub concurrency: usize,
    pub lease_secs: i64,
    pub poll_interval: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 2,
            lease_secs: 300,
            poll_interval: Duration::from_secs(2),
        }
    }
}

pub struct Worker {
    ctx: JobContext,
    handlers: Arc<HashMap<&'static str, Arc<dyn JobHandler>>>,
    config: WorkerConfig,
    name: String,
}

impl Worker {
    pub fn new(ctx: JobContext, handlers: Vec<Arc<dyn JobHandler>>, config: WorkerConfig) -> Self {
        let map = handlers.into_iter().map(|h| (h.kind(), h)).collect();
        Self {
            ctx,
            handlers: Arc::new(map),
            config,
            name: format!("worker-{}", fa_domain::JobId::new()),
        }
    }

    /// Runs `concurrency` claim loops until `cancel` fires; in-flight jobs finish first.
    pub async fn run(self, cancel: CancellationToken) {
        let mut set = tokio::task::JoinSet::new();
        for i in 0..self.config.concurrency.max(1) {
            let ctx = self.ctx.clone();
            let handlers = self.handlers.clone();
            let cfg = self.config.clone();
            let cancel = cancel.clone();
            let name = format!("{}-{i}", self.name);
            set.spawn(async move { claim_loop(ctx, handlers, cfg, name, cancel).await });
        }
        while set.join_next().await.is_some() {}
    }

    /// Claims and processes at most one job; returns whether one was processed.
    pub async fn run_once(&self) -> Result<bool, JobError> {
        process_one(&self.ctx, &self.handlers, &self.config, &self.name).await
    }
}

async fn claim_loop(
    ctx: JobContext,
    handlers: Arc<HashMap<&'static str, Arc<dyn JobHandler>>>,
    cfg: WorkerConfig,
    name: String,
    cancel: CancellationToken,
) {
    while !cancel.is_cancelled() {
        match process_one(&ctx, &handlers, &cfg, &name).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(e) => tracing::error!(error = %e, "job worker error"),
        }
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(cfg.poll_interval) => {}
        }
    }
}

async fn process_one(
    ctx: &JobContext,
    handlers: &HashMap<&'static str, Arc<dyn JobHandler>>,
    cfg: &WorkerConfig,
    name: &str,
) -> Result<bool, JobError> {
    let mut tx = ctx.store.tx_system().await?;
    let job = JobRepo::claim(&mut tx, name, cfg.lease_secs).await?;
    tx.commit().await?;
    let Some(job) = job else { return Ok(false) };
    let result = dispatch(ctx, handlers, &job).await;
    let mut tx = ctx.store.tx_system().await?;
    match result {
        Ok(()) => JobRepo::complete(&mut tx, job.id).await?,
        Err(e) => {
            tracing::warn!(job = %job.id, kind = %job.kind, attempt = job.attempts, error = %e, "job failed");
            JobRepo::fail(&mut tx, job.id, &e.to_string()).await?;
        }
    }
    tx.commit().await?;
    Ok(true)
}

async fn dispatch(
    ctx: &JobContext,
    handlers: &HashMap<&'static str, Arc<dyn JobHandler>>,
    job: &Job,
) -> Result<(), JobError> {
    let Some(h) = handlers.get(job.kind.as_str()) else {
        return Err(JobError::Handler(format!(
            "no handler for kind {}",
            job.kind
        )));
    };
    // A panicking handler fails the job instead of killing the worker.
    let h = h.clone();
    let ctx = ctx.clone();
    let payload = job.payload.clone();
    tokio::spawn(async move { h.run(&ctx, payload).await })
        .await
        .map_err(|e| JobError::Handler(format!("handler panicked: {e}")))?
}
