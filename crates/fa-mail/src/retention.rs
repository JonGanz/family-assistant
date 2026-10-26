//! Message retention (D-034): pending messages older than `mail_max_pending_days` expire, then
//! the content policy purges bodies, snippets, headers and attachment text.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use fa_domain::HouseholdId;
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{HouseholdRepo, MessageRepo, Store};

use crate::sync::job_err;
use crate::{MailDeps, Result};

const BATCH: i64 = 500;
/// Upper bound of batches per household and run, so one run cannot hog the worker.
const MAX_BATCHES: usize = 40;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RetentionResult {
    pub expired: u64,
    pub purged: u64,
}

/// Runs the policy of one household.
pub async fn run_household(
    store: &Store,
    household: HouseholdId,
    now: DateTime<Utc>,
) -> Result<RetentionResult> {
    let mut tx = store.tx_for(household).await?;
    let h = HouseholdRepo::get(&mut tx, household)
        .await?
        .ok_or(crate::MailError::NotFound)?
        .value;
    let policy = h.retention_mail;
    let mut out = RetentionResult::default();

    let cutoff = now - Duration::days(policy.max_pending_days.into());
    let old = MessageRepo::pending_older_than(&mut tx, cutoff).await?;
    for chunk in old.chunks(BATCH as usize) {
        out.expired += MessageRepo::expire(&mut tx, chunk).await?;
    }

    for _ in 0..MAX_BATCHES {
        let ids = MessageRepo::retention_candidates(
            &mut tx,
            policy.content_policy,
            policy.examples_per_category,
            BATCH,
        )
        .await?;
        if ids.is_empty() {
            break;
        }
        out.purged += MessageRepo::purge_content(&mut tx, &ids).await?;
    }
    tx.commit().await?;
    Ok(out)
}

/// Runs the policy of every household.
pub async fn run_all(store: &Store, now: DateTime<Utc>) -> Result<RetentionResult> {
    let households = {
        let mut tx = store.tx_system().await?;
        HouseholdRepo::list(&mut tx).await?
    };
    let mut total = RetentionResult::default();
    for h in households {
        let r = run_household(store, h.value.id, now).await?;
        total.expired += r.expired;
        total.purged += r.purged;
    }
    Ok(total)
}

struct Retention;

impl JobHandler for Retention {
    fn kind(&self) -> &'static str {
        "mail.retention"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let r = run_all(&ctx.store, Utc::now()).await.map_err(job_err)?;
            tracing::info!(expired = r.expired, purged = r.purged, "mail.retention");
            Ok(())
        })
    }
}

pub fn handlers(_: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(Retention)]
}
