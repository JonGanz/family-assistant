//! Repository extensions for M3 task T-11: calendar routing targets and proposal edits.

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{CalendarId, CalendarSourceId, EventId, HouseholdId, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{CalendarRepo, EventRepo};
use crate::{Result, StoreError};

/// `(calendar, source, is_default_target, members)`; plain tuple because this module is not re-exported.
pub type WritableCalendar = (CalendarId, CalendarSourceId, bool, Vec<MemberId>);

#[derive(sqlx::FromRow)]
struct WcRow {
    id: Uuid,
    source_id: Uuid,
    is_default_target: bool,
    member_ids: Vec<Uuid>,
}

impl CalendarRepo {
    /// Non-archived, enabled calendars of non-archived writable sources, oldest first.
    pub async fn writable_targets(conn: &mut PgConnection) -> Result<Vec<WritableCalendar>> {
        let rows: Vec<WcRow> = sqlx::query_as(
            "SELECT c.id, c.source_id, c.is_default_target,
                    ARRAY(SELECT member_id FROM calendar_member WHERE calendar_id = c.id
                          ORDER BY member_id) AS member_ids
             FROM calendar c JOIN calendar_source s ON s.id = c.source_id
             WHERE c.archived_at IS NULL AND c.enabled AND s.archived_at IS NULL AND s.writable
             ORDER BY c.created_at, c.id",
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id.into(),
                    r.source_id.into(),
                    r.is_default_target,
                    r.member_ids.into_iter().map(Into::into).collect(),
                )
            })
            .collect())
    }
}

impl EventRepo {
    /// Applies approval edits to a proposed event.
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_edit(
        conn: &mut PgConnection,
        id: EventId,
        title: &str,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
        all_day: bool,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE event SET title = $2, start_at = $3, end_at = $4, all_day = $5,
                    start_date = $6, end_date = $7, version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(title)
        .bind(start_at)
        .bind(end_at)
        .bind(all_day)
        .bind(start_date)
        .bind(end_date)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Replaces the event's own members.
    pub async fn replace_members(
        conn: &mut PgConnection,
        household: HouseholdId,
        id: EventId,
        members: &[MemberId],
    ) -> Result<()> {
        sqlx::query("DELETE FROM event_member WHERE event_id = $1")
            .bind(id.as_uuid())
            .execute(&mut *conn)
            .await?;
        let ids: Vec<Uuid> = members.iter().map(|m| m.as_uuid()).collect();
        sqlx::query(
            "INSERT INTO event_member (event_id, member_id, household_id)
             SELECT $1, m, $2 FROM unnest($3::uuid[]) AS m ON CONFLICT DO NOTHING",
        )
        .bind(id.as_uuid())
        .bind(household.as_uuid())
        .bind(&ids)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Merges `{key: value}` into `source_ref` (kept apart from the sync-managed `external_url`).
    pub async fn merge_source_ref(
        conn: &mut PgConnection,
        id: EventId,
        patch: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE event SET source_ref = COALESCE(source_ref, '{}'::jsonb) || $2::jsonb,
                    version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(patch)
        .execute(conn)
        .await?;
        Ok(())
    }
}
