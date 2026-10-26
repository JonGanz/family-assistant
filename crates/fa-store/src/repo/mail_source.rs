//! Mail sources (Gmail or IMAP mailboxes) and per-folder IMAP sync state.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MailSource, MailSourceId, MailSourceState, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct MailSourceRepo;

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    household_id: Uuid,
    provider: String,
    name: String,
    address: String,
    oauth_account_id: Option<Uuid>,
    imap_host: Option<String>,
    imap_port: Option<i32>,
    imap_username: Option<String>,
    secret_id: Option<Uuid>,
    folders: Vec<String>,
    archive_folder: Option<String>,
    gmail_ext: bool,
    read_only: bool,
    visibility: String,
    backfill_days: i32,
    query: Option<String>,
    poll_minutes: i32,
    history_id: Option<String>,
    state: String,
    last_synced_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const COLS: &str = "id, household_id, provider, name, address, oauth_account_id, imap_host, imap_port,
    imap_username, secret_id, folders, archive_folder, gmail_ext, read_only, visibility, backfill_days,
    query, poll_minutes, history_id, state, last_synced_at, last_error, archived_at, version,
    created_at, updated_at";

impl TryFrom<Row> for Versioned<MailSource> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: MailSource {
                id: r.id.into(),
                household_id: r.household_id.into(),
                provider: parse_enum(&r.provider)?,
                name: r.name,
                address: r.address,
                oauth_account_id: r.oauth_account_id.map(Into::into),
                imap_host: r.imap_host,
                imap_port: r.imap_port,
                imap_username: r.imap_username,
                secret_id: r.secret_id.map(Into::into),
                folders: r.folders,
                archive_folder: r.archive_folder,
                gmail_ext: r.gmail_ext,
                read_only: r.read_only,
                visibility: parse_enum(&r.visibility)?,
                backfill_days: r.backfill_days,
                query: r.query,
                poll_minutes: r.poll_minutes,
                history_id: r.history_id,
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

fn ids(v: &[MemberId]) -> Vec<Uuid> {
    v.iter().map(|m| m.as_uuid()).collect()
}

impl MailSourceRepo {
    /// Inserts the source and its owners. The address is stored lower-cased.
    pub async fn insert(
        conn: &mut PgConnection,
        s: &MailSource,
        owners: &[MemberId],
    ) -> Result<Versioned<MailSource>> {
        let row: Row = sqlx::query_as(&format!(
            "INSERT INTO mail_source (id, household_id, provider, name, address, oauth_account_id,
                    imap_host, imap_port, imap_username, secret_id, folders, archive_folder,
                    gmail_ext, read_only, visibility, backfill_days, query, poll_minutes, state)
             VALUES ($1, $2, $3, $4, lower($5), $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                     $17, $18, $19)
             RETURNING {COLS}"
        ))
        .bind(s.id.as_uuid())
        .bind(s.household_id.as_uuid())
        .bind(enum_str(&s.provider))
        .bind(&s.name)
        .bind(&s.address)
        .bind(s.oauth_account_id.map(|o| o.as_uuid()))
        .bind(&s.imap_host)
        .bind(s.imap_port)
        .bind(&s.imap_username)
        .bind(s.secret_id.map(|o| o.as_uuid()))
        .bind(&s.folders)
        .bind(&s.archive_folder)
        .bind(s.gmail_ext)
        .bind(s.read_only)
        .bind(enum_str(&s.visibility))
        .bind(s.backfill_days)
        .bind(&s.query)
        .bind(s.poll_minutes)
        .bind(enum_str(&s.state))
        .fetch_one(&mut *conn)
        .await?;
        Self::set_owners(conn, s.id, owners).await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: MailSourceId,
    ) -> Result<Option<Versioned<MailSource>>> {
        let row: Option<Row> =
            sqlx::query_as(&format!("SELECT {COLS} FROM mail_source WHERE id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Sources with their owners.
    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<(Versioned<MailSource>, Vec<MemberId>)>> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM mail_source WHERE $1 OR archived_at IS NULL ORDER BY created_at, id"
        ))
        .bind(include_archived)
        .fetch_all(&mut *conn)
        .await?;
        let owners: Vec<(Uuid, Uuid)> =
            sqlx::query_as("SELECT source_id, member_id FROM mail_source_owner ORDER BY member_id")
                .fetch_all(&mut *conn)
                .await?;
        let mut by_source: HashMap<Uuid, Vec<MemberId>> = HashMap::new();
        for (s, m) in owners {
            by_source.entry(s).or_default().push(m.into());
        }
        rows.into_iter()
            .map(|r| {
                let o = by_source.remove(&r.id).unwrap_or_default();
                Ok((r.try_into()?, o))
            })
            .collect()
    }

