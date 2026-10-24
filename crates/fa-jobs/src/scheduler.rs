use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use croner::Cron;
use fa_store::{JobRepo, NewJob, ScheduleRepo, Store};
use tokio_util::sync::CancellationToken;

use crate::JobError;

const INTERVAL: Duration = Duration::from_secs(30);

pub struct Scheduler {
    store: Store,
}

impl Scheduler {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn run(self, cancel: CancellationToken) {
        loop {
            if let Err(e) = tick(&self.store, Utc::now()).await {
                tracing::error!(error = %e, "scheduler tick failed");
            }
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(INTERVAL) => {}
            }
        }
    }
}

/// Enqueues every enabled schedule that is due at `now` (the latest missed fire time only).
/// The idempotency key `schedule:{id}:{fire_time}` makes this safe across replicas and retries.
/// Returns the number of jobs actually enqueued.
pub async fn tick(store: &Store, now: DateTime<Utc>) -> Result<usize, JobError> {
    let mut tx = store.tx_system().await?;
    let schedules = ScheduleRepo::list(&mut tx, true).await?;
    let mut enqueued = 0;
    for s in schedules {
        let cron = match Cron::from_str(&s.cron) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(schedule = %s.name, error = %e, "invalid cron expression");
                continue;
            }
        };
        let mut from = s
            .last_enqueued_at
            .unwrap_or(now - chrono::Duration::seconds(30));
        let mut fire: Option<DateTime<Utc>> = None;
        for _ in 0..10_000 {
            match cron.find_next_occurrence(&from, false) {
                Ok(next) if next <= now => {
                    fire = Some(next);
                    from = next;
                }
                _ => break,
            }
        }
        let Some(fire) = fire else { continue };
        let mut job = NewJob::new(s.job_kind.clone());
        job.payload = s.payload.clone();
        job.idempotency_key = Some(format!("schedule:{}:{}", s.id, fire.timestamp()));
        if JobRepo::enqueue(&mut tx, &job).await?.is_some() {
            enqueued += 1;
        }
        ScheduleRepo::mark_enqueued(&mut tx, s.id, fire).await?;
    }
    tx.commit().await?;
    Ok(enqueued)
}
