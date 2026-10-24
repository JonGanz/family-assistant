//! Bridge from Postgres `NOTIFY fa_change` to the in-process SSE [`Hub`] (arch/Feeds.md section 6).

use std::sync::Arc;
use std::time::Duration;

use fa_store::{CHANNEL, ChangeMessage, HouseholdRepo, Store};
use sqlx::postgres::{PgListener, PgPoolOptions};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::Hub;

const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Parses one NOTIFY payload and pushes it into the hub. Malformed payloads are logged and dropped.
pub fn deliver(hub: &Hub, payload: &str) {
    match serde_json::from_str::<ChangeMessage>(payload) {
        Ok(m) => {
            hub.publish(m.household, &m.kind, m.data);
        }
        Err(e) => tracing::warn!(error = %e, "dropping malformed fa_change payload"),
    }
}

/// Tells every client of every household to refetch (events may have been missed).
async fn broadcast_resync(store: &Store, hub: &Hub) {
    let households = async {
        let mut tx = store.tx_system().await?;
        let all = HouseholdRepo::list(&mut tx).await?;
        tx.rollback().await?;
        Ok::<_, fa_store::StoreError>(all)
    }
    .await;
    match households {
        Ok(all) => {
            for h in all {
                hub.publish(h.value.id, "resync", serde_json::json!({}));
            }
        }
        Err(e) => tracing::warn!(error = %e, "could not list households for resync"),
    }
}

/// The listener lives on its own connection, so it never takes a slot from the request pool.
async fn connect(store: &Store) -> Result<PgListener, sqlx::Error> {
    let options = store.pool().connect_options().as_ref().clone();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut l = PgListener::connect_with(&pool).await?;
    l.listen(CHANNEL).await?;
    Ok(l)
}

/// Listens until `cancel` fires. After a lost connection it reconnects with exponential backoff
/// (capped at 30 s) and then broadcasts `resync`.
pub async fn run_bridge(store: Store, hub: Hub, cancel: CancellationToken) {
    run_bridge_signalled(store, hub, cancel, None).await;
}

/// Like [`run_bridge`]; `ready` is notified each time `LISTEN` is established (tests wait for it).
pub async fn run_bridge_signalled(
    store: Store,
    hub: Hub,
    cancel: CancellationToken,
    ready: Option<Arc<Notify>>,
) {
    let mut backoff = Duration::from_millis(500);
    let mut reconnect = false;
    loop {
        let listener = tokio::select! {
            _ = cancel.cancelled() => return,
            r = connect(&store) => r,
        };
        let mut listener = match listener {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(error = %e, retry_in = ?backoff, "fa_change listener cannot connect");
                reconnect = true;
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        backoff = Duration::from_millis(500);
        if reconnect {
            tracing::info!("fa_change listener reconnected");
            broadcast_resync(&store, &hub).await;
        } else {
            tracing::info!("fa_change listener started");
        }
        reconnect = true;
        if let Some(r) = &ready {
            r.notify_one();
        }
        loop {
            let next = tokio::select! {
                _ = cancel.cancelled() => return,
                r = listener.try_recv() => r,
            };
            match next {
                Ok(Some(n)) => deliver(&hub, n.payload()),
                // The connection was lost; sqlx would silently reconnect and drop notifications,
                // so start over and resync instead.
                Ok(None) => {
                    tracing::warn!("fa_change listener lost its connection");
                    break;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "fa_change listener failed");
                    break;
                }
            }
        }
    }
}
