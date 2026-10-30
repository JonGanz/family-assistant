use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MemberId, UserAccount, UserAccountId};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Result, StoreError, Versioned};

pub struct UserAccountRepo;

#[derive(Debug, Clone, Copy)]
pub struct AccountLockout {
    pub failed_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

struct Row {
    id: Uuid,
    household_id: Uuid,
    member_id: Uuid,
    username: String,
    password_hash: String,
    disabled_at: Option<DateTime<Utc>>,
    initial_password: bool,
    created_at: DateTime<Utc>,
    version: i32,
}

impl From<Row> for Versioned<UserAccount> {
    fn from(r: Row) -> Self {
        Versioned {
            version: r.version,
            value: UserAccount {
                id: r.id.into(),
                household_id: r.household_id.into(),
                member_id: r.member_id.into(),
                username: r.username,
                password_hash: r.password_hash,
                disabled_at: r.disabled_at,
                initial_password: r.initial_password,
                created_at: r.created_at,
            },
        }
    }
}

impl UserAccountRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        a: &UserAccount,
    ) -> Result<Versioned<UserAccount>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO user_account (id, household_id, member_id, username, password_hash, initial_password)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, household_id, member_id, username, password_hash, disabled_at, initial_password, created_at, version",
            a.id.as_uuid(),
            a.household_id.as_uuid(),
            a.member_id.as_uuid(),
            a.username,
            a.password_hash,
            a.initial_password
        )
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: UserAccountId,
    ) -> Result<Option<Versioned<UserAccount>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, username, password_hash, disabled_at, initial_password, created_at, version
             FROM user_account WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn get_by_member(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Option<Versioned<UserAccount>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, username, password_hash, disabled_at, initial_password, created_at, version
             FROM user_account WHERE member_id = $1",
            member.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Case-insensitive login lookup. Pass `household` when known; in a system tx with `None`
    /// the oldest matching account across households is returned (single-household installs).
    pub async fn find_by_username(
        conn: &mut PgConnection,
        household: Option<HouseholdId>,
        username: &str,
    ) -> Result<Option<Versioned<UserAccount>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, username, password_hash, disabled_at, initial_password, created_at, version
             FROM user_account
             WHERE lower(username) = lower($2) AND ($1::uuid IS NULL OR household_id = $1)
             ORDER BY created_at LIMIT 1",
            household.map(|h| h.as_uuid()),
            username
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn list(conn: &mut PgConnection) -> Result<Vec<Versioned<UserAccount>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, username, password_hash, disabled_at, initial_password, created_at, version
             FROM user_account ORDER BY created_at, id"
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Changes the password hash (version-checked) and clears lockout. Returns the new version.
    pub async fn update_password(
        conn: &mut PgConnection,
        id: UserAccountId,
        password_hash: &str,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE user_account SET password_hash = $2, failed_attempts = 0, locked_until = NULL,
                    version = version + 1
             WHERE id = $1 AND version = $3 RETURNING version",
            id.as_uuid(),
            password_hash,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(super::conflict_or_missing(conn, "user_account", id.as_uuid()).await),
        }
    }

    /// Sets or clears the "password chosen by another adult" flag.
    pub async fn set_initial_password(
        conn: &mut PgConnection,
        id: UserAccountId,
        initial: bool,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE user_account SET initial_password = $2 WHERE id = $1",
            id.as_uuid(),
            initial
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Soft delete: disables the account. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: UserAccountId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE user_account SET disabled_at = COALESCE(disabled_at, now()), version = version + 1
             WHERE id = $1",
            id.as_uuid()
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

    /// Locks the account row (`FOR UPDATE`) until the transaction ends, so parallel attempts
    /// serialise between this check and `record_failure`.
    pub async fn lockout(conn: &mut PgConnection, id: UserAccountId) -> Result<AccountLockout> {
        let r = sqlx::query!(
            "SELECT failed_attempts, locked_until FROM user_account WHERE id = $1 FOR UPDATE",
            id.as_uuid()
        )
        .fetch_one(conn)
        .await?;
        Ok(AccountLockout {
            failed_attempts: r.failed_attempts,
            locked_until: r.locked_until,
        })
    }

    /// Records a failed login; locks for `lock_secs` when `max_attempts` is reached (counter resets).
    /// An account that is already locked is left as it is.
    pub async fn record_failure(
        conn: &mut PgConnection,
        id: UserAccountId,
        max_attempts: i32,
        lock_secs: i64,
    ) -> Result<AccountLockout> {
        let r = sqlx::query!(
            "UPDATE user_account SET
                 failed_attempts = CASE WHEN failed_attempts + 1 >= $2 THEN 0 ELSE failed_attempts + 1 END,
                 locked_until = CASE WHEN failed_attempts + 1 >= $2
                                     THEN now() + make_interval(secs => $3) ELSE locked_until END
             WHERE id = $1 AND (locked_until IS NULL OR locked_until <= now())
             RETURNING failed_attempts, locked_until",
            id.as_uuid(),
            max_attempts,
            lock_secs as f64
        )
        .fetch_optional(&mut *conn)
        .await?;
        match r {
            Some(r) => Ok(AccountLockout {
                failed_attempts: r.failed_attempts,
                locked_until: r.locked_until,
            }),
            None => Self::lockout(conn, id).await,
        }
    }

    pub async fn clear_failures(conn: &mut PgConnection, id: UserAccountId) -> Result<()> {
        sqlx::query!(
            "UPDATE user_account SET failed_attempts = 0, locked_until = NULL WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}
