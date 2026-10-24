use chrono::{DateTime, Utc};
use fa_domain::feeds::OAuthAccount;
use fa_domain::{HouseholdId, MemberId, OAuthAccountId, SecretId, SourceState};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct OAuthAccountRepo;
pub struct OAuthPendingRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    provider: String,
    owner_member: Uuid,
    email: String,
    scopes: Vec<String>,
    secret_id: Option<Uuid>,
    state: String,
    last_error: Option<String>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<OAuthAccount> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: OAuthAccount {
                id: r.id.into(),
                household_id: r.household_id.into(),
                provider: r.provider,
                owner_member: r.owner_member.into(),
                email: r.email,
                scopes: r.scopes,
                secret_id: r.secret_id.map(Into::into),
                state: parse_enum(&r.state)?,
                last_error: r.last_error,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl OAuthAccountRepo {
    /// Keyed by (household, provider, email). A re-connect resets the state to `ok`.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert(
        conn: &mut PgConnection,
        household: HouseholdId,
        provider: &str,
        owner_member: MemberId,
        email: &str,
        scopes: &[String],
        secret_id: Option<SecretId>,
    ) -> Result<Versioned<OAuthAccount>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO oauth_account (id, household_id, provider, owner_member, email, scopes, secret_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (household_id, provider, email) DO UPDATE SET
                 owner_member = EXCLUDED.owner_member, scopes = EXCLUDED.scopes,
                 secret_id = COALESCE(EXCLUDED.secret_id, oauth_account.secret_id),
                 state = 'ok', last_error = NULL, version = oauth_account.version + 1
             RETURNING id, household_id, provider, owner_member, email, scopes, secret_id, state,
                       last_error, version, created_at, updated_at",
            OAuthAccountId::new().as_uuid(),
            household.as_uuid(),
            provider,
            owner_member.as_uuid(),
            email,
            scopes,
            secret_id.map(|s| s.as_uuid())
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: OAuthAccountId,
    ) -> Result<Option<Versioned<OAuthAccount>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, provider, owner_member, email, scopes, secret_id, state,
                    last_error, version, created_at, updated_at
             FROM oauth_account WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// `owners = None` lists all accounts of the household, otherwise only those of the given members.
    pub async fn list(
        conn: &mut PgConnection,
        owners: Option<&[MemberId]>,
    ) -> Result<Vec<Versioned<OAuthAccount>>> {
        let ids: Option<Vec<Uuid>> = owners.map(|o| o.iter().map(|m| m.as_uuid()).collect());
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, provider, owner_member, email, scopes, secret_id, state,
                    last_error, version, created_at, updated_at
             FROM oauth_account WHERE $1::uuid[] IS NULL OR owner_member = ANY($1)
             ORDER BY created_at, id",
            ids.as_deref()
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn set_state(
        conn: &mut PgConnection,
        id: OAuthAccountId,
        state: SourceState,
        last_error: Option<&str>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE oauth_account SET state = $2, last_error = $3, version = version + 1 WHERE id = $1",
            id.as_uuid(),
            enum_str(&state),
            last_error
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

    pub async fn set_secret(
        conn: &mut PgConnection,
        id: OAuthAccountId,
        secret_id: SecretId,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE oauth_account SET secret_id = $2, version = version + 1 WHERE id = $1",
            id.as_uuid(),
            secret_id.as_uuid()
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

/// Short-lived state of one OAuth authorisation flow.
#[derive(Debug, Clone)]
pub struct OAuthPending {
    pub state: String,
    pub household_id: HouseholdId,
    pub session_id: String,
    pub member: MemberId,
    pub pkce_verifier: String,
    pub return_to: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl OAuthPendingRepo {
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        conn: &mut PgConnection,
        state: &str,
        session_id: &str,
        member: MemberId,
        pkce_verifier: &str,
        return_to: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO oauth_pending (state, household_id, session_id, member_id, pkce_verifier, return_to, expires_at)
             SELECT $1, household_id, $2, id, $4, $5, $6 FROM member WHERE id = $3",
            state,
            session_id,
            member.as_uuid(),
            pkce_verifier,
            return_to,
            expires_at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Delete-returning, so a state can be used once. Expired rows are returned too: the caller checks `expires_at`.
    pub async fn take(conn: &mut PgConnection, state: &str) -> Result<Option<OAuthPending>> {
        let r = sqlx::query!(
            "DELETE FROM oauth_pending WHERE state = $1
             RETURNING state, household_id, session_id, member_id, pkce_verifier, return_to, expires_at",
            state
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| OAuthPending {
            state: r.state,
            household_id: r.household_id.into(),
            session_id: r.session_id,
            member: r.member_id.into(),
            pkce_verifier: r.pkce_verifier,
            return_to: r.return_to,
            expires_at: r.expires_at,
        }))
    }

    pub async fn delete_expired(conn: &mut PgConnection) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM oauth_pending WHERE expires_at <= now()")
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}
