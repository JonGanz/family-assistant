//! Repo additions for the household-config import/export (T-08).

use fa_domain::{CalendarSourceId, MemberId};
use sqlx::PgConnection;

use crate::{Result, StoreError};

pub struct ConfigExtRepo;

impl ConfigExtRepo {
    /// The source's `enabled` flag (not part of the `CalendarSource` entity).
    pub async fn source_enabled(conn: &mut PgConnection, id: CalendarSourceId) -> Result<bool> {
        sqlx::query_scalar!(
            "SELECT enabled FROM calendar_source WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?
        .ok_or(StoreError::NotFound)
    }

    /// Replaces owner, CalDAV url/username and the poll interval (config import).
    pub async fn set_source_config(
        conn: &mut PgConnection,
        id: CalendarSourceId,
        owner_member: Option<MemberId>,
        url: Option<&str>,
        username: Option<&str>,
        poll_minutes: i32,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar_source SET owner_member = $2, url = $3, username = $4, poll_minutes = $5,
                    version = version + 1
             WHERE id = $1",
            id.as_uuid(),
            owner_member.map(|m| m.as_uuid()),
            url,
            username,
            poll_minutes
        )
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
