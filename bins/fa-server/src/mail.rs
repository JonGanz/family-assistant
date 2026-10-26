//! fa-mail glue the server owns: handler registration and the `mail` operator commands.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use fa_domain::{HouseholdId, MailSourceId, MessageId};
use fa_jobs::JobHandler;
use fa_mail::MailDeps;
use fa_mail::process::ProcessService;
use fa_mail::sync::SyncService;
use fa_store::{HouseholdRepo, MailSourceRepo, MessageRepo, Store};

use crate::cli::MailAction;

pub fn process_service(deps: &Arc<MailDeps>) -> ProcessService {
    fa_mail::process::process_service(deps)
}

/// All fa-mail job handlers.
pub fn handlers(deps: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    fa_mail::handlers(deps)
}

/// Processes every pending message of the household in this process. Returns `(ok, failed)`.
async fn process_pending(
    store: &Store,
    deps: &Arc<MailDeps>,
    household: HouseholdId,
) -> Result<(usize, usize)> {
    let svc = process_service(deps);
    let (mut ok, mut failed) = (0, 0);
    // A failing message stays pending: remember the ids already tried so the loop ends.
    let mut tried: std::collections::HashSet<MessageId> = Default::default();
    loop {
        let batch = {
            let mut tx = store.tx_for(household).await?;
            let ids = MessageRepo::pending(&mut tx, 200).await?;
            tx.rollback().await?;
            ids.into_iter()
                .filter(|i| !tried.contains(i))
                .collect::<Vec<_>>()
        };
        if batch.is_empty() {
            return Ok((ok, failed));
        }
        for id in batch {
            tried.insert(id);
            match svc.process(household, id, None).await {
                Ok(_) => ok += 1,
                Err(e) => {
                    failed += 1;
                    tracing::warn!(message = %id, error = %e, "processing failed");
                }
            }
        }
    }
}

pub async fn run(
    store: &Store,
    deps: Arc<MailDeps>,
    household: HouseholdId,
    action: MailAction,
) -> Result<()> {
    {
        let mut tx = store.tx_for(household).await?;
        HouseholdRepo::get(&mut tx, household)
            .await?
            .ok_or_else(|| anyhow!("household {household} not found"))?;
        tx.rollback().await?;
    }
    match action {
        MailAction::SyncNow { source } => {
            let sources: Vec<(MailSourceId, String)> = {
                let mut tx = store.tx_for(household).await?;
                let all = MailSourceRepo::list(&mut tx, false).await?;
                tx.rollback().await?;
                all.into_iter()
                    .map(|(s, _)| (s.value.id, s.value.name))
                    .filter(|(_, n)| source.as_ref().is_none_or(|w| w == n))
                    .collect()
            };
            if sources.is_empty() {
                match source {
                    Some(n) => bail!("no mail source named '{n}'"),
                    None => bail!("no mail sources yet; connect one in Settings -> Mail"),
                }
            }
            let sync = SyncService::new(deps.clone());
            for (id, name) in sources {
                match sync.sync_source(household, id).await {
                    Ok(()) => println!("source '{name}': synced"),
                    Err(e) => println!("source '{name}': {e}"),
                }
            }
            let (ok, failed) = process_pending(store, &deps, household)
                .await
                .context("processing")?;
            println!("processed: {ok} message(s), {failed} failed");
        }
        MailAction::ProcessNow => {
            let (ok, failed) = process_pending(store, &deps, household)
                .await
                .context("processing")?;
            println!("processed: {ok} message(s), {failed} failed");
        }
    }
    Ok(())
}
