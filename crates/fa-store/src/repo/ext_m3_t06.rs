//! Repository extensions for M3 task T-06: household lookup for job payloads that carry only a
//! message or bill id (use a system transaction).

use fa_domain::{BillId, HouseholdId, MessageId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{BillRepo, MessageRepo};
use crate::Result;

impl BillRepo {
    /// The household of a bill (system transaction).
    pub async fn household_of(conn: &mut PgConnection, id: BillId) -> Result<Option<HouseholdId>> {
        let h: Option<Uuid> = sqlx::query_scalar("SELECT household_id FROM bill WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        Ok(h.map(Into::into))
    }
}

impl MessageRepo {
    /// The household of a message (system transaction).
    pub async fn household_of(
        conn: &mut PgConnection,
        id: MessageId,
    ) -> Result<Option<HouseholdId>> {
        let h: Option<Uuid> = sqlx::query_scalar("SELECT household_id FROM message WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        Ok(h.map(Into::into))
    }
}
