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
    let ids = households.iter().map(|h| h.value.id).collect();
    run_each(ids, |id| Box::pin(run_household(store, id, now))).await
}

/// Runs `run` for every household. A failing household is logged and skipped; the first error
/// is returned at the end, so one broken household cannot starve the others.
async fn run_each<'a>(
    ids: Vec<HouseholdId>,
    mut run: impl FnMut(HouseholdId) -> BoxFuture<'a, Result<RetentionResult>>,
) -> Result<RetentionResult> {
    let mut total = RetentionResult::default();
    let mut first_err = None;
    for id in ids {
        match run(id).await {
            Ok(r) => {
                total.expired += r.expired;
                total.purged += r.purged;
            }
            Err(e) => {
                tracing::warn!(household = %id, error = %e, "mail.retention failed for a household");
                first_err.get_or_insert(e);
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(total),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_failing_household_does_not_stop_the_rest() {
        let ids: Vec<HouseholdId> = (0..3).map(|_| HouseholdId::new()).collect();
        let bad = ids[0];
        let seen = std::sync::Mutex::new(Vec::new());
        let r = run_each(ids.clone(), |id| {
            seen.lock().unwrap().push(id);
            Box::pin(async move {
                if id == bad {
                    Err(crate::MailError::NotFound)
                } else {
                    Ok(RetentionResult {
                        expired: 1,
                        purged: 2,
                    })
                }
            })
        })
        .await;
        assert!(matches!(r, Err(crate::MailError::NotFound)));
        assert_eq!(*seen.lock().unwrap(), ids);
    }

    #[tokio::test]
    async fn all_households_ok_sums_up() {
        let ids: Vec<HouseholdId> = (0..2).map(|_| HouseholdId::new()).collect();
        let r = run_each(ids, |_| {
            Box::pin(async {
                Ok(RetentionResult {
                    expired: 1,
                    purged: 2,
                })
            })
        })
        .await
        .unwrap();
        assert_eq!(
            r,
            RetentionResult {
                expired: 2,
                purged: 4
            }
        );
    }
}
