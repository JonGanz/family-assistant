//! Repository extensions for M3 task T-12 (Wave 3-5): add `impl XxxRepo { ... }` blocks here.

use fa_domain::{CalendarId, CalendarSourceId, EventId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{CalendarRepo, CalendarSourceRepo, EventRepo};
use crate::{Result, StoreError};

impl CalendarSourceRepo {
    /// Sets whether the app may write events to the source's calendars.
    pub async fn set_writable(
        conn: &mut PgConnection,
        id: CalendarSourceId,
        writable: bool,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE calendar_source SET writable = $2, version = version + 1 WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(writable)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl CalendarRepo {
    /// Marks the calendar as the default target of extracted events (or not).
    pub async fn set_default_target(
        conn: &mut PgConnection,
        id: CalendarId,
        default_target: bool,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE calendar SET is_default_target = $2, version = version + 1 WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(default_target)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl EventRepo {
    /// Approved extracted events whose provider write failed: `(event, sender label)`.
    pub async fn failed_writes(conn: &mut PgConnection) -> Result<Vec<(EventId, Option<String>)>> {
        let rows: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT e.id, COALESCE(m.from_name, m.from_addr)
             FROM event e LEFT JOIN message m ON m.id = e.message_id
             WHERE e.origin = 'extracted' AND e.status = 'approved' AND e.write_error IS NOT NULL
             ORDER BY e.start_at, e.id",
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(|(i, f)| (i.into(), f)).collect())
    }
}
