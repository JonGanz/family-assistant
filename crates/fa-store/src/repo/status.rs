use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::Result;

/// System-wide key/value status (health and backup checks). Not RLS-scoped.
pub struct SystemStatusRepo;

impl SystemStatusRepo {
    pub async fn set(conn: &mut PgConnection, key: &str, value: &serde_json::Value) -> Result<()> {
        sqlx::query!(
            "INSERT INTO system_status (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
            key,
            value
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn get(
        conn: &mut PgConnection,
        key: &str,
    ) -> Result<Option<(serde_json::Value, DateTime<Utc>)>> {
        let r = sqlx::query!(
            "SELECT value, updated_at FROM system_status WHERE key = $1",
            key
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| (r.value, r.updated_at)))
    }
}
