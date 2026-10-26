//! Repository extensions for M3 task T-09 (mail sync side).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MailSourceId, MessageId, SecretId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{MailSourceRepo, MessageRepo};
use crate::{Result, StoreError};

fn uuids(v: &[MessageId]) -> Vec<Uuid> {
    v.iter().map(|x| x.as_uuid()).collect()
}

impl MailSourceRepo {
    /// Like `due_for_sync` with each source's `poll_minutes` (for the per-slot job key).
    pub async fn due_with_poll(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<(HouseholdId, MailSourceId, i32)>> {
        let rows: Vec<(Uuid, Uuid, i32)> = sqlx::query_as(
            "SELECT household_id, id, poll_minutes FROM mail_source
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
            .map(|(h, s, p)| (h.into(), s.into(), p))
            .collect())
    }

    /// Points the source at its (re)written credential.
    pub async fn set_secret(
        conn: &mut PgConnection,
        id: MailSourceId,
        secret: SecretId,
    ) -> Result<()> {
        let n = sqlx::query("UPDATE mail_source SET secret_id = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(secret.as_uuid())
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

impl MessageRepo {
    /// Which of the provider ids are already stored for the source.
    pub async fn existing_remote_ids(
        conn: &mut PgConnection,
        source: MailSourceId,
        remote_ids: &[String],
    ) -> Result<HashSet<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT remote_id FROM message WHERE source_id = $1 AND remote_id = ANY($2)",
        )
        .bind(source.as_uuid())
        .bind(remote_ids)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().collect())
    }

    /// A message with this `Message-ID` header is already stored for the source.
    pub async fn exists_rfc822(
        conn: &mut PgConnection,
        source: MailSourceId,
        rfc822_id: &str,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM message WHERE source_id = $1 AND rfc822_id = $2)",
        )
        .bind(source.as_uuid())
        .bind(rfc822_id)
        .fetch_one(conn)
        .await?)
    }

    /// `(id, remote_id, labels)` of the stored messages among `remote_ids`.
    pub async fn label_state(
        conn: &mut PgConnection,
        source: MailSourceId,
        remote_ids: &[String],
    ) -> Result<Vec<(MessageId, String, Vec<String>)>> {
        let rows: Vec<(Uuid, String, Vec<String>)> = sqlx::query_as(
            "SELECT id, remote_id, gmail_labels FROM message
             WHERE source_id = $1 AND remote_id = ANY($2)",
        )
        .bind(source.as_uuid())
        .bind(remote_ids)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(|(i, r, l)| (i.into(), r, l)).collect())
    }

    /// A moved IMAP message: its provider id and folder changed.
    pub async fn relocate(
        conn: &mut PgConnection,
        id: MessageId,
        remote_id: &str,
        folder: &str,
    ) -> Result<()> {
        let n = sqlx::query("UPDATE message SET remote_id = $2, folder = $3 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(remote_id)
            .bind(folder)
            .execute(conn)
            .await?
            .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Marks pending messages `expired`. Returns how many changed.
    pub async fn expire(conn: &mut PgConnection, ids: &[MessageId]) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE message SET processing_state = 'expired'
             WHERE id = ANY($1) AND processing_state = 'pending'",
        )
        .bind(uuids(ids))
        .execute(conn)
        .await?
        .rows_affected())
    }
}
