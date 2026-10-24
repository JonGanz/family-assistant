use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MemberId, UserAccountId};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::Result;

/// Server-side session row. `id` is the opaque cookie value (not a UUID), so `SessionId` is
/// not used for it. `data` is the serialized tower-sessions record.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub account_id: UserAccountId,
    pub data: Vec<u8>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct SessionRepo;

struct Row {
    id: String,
    household_id: Uuid,
    member_id: Uuid,
    account_id: Uuid,
    data: Vec<u8>,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl From<Row> for SessionRecord {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            household_id: r.household_id.into(),
            member_id: r.member_id.into(),
            account_id: r.account_id.into(),
            data: r.data,
            user_agent: r.user_agent,
            created_at: r.created_at,
            last_seen: r.last_seen,
            expires_at: r.expires_at,
        }
    }
}

impl SessionRepo {
    /// Insert or replace (rolling sessions re-save with a new expiry).
    pub async fn put(conn: &mut PgConnection, s: &SessionRecord) -> Result<()> {
        sqlx::query!(
            "INSERT INTO session (id, household_id, member_id, account_id, data, user_agent, last_seen, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, user_agent = EXCLUDED.user_agent,
                 last_seen = EXCLUDED.last_seen, expires_at = EXCLUDED.expires_at",
            s.id,
            s.household_id.as_uuid(),
            s.member_id.as_uuid(),
            s.account_id.as_uuid(),
            s.data,
            s.user_agent,
            s.last_seen,
            s.expires_at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Returns the session only if it has not expired.
    pub async fn get(conn: &mut PgConnection, id: &str) -> Result<Option<SessionRecord>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, account_id, data, user_agent, created_at, last_seen, expires_at
             FROM session WHERE id = $1 AND expires_at > now()",
            id
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn delete(conn: &mut PgConnection, id: &str) -> Result<bool> {
        Ok(sqlx::query!("DELETE FROM session WHERE id = $1", id)
            .execute(conn)
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn list_for_member(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Vec<SessionRecord>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, account_id, data, user_agent, created_at, last_seen, expires_at
             FROM session WHERE member_id = $1 AND expires_at > now() ORDER BY created_at DESC",
            member.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn delete_for_member(conn: &mut PgConnection, member: MemberId) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM session WHERE member_id = $1", member.as_uuid())
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    /// Cleanup job (system tx): removes expired sessions across households.
    pub async fn delete_expired(conn: &mut PgConnection) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM session WHERE expires_at <= now()")
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}
