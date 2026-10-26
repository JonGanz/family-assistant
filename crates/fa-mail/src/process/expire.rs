//! Job `inbox.expire` (D-061): stale proposals become `expired` or `skipped`.

use chrono::{DateTime, Duration, Utc};
use fa_domain::TaskStatus;
use fa_store::{ChangeBus, ContactSuggestionRepo, EventRepo, HouseholdRepo, Store, TaskRepo};

use crate::Result;

/// Expires stale proposals of every household. Returns how many items changed.
pub async fn expire_all(store: &Store, now: DateTime<Utc>) -> Result<u64> {
    let households = {
        let mut tx = store.tx_system().await?;
        HouseholdRepo::list(&mut tx).await?
    };
    let mut total = 0;
    for h in households {
        let hh = h.value;
        let mut tx = store.tx_for(hh.id).await?;
        let mut n = EventRepo::expire_past(&mut tx, now).await?;
        let task_cutoff = now - Duration::days(hh.inbox.task_expiry_days.into());
        for t in TaskRepo::suggested_email_older_than(&mut tx, task_cutoff).await? {
            TaskRepo::set_status(&mut tx, t, TaskStatus::Skipped, None, None, None).await?;
            n += 1;
        }
        let contact_cutoff = now - Duration::days(hh.inbox.contact_expiry_days.into());
        n += ContactSuggestionRepo::expire_older_than(&mut tx, contact_cutoff).await?;
        if n > 0 {
            ChangeBus::publish(&mut tx, hh.id, "inbox.updated", &serde_json::json!({})).await?;
        }
        tx.commit().await?;
        total += n;
    }
    Ok(total)
}
