use chrono::{DateTime, Utc};
use fa_domain::{AuditEntry, HouseholdId};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::Result;

/// Append-only: there is deliberately no update or delete.
pub struct AuditRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    actor_member_id: Option<Uuid>,
    device_id: Option<Uuid>,
    action: String,
    subject_type: String,
    subject_id: Option<String>,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    at: DateTime<Utc>,
}

impl From<Row> for AuditEntry {
    fn from(r: Row) -> Self {
        AuditEntry {
            id: r.id.into(),
            household_id: r.household_id.into(),
            actor_member: r.actor_member_id.map(Into::into),
            device: r.device_id.map(Into::into),
            action: r.action,
            subject_type: r.subject_type,
            subject_id: r.subject_id,
            before: r.before,
            after: r.after,
            at: r.at,
        }
    }
}

impl AuditRepo {
    pub async fn insert(conn: &mut PgConnection, e: &AuditEntry) -> Result<()> {
        sqlx::query!(
            "INSERT INTO audit_entry (id, household_id, actor_member_id, device_id, action, subject_type,
                                      subject_id, before, after, at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            e.id.as_uuid(),
            e.household_id.as_uuid(),
            e.actor_member.map(|m| m.as_uuid()),
            e.device.map(|d| d.as_uuid()),
            e.action,
            e.subject_type,
            e.subject_id,
            e.before,
            e.after,
            e.at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Newest first.
    pub async fn list(
        conn: &mut PgConnection,
        household: HouseholdId,
        limit: i64,
    ) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, actor_member_id, device_id, action, subject_type, subject_id,
                    before, after, at
             FROM audit_entry WHERE household_id = $1 ORDER BY at DESC, id DESC LIMIT $2",
            household.as_uuid(),
            limit
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
