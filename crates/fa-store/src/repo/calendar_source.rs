use chrono::{DateTime, Utc};
use fa_domain::api::SourcePatch;
use fa_domain::{CalendarSource, CalendarSourceId, HouseholdId, Role, SecretId, SourceState};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum, parse_role};
use crate::{Result, StoreError, Versioned};

pub struct CalendarSourceRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    kind: String,
    name: String,
    owner_member: Option<Uuid>,
    oauth_account: Option<Uuid>,
    url: Option<String>,
    username: Option<String>,
    secret_id: Option<Uuid>,
    writable: bool,
    poll_minutes: i32,
    state: String,
    last_synced_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<CalendarSource> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: CalendarSource {
                id: r.id.into(),
                household_id: r.household_id.into(),
                kind: parse_enum(&r.kind)?,
                name: r.name,
                owner_member: r.owner_member.map(Into::into),
                oauth_account: r.oauth_account.map(Into::into),
                url: r.url,
                username: r.username,
                secret_id: r.secret_id.map(Into::into),
                writable: r.writable,
                poll_minutes: r.poll_minutes,
                state: parse_enum(&r.state)?,
                last_synced_at: r.last_synced_at,
                last_error: r.last_error,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl CalendarSourceRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        s: &CalendarSource,
    ) -> Result<Versioned<CalendarSource>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO calendar_source (id, household_id, kind, name, owner_member, oauth_account, url,
                                          username, secret_id, writable, poll_minutes, state)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id, household_id, kind, name, owner_member, oauth_account, url, username, secret_id,
                       writable, poll_minutes, state, last_synced_at, last_error, archived_at, version,
                       created_at, updated_at",
            s.id.as_uuid(),
            s.household_id.as_uuid(),
            enum_str(&s.kind),
            s.name,
            s.owner_member.map(|m| m.as_uuid()),
            s.oauth_account.map(|a| a.as_uuid()),
            s.url,
            s.username,
            s.secret_id.map(|x| x.as_uuid()),
            s.writable,
            s.poll_minutes,
            enum_str(&s.state)
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: CalendarSourceId,
    ) -> Result<Option<Versioned<CalendarSource>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, name, owner_member, oauth_account, url, username, secret_id,
                    writable, poll_minutes, state, last_synced_at, last_error, archived_at, version,
                    created_at, updated_at
             FROM calendar_source WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<Versioned<CalendarSource>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, name, owner_member, oauth_account, url, username, secret_id,
                    writable, poll_minutes, state, last_synced_at, last_error, archived_at, version,
                    created_at, updated_at
             FROM calendar_source WHERE $1 OR archived_at IS NULL ORDER BY created_at, id",
            include_archived
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// The role of the source owner (`None` for a household source), for the authz `Resource::Source`.
    pub async fn owner_role(conn: &mut PgConnection, id: CalendarSourceId) -> Result<Option<Role>> {
        let r = sqlx::query_scalar!(
            "SELECT m.role FROM calendar_source s JOIN member m ON m.id = s.owner_member WHERE s.id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        r.as_deref().map(parse_role).transpose()
    }

    /// Applies `name`, `poll_minutes` and (for non-ICS sources) `credentials.url`/`username`.
    /// Password and ICS URL are secrets: the caller stores them and calls [`Self::set_secret`].
    pub async fn update(
        conn: &mut PgConnection,
        id: CalendarSourceId,
        p: &SourcePatch,
        expected_version: i32,
    ) -> Result<i32> {
        let (url, username) = match &p.credentials {
            Some(c) => (c.url.as_deref(), c.username.as_deref()),
            None => (None, None),
        };
        let v = sqlx::query_scalar!(
            "UPDATE calendar_source SET
                    name = COALESCE($2, name), poll_minutes = COALESCE($3, poll_minutes),
                    url = CASE WHEN kind = 'caldav' THEN COALESCE($4, url) ELSE url END,
                    username = CASE WHEN kind = 'caldav' THEN COALESCE($5, username) ELSE username END,
                    version = version + 1
             WHERE id = $1 AND version = $6
             RETURNING version",
            id.as_uuid(),
            p.name,
            p.poll_minutes,
            url,
            username,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "calendar_source", id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: CalendarSourceId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar_source SET archived_at = COALESCE(archived_at, now()), version = version + 1
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

    pub async fn set_enabled(
        conn: &mut PgConnection,
        id: CalendarSourceId,
        enabled: bool,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar_source SET enabled = $2, version = version + 1 WHERE id = $1",
            id.as_uuid(),
            enabled
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
        id: CalendarSourceId,
        secret_id: SecretId,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar_source SET secret_id = $2, version = version + 1 WHERE id = $1",
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

    /// Records the outcome of a sync run. `synced_at` is only set on success (`state = ok`) by callers.
    pub async fn set_sync_result(
        conn: &mut PgConnection,
        id: CalendarSourceId,
        state: SourceState,
        last_error: Option<&str>,
        synced_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar_source SET state = $2, last_error = $3,
                    last_synced_at = COALESCE($4, last_synced_at), version = version + 1
             WHERE id = $1",
            id.as_uuid(),
            enum_str(&state),
            last_error,
            synced_at
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

    /// Enabled, non-archived sources whose last sync plus `poll_minutes` has passed (or never ran).
    /// Use a system tx: it spans households.
    pub async fn due(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(HouseholdId, CalendarSourceId)>> {
        let rows = sqlx::query!(
            "SELECT household_id, id FROM calendar_source
             WHERE enabled AND archived_at IS NULL
               AND (last_synced_at IS NULL OR last_synced_at + make_interval(mins => poll_minutes) <= $1)
             ORDER BY last_synced_at NULLS FIRST, id",
            now
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.household_id.into(), r.id.into()))
            .collect())
    }
}