    /// Updates the editable settings (not provider, oauth account, history, sync stamps or owners).
    pub async fn update(
        conn: &mut PgConnection,
        s: &MailSource,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE mail_source SET name = $2, address = lower($3), imap_host = $4, imap_port = $5,
                    imap_username = $6, secret_id = $7, folders = $8, archive_folder = $9,
                    gmail_ext = $10, read_only = $11, visibility = $12, backfill_days = $13,
                    query = $14, poll_minutes = $15, state = $16, version = version + 1
             WHERE id = $1 AND version = $17 RETURNING version",
        )
        .bind(s.id.as_uuid())
        .bind(&s.name)
        .bind(&s.address)
        .bind(&s.imap_host)
        .bind(s.imap_port)
        .bind(&s.imap_username)
        .bind(s.secret_id.map(|o| o.as_uuid()))
        .bind(&s.folders)
        .bind(&s.archive_folder)
        .bind(s.gmail_ext)
        .bind(s.read_only)
        .bind(enum_str(&s.visibility))
        .bind(s.backfill_days)
        .bind(&s.query)
        .bind(s.poll_minutes)
        .bind(enum_str(&s.state))
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "mail_source", s.id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: MailSourceId) -> Result<()> {
        let n = sqlx::query(
            "UPDATE mail_source SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn owners(conn: &mut PgConnection, id: MailSourceId) -> Result<Vec<MemberId>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT member_id FROM mail_source_owner WHERE source_id = $1 ORDER BY member_id",
        )
        .bind(id.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Replaces the owners.
    pub async fn set_owners(
        conn: &mut PgConnection,
        id: MailSourceId,
        owners: &[MemberId],
    ) -> Result<()> {
        sqlx::query("DELETE FROM mail_source_owner WHERE source_id = $1")
            .bind(id.as_uuid())
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO mail_source_owner (source_id, member_id, household_id)
             SELECT $1, m, (SELECT household_id FROM mail_source WHERE id = $1)
             FROM unnest($2::uuid[]) AS m ON CONFLICT DO NOTHING",
        )
        .bind(id.as_uuid())
        .bind(ids(owners))
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Sources due for a sync pass across all households (run in a system tx): not archived,
    /// state `ok`, never synced or `last_synced_at + poll_minutes <= now`.
    pub async fn due_for_sync(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(HouseholdId, MailSourceId)>> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT household_id, id FROM mail_source
             WHERE archived_at IS NULL AND state = 'ok'
               AND (last_synced_at IS NULL
                    OR last_synced_at + make_interval(mins => poll_minutes) <= $1)
             ORDER BY last_synced_at NULLS FIRST, id",
        )
        .bind(now)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(h, s)| (h.into(), s.into()))
            .collect())
    }

    /// Records a sync outcome. `history_id` and `synced_at` of `None` keep the stored value;
    /// `last_error` is always overwritten (`None` clears it).
    pub async fn set_sync_state(
        conn: &mut PgConnection,
        id: MailSourceId,
        history_id: Option<&str>,
        state: MailSourceState,
        last_error: Option<&str>,
        synced_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE mail_source SET history_id = COALESCE($2, history_id), state = $3,
                    last_error = $4, last_synced_at = COALESCE($5, last_synced_at)
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(history_id)
        .bind(enum_str(&state))
        .bind(last_error)
        .bind(synced_at)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Forgets the Gmail history cursor (a 404 from `history.list` forces a full sync).
    pub async fn clear_history_id(conn: &mut PgConnection, id: MailSourceId) -> Result<()> {
        sqlx::query("UPDATE mail_source SET history_id = NULL WHERE id = $1")
            .bind(id.as_uuid())
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn set_gmail_ext(conn: &mut PgConnection, id: MailSourceId, on: bool) -> Result<()> {
        sqlx::query("UPDATE mail_source SET gmail_ext = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(on)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Non-archived sources the member owns, or that are visible to the household.
    pub async fn readable_by(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Vec<MailSourceId>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT s.id FROM mail_source s
             WHERE s.archived_at IS NULL
               AND (s.visibility = 'household'
                    OR EXISTS (SELECT 1 FROM mail_source_owner o
                               WHERE o.source_id = s.id AND o.member_id = $1))
             ORDER BY s.created_at, s.id",
        )
        .bind(member.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Messages per source.
    pub async fn message_counts(conn: &mut PgConnection) -> Result<HashMap<MailSourceId, i64>> {
        let rows: Vec<(Uuid, i64)> =
            sqlx::query_as("SELECT source_id, count(*) FROM message GROUP BY source_id")
                .fetch_all(conn)
                .await?;
        Ok(rows.into_iter().map(|(s, n)| (s.into(), n)).collect())
    }
}

/// IMAP sync position of one folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailFolderState {
    pub source_id: MailSourceId,
    pub folder: String,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub highestmodseq: Option<i64>,
}

pub struct MailFolderStateRepo;

type FolderRow = (Uuid, String, i64, i64, Option<i64>);

fn folder(r: FolderRow) -> MailFolderState {
    MailFolderState {
        source_id: r.0.into(),
        folder: r.1,
        uidvalidity: r.2,
        uidnext: r.3,
        highestmodseq: r.4,
    }
}

impl MailFolderStateRepo {
    pub async fn get(
        conn: &mut PgConnection,
        source: MailSourceId,
        folder_name: &str,
    ) -> Result<Option<MailFolderState>> {
        let row: Option<FolderRow> = sqlx::query_as(
            "SELECT source_id, folder, uidvalidity, uidnext, highestmodseq
             FROM mail_folder_state WHERE source_id = $1 AND folder = $2",
        )
        .bind(source.as_uuid())
        .bind(folder_name)
        .fetch_optional(conn)
        .await?;
        Ok(row.map(folder))
    }

    pub async fn list(
        conn: &mut PgConnection,
        source: MailSourceId,
    ) -> Result<Vec<MailFolderState>> {
        let rows: Vec<FolderRow> = sqlx::query_as(
            "SELECT source_id, folder, uidvalidity, uidnext, highestmodseq
             FROM mail_folder_state WHERE source_id = $1 ORDER BY folder",
        )
        .bind(source.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(folder).collect())
    }

    pub async fn upsert(conn: &mut PgConnection, s: &MailFolderState) -> Result<()> {
        let n = sqlx::query(
            "INSERT INTO mail_folder_state (source_id, folder, household_id, uidvalidity, uidnext,
                                            highestmodseq)
             SELECT $1, $2, household_id, $3, $4, $5 FROM mail_source WHERE id = $1
             ON CONFLICT (source_id, folder) DO UPDATE SET
                 uidvalidity = EXCLUDED.uidvalidity, uidnext = EXCLUDED.uidnext,
                 highestmodseq = EXCLUDED.highestmodseq",
        )
        .bind(s.source_id.as_uuid())
        .bind(&s.folder)
        .bind(s.uidvalidity)
        .bind(s.uidnext)
        .bind(s.highestmodseq)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Forgets a folder's position (UIDVALIDITY changed).
    pub async fn reset(
        conn: &mut PgConnection,
        source: MailSourceId,
        folder_name: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM mail_folder_state WHERE source_id = $1 AND folder = $2")
            .bind(source.as_uuid())
            .bind(folder_name)
            .execute(conn)
            .await?;
        Ok(())
    }
}
