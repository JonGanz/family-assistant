use chrono::{DateTime, Utc};
use fa_domain::api::CalendarPatch;
use fa_domain::{Calendar, CalendarId, CalendarSourceId, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::conflict_or_missing;
use crate::{Result, StoreError, Versioned};

pub struct CalendarRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    source_id: Uuid,
    remote_id: String,
    name: String,
    color: Option<String>,
    enabled: bool,
    is_default_target: bool,
    sync_state: Option<String>,
    last_full_sync_at: Option<DateTime<Utc>>,
    version: i32,
}

impl From<Row> for Versioned<Calendar> {
    fn from(r: Row) -> Self {
        Versioned {
            version: r.version,
            value: Calendar {
                id: r.id.into(),
                household_id: r.household_id.into(),
                source: r.source_id.into(),
                remote_id: r.remote_id,
                name: r.name,
                color: r.color,
                enabled: r.enabled,
                is_default_target: r.is_default_target,
                sync_state: r.sync_state,
                last_full_sync_at: r.last_full_sync_at,
            },
        }
    }
}

impl CalendarRepo {
    /// Inserts a newly discovered calendar as `enabled = false`, or refreshes the remote name of a known
    /// one (and un-archives it). The user's own colour choice is kept. `created` tells which happened.
    pub async fn upsert_discovered(
        conn: &mut PgConnection,
        source: CalendarSourceId,
        remote_id: &str,
        name: &str,
        color: Option<&str>,
    ) -> Result<(Versioned<Calendar>, bool)> {
        let r = sqlx::query!(
            r#"WITH src AS (SELECT household_id FROM calendar_source WHERE id = $2)
               INSERT INTO calendar (id, household_id, source_id, remote_id, name, color)
               SELECT $1, household_id, $2, $3, $4, $5 FROM src
               ON CONFLICT (source_id, remote_id) DO UPDATE SET
                   name = EXCLUDED.name, color = COALESCE(calendar.color, EXCLUDED.color),
                   archived_at = NULL,
                   version = CASE WHEN calendar.name IS DISTINCT FROM EXCLUDED.name
                                    OR calendar.archived_at IS NOT NULL
                                  THEN calendar.version + 1 ELSE calendar.version END
               RETURNING id, household_id, source_id, remote_id, name, color, enabled, is_default_target,
                         sync_state, last_full_sync_at, version, (xmax = 0) AS "created!""#,
            CalendarId::new().as_uuid(),
            source.as_uuid(),
            remote_id,
            name,
            color
        )
        .fetch_optional(conn)
        .await?
        .ok_or(StoreError::NotFound)?;
        let created = r.created;
        let row = Row {
            id: r.id,
            household_id: r.household_id,
            source_id: r.source_id,
            remote_id: r.remote_id,
            name: r.name,
            color: r.color,
            enabled: r.enabled,
            is_default_target: r.is_default_target,
            sync_state: r.sync_state,
            last_full_sync_at: r.last_full_sync_at,
            version: r.version,
        };
        Ok((row.into(), created))
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: CalendarId,
    ) -> Result<Option<Versioned<Calendar>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, source_id, remote_id, name, color, enabled, is_default_target,
                    sync_state, last_full_sync_at, version
             FROM calendar WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Non-archived calendars, of one source or of all.
    pub async fn list(
        conn: &mut PgConnection,
        source: Option<CalendarSourceId>,
    ) -> Result<Vec<Versioned<Calendar>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, source_id, remote_id, name, color, enabled, is_default_target,
                    sync_state, last_full_sync_at, version
             FROM calendar WHERE archived_at IS NULL AND ($1::uuid IS NULL OR source_id = $1)
             ORDER BY created_at, id",
            source.map(|s| s.as_uuid())
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Applies `name`, `color` and `enabled` (members go through [`Self::set_members`]).
    pub async fn update(
        conn: &mut PgConnection,
        id: CalendarId,
        p: &CalendarPatch,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE calendar SET name = COALESCE($2, name), color = COALESCE($3, color),
                    enabled = COALESCE($4, enabled), version = version + 1
             WHERE id = $1 AND version = $5
             RETURNING version",
            id.as_uuid(),
            p.name,
            p.color,
            p.enabled,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "calendar", id.as_uuid()).await),
        }
    }

    /// Archives (and disables) calendars of the source that the remote no longer lists.
    /// Returns the archived calendars' ids so the caller can drop their events.
    pub async fn archive_missing(
        conn: &mut PgConnection,
        source: CalendarSourceId,
        seen_remote_ids: &[String],
    ) -> Result<Vec<CalendarId>> {
        let rows = sqlx::query_scalar!(
            "UPDATE calendar SET archived_at = now(), enabled = false, version = version + 1
             WHERE source_id = $1 AND archived_at IS NULL AND NOT (remote_id = ANY($2))
             RETURNING id",
            source.as_uuid(),
            seen_remote_ids
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn set_enabled_all(
        conn: &mut PgConnection,
        source: CalendarSourceId,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE calendar SET enabled = $2, version = version + 1 WHERE source_id = $1 AND archived_at IS NULL",
            source.as_uuid(),
            enabled
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Stores the incremental-sync token and/or the time of the last full sync.
    /// `sync_state = None` clears the token. `full_sync_at = None` leaves the time unchanged.
    pub async fn set_sync_state(
        conn: &mut PgConnection,
        id: CalendarId,
        sync_state: Option<&str>,
        full_sync_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE calendar SET sync_state = $2, last_full_sync_at = COALESCE($3, last_full_sync_at)
             WHERE id = $1",
            id.as_uuid(),
            sync_state,
            full_sync_at
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

    /// Replaces the member links. An empty slice makes it a household calendar.
    pub async fn set_members(
        conn: &mut PgConnection,
        id: CalendarId,
        members: &[MemberId],
    ) -> Result<()> {
        let ids: Vec<Uuid> = members.iter().map(|m| m.as_uuid()).collect();
        sqlx::query!(
            "DELETE FROM calendar_member WHERE calendar_id = $1 AND NOT (member_id = ANY($2))",
            id.as_uuid(),
            &ids
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO calendar_member (calendar_id, member_id, household_id)
             SELECT $1, m.id, m.household_id FROM member m WHERE m.id = ANY($2)
             ON CONFLICT DO NOTHING",
            id.as_uuid(),
            &ids
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn members(conn: &mut PgConnection, id: CalendarId) -> Result<Vec<MemberId>> {
        let rows = sqlx::query_scalar!(
            "SELECT member_id FROM calendar_member WHERE calendar_id = $1 ORDER BY member_id",
            id.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
