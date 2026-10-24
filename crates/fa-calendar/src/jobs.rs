use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use fa_domain::{CalendarSourceId, HouseholdId};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{CalendarSourceRepo, EventRepo, HouseholdRepo, Store};

use crate::error::CalendarError;
use crate::service::{CalendarService, enqueue_sync_in};

impl From<CalendarError> for JobError {
    fn from(e: CalendarError) -> Self {
        match e {
            CalendarError::Store(s) => JobError::Store(s),
            other => JobError::Handler(other.to_string()),
        }
    }
}

/// Enqueues `calendar.sync{source}` for every due source. Returns how many jobs were added.
pub async fn enqueue_due(store: &Store, now: DateTime<Utc>) -> Result<usize, JobError> {
    let mut tx = store.tx_system().await?;
    let due = CalendarSourceRepo::due(&mut tx, now).await?;
    let mut n = 0;
    for (h, s) in due {
        if enqueue_sync_in(&mut tx, h, s, now).await? {
            n += 1;
        }
    }
    tx.commit().await?;
    Ok(n)
}

/// Deletes synced occurrences older than each household's `calendar_past_days`.
pub async fn prune(store: &Store, now: DateTime<Utc>) -> Result<u64, JobError> {
    let households = {
        let mut tx = store.tx_system().await?;
        HouseholdRepo::list(&mut tx).await?
    };
    let mut total = 0;
    for h in households {
        let mut tx = store.tx_for(h.value.id).await?;
        total += EventRepo::prune_before(
            &mut tx,
            now - Duration::days(h.value.calendar_past_days.into()),
        )
        .await?;
        tx.commit().await?;
    }
    Ok(total)
}

struct SyncDue;
struct Prune;
struct Sync(Arc<CalendarService>);

impl JobHandler for SyncDue {
    fn kind(&self) -> &'static str {
        "calendar.sync_due"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            enqueue_due(&ctx.store, Utc::now()).await?;
            Ok(())
        })
    }
}

impl JobHandler for Prune {
    fn kind(&self) -> &'static str {
        "calendar.prune"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let n = prune(&ctx.store, Utc::now()).await?;
            tracing::info!(deleted = n, "calendar.prune");
            Ok(())
        })
    }
}

impl JobHandler for Sync {
    fn kind(&self) -> &'static str {
        "calendar.sync"
    }
    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let get = |k: &str| {
                payload
                    .get(k)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| JobError::Handler(format!("payload lacks `{k}`")))
            };
            let h: HouseholdId = get("household")?
                .parse()
                .map_err(|_| JobError::Handler("bad household id".into()))?;
            let s: CalendarSourceId = get("source")?
                .parse()
                .map_err(|_| JobError::Handler("bad source id".into()))?;
            Ok(self.0.sync_source(h, s).await?)
        })
    }
}

pub fn handlers(service: Arc<CalendarService>) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(SyncDue), Arc::new(Sync(service)), Arc::new(Prune)]
}

/// `(job kind, cron)`; `calendar.sync` is only ever enqueued.
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("calendar.sync_due", "* * * * *"),
        ("calendar.prune", "30 3 * * *"),
    ]
}
