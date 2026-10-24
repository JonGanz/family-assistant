//! Cross-process change events: `pg_notify` inside the writer's transaction (arch/Feeds.md section 6).

use fa_domain::HouseholdId;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::{Result, StoreError};

pub const CHANNEL: &str = "fa_change";

/// Payloads above this size are rejected (NOTIFY allows 8 KB, we stay well below).
const MAX_PAYLOAD_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeMessage {
    pub household: HouseholdId,
    pub kind: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub struct ChangeBus;

impl ChangeBus {
    /// Delivered on commit of the surrounding transaction, dropped on rollback.
    pub async fn publish(
        conn: &mut PgConnection,
        household: HouseholdId,
        kind: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        let payload = serde_json::to_string(&ChangeMessage {
            household,
            kind: kind.to_string(),
            data: data.clone(),
        })
        .map_err(|e| StoreError::Corrupt(e.to_string()))?;
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(StoreError::PayloadTooLarge(payload.len()));
        }
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(CHANNEL)
            .bind(payload)
            .execute(conn)
            .await?;
        Ok(())
    }
}
