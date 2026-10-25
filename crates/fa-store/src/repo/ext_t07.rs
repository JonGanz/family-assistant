//! Repository extensions added by M2 t07 (see plan/M2).

use chrono::{DateTime, Utc};
use fa_domain::TaskDefinitionId;
use sqlx::PgConnection;

use super::TaskDefinitionRepo;
use crate::Result;

impl TaskDefinitionRepo {
    /// Stores a condition evaluation: `None` marks an unparsable condition (`condition_ok = NULL`).
    pub async fn set_condition_outcome(
        conn: &mut PgConnection,
        id: TaskDefinitionId,
        ok: Option<bool>,
        checked_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE task_definition SET condition_ok = $2, condition_checked_at = $3, updated_at = now()
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(ok)
        .bind(checked_at)
        .execute(conn)
        .await?;
        Ok(())
    }
}
