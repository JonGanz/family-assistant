use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{CalendarId, Event, EventId, Role};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{parse_enum, parse_role};
use crate::{Result, StoreError};

pub struct EventRepo;

/// One synced occurrence, as produced by a connector. `instance_key` is `''` for a non-recurring event.
#[derive(Debug, Clone, PartialEq)]
pub struct NewOccurrence {
    pub remote_uid: String,
    pub instance_key: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub source_tz: Option<String>,
    pub is_private: bool,
    pub cancelled: bool,
    pub remote_etag: Option<String>,
    /// Link back to the event at its provider; stored as `source_ref {kind: calendar, external_url}`.
    pub external_url: Option<String>,
}

/// An event with what `fa_domain::calendar::view` and `EventView` need, loaded in one query.
#[derive(Debug, Clone)]
pub struct EventRow {
    pub event: Event,
    /// The calendar's colour.
    pub color: Option<String>,
    /// Calendar members (plus the event's own members) with their roles. Empty means a household calendar.
    pub members: Vec<(fa_domain::MemberId, Role)>,
    /// The role of the owner of the calendar's source (`None` for a household source).
    pub owner_role: Option<Role>,
}

impl EventRepo {
    /// Inserts or updates occurrences by (calendar, remote_uid, instance_key). A row whose content is
    /// unchanged is left alone (no version bump), so the count is the number inserted or changed.
    pub async fn upsert_synced(
        conn: &mut PgConnection,
        calendar: CalendarId,
        occurrences: &[NewOccurrence],
    ) -> Result<u64> {
        let household = sqlx::query_scalar!(
            "SELECT household_id FROM calendar WHERE id = $1",
            calendar.as_uuid()
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(StoreError::NotFound)?;
        let mut changed = 0;
        for o in occurrences {
            let dedupe_key = format!("{calendar}:{}:{}", o.remote_uid, o.instance_key);
            changed += sqlx::query!(
                "INSERT INTO event (id, household_id, calendar_id, remote_uid, instance_key, title,
                                    description, location, start_at, end_at, all_day, start_date, end_date,
                                    source_tz, is_private, cancelled, origin, dedupe_key, remote_etag, source_ref)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 'synced', $17, $18,
                         CASE WHEN $19::text IS NULL THEN NULL
                              ELSE jsonb_build_object('kind', 'calendar', 'external_url', $19::text) END)
                 ON CONFLICT (calendar_id, remote_uid, instance_key) DO UPDATE SET
                     title = EXCLUDED.title, description = EXCLUDED.description, location = EXCLUDED.location,
                     start_at = EXCLUDED.start_at, end_at = EXCLUDED.end_at, all_day = EXCLUDED.all_day,
                     start_date = EXCLUDED.start_date, end_date = EXCLUDED.end_date,
                     source_tz = EXCLUDED.source_tz, is_private = EXCLUDED.is_private,
                     cancelled = EXCLUDED.cancelled, remote_etag = EXCLUDED.remote_etag,
                     source_ref = EXCLUDED.source_ref,
                     version = event.version + 1
                 WHERE (event.title, event.description, event.location, event.start_at, event.end_at,
                        event.all_day, event.start_date, event.end_date, event.source_tz,
                        event.is_private, event.cancelled, event.remote_etag, event.source_ref)
                       IS DISTINCT FROM
                       (EXCLUDED.title, EXCLUDED.description, EXCLUDED.location, EXCLUDED.start_at,
                        EXCLUDED.end_at, EXCLUDED.all_day, EXCLUDED.start_date, EXCLUDED.end_date,
                        EXCLUDED.source_tz, EXCLUDED.is_private, EXCLUDED.cancelled, EXCLUDED.remote_etag,
                        EXCLUDED.source_ref)",
                EventId::new().as_uuid(),
                household,
                calendar.as_uuid(),
                o.remote_uid,
                o.instance_key,
                o.title,
                o.description,
                o.location,
                o.start_at,
                o.end_at,
                o.all_day,
                o.start_date,
                o.end_date,
                o.source_tz,
                o.is_private,
                o.cancelled,
                dedupe_key,
                o.remote_etag,
                o.external_url
            )
            .execute(&mut *conn)
            .await?
            .rows_affected();
        }
        Ok(changed)
    }

    /// Deletes synced rows of this calendar overlapping `[from, to)` that are not in `seen`
    /// (`(remote_uid, instance_key)` pairs).
    pub async fn delete_unseen_in_window(
        conn: &mut PgConnection,
        calendar: CalendarId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        seen: &[(String, String)],
    ) -> Result<u64> {
        let uids: Vec<String> = seen.iter().map(|s| s.0.clone()).collect();
        let keys: Vec<String> = seen.iter().map(|s| s.1.clone()).collect();
        Ok(sqlx::query!(
            "DELETE FROM event e
             WHERE e.calendar_id = $1 AND e.origin = 'synced'
               AND e.start_at < $3 AND e.end_at >= $2
               AND NOT EXISTS (SELECT 1 FROM unnest($4::text[], $5::text[]) AS s(uid, ik)
                               WHERE s.uid = e.remote_uid AND s.ik = e.instance_key)",
            calendar.as_uuid(),
            from,
            to,
            &uids,
            &keys
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Deletes one instance, or every instance when `instance_key` is `None`.
    pub async fn delete_by_uid(
        conn: &mut PgConnection,
        calendar: CalendarId,
        uid: &str,
        instance_key: Option<&str>,
    ) -> Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM event WHERE calendar_id = $1 AND origin = 'synced' AND remote_uid = $2
               AND ($3::text IS NULL OR instance_key = $3)",
            calendar.as_uuid(),
            uid,
            instance_key
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    pub async fn delete_calendar_events(
        conn: &mut PgConnection,
        calendar: CalendarId,
    ) -> Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM event WHERE calendar_id = $1 AND origin = 'synced'",
            calendar.as_uuid()
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Deletes synced occurrences that ended before `before`.
    pub async fn prune_before(conn: &mut PgConnection, before: DateTime<Utc>) -> Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM event WHERE origin = 'synced' AND end_at < $1",
            before
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Visible events overlapping `[from, to)`: not cancelled, not proposed or rejected, on enabled
    /// calendars of non-archived sources. `calendars = None` means all of them.
    pub async fn range(
        conn: &mut PgConnection,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        calendars: Option<&[CalendarId]>,
    ) -> Result<Vec<EventRow>> {
        let cals: Option<Vec<Uuid>> = calendars.map(|c| c.iter().map(|x| x.as_uuid()).collect());
        let rows = sqlx::query!(
            r#"SELECT e.id, e.household_id, e.calendar_id, e.remote_uid, e.instance_key, e.title,
                      e.description, e.location, e.start_at, e.end_at, e.all_day, e.start_date, e.end_date,
                      e.source_tz, e.is_private, e.cancelled, e.origin, e.status, e.confidence,
                      e.source_ref, e.dedupe_key, e.remote_etag, e.created_at, e.updated_at,
                      c.color AS "color?",
                      om.role AS "owner_role?",
                      ARRAY(SELECT m.id FROM member m WHERE m.id IN
                              (SELECT member_id FROM calendar_member WHERE calendar_id = e.calendar_id
                               UNION SELECT member_id FROM event_member WHERE event_id = e.id)
                            ORDER BY m.id) AS "member_ids!",
                      ARRAY(SELECT m.role FROM member m WHERE m.id IN
                              (SELECT member_id FROM calendar_member WHERE calendar_id = e.calendar_id
                               UNION SELECT member_id FROM event_member WHERE event_id = e.id)
                            ORDER BY m.id) AS "member_roles!"
               FROM event e
               LEFT JOIN calendar c ON c.id = e.calendar_id
               LEFT JOIN calendar_source s ON s.id = c.source_id
               LEFT JOIN member om ON om.id = s.owner_member
               WHERE e.start_at < $2 AND (e.end_at > $1 OR e.start_at >= $1)
                 AND NOT e.cancelled
                 AND (e.status IS NULL OR e.status IN ('approved', 'written'))
                 AND (e.calendar_id IS NULL OR (c.enabled AND c.archived_at IS NULL AND s.archived_at IS NULL))
                 AND ($3::uuid[] IS NULL OR e.calendar_id = ANY($3))
               ORDER BY e.start_at, e.end_at, e.id"#,
            from,
            to,
            cals.as_deref()
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|r| {
                let members = r
                    .member_ids
                    .into_iter()
                    .zip(r.member_roles.iter())
                    .map(|(id, role)| Ok((id.into(), parse_role(role)?)))
                    .collect::<Result<Vec<_>>>()?;
                Ok(EventRow {
                    event: Event {
                        id: r.id.into(),
                        household_id: r.household_id.into(),
                        calendar: r.calendar_id.map(Into::into),
                        remote_uid: r.remote_uid,
                        instance_key: r.instance_key,
                        title: r.title,
                        description: r.description,
                        location: r.location,
                        start_at: r.start_at,
                        end_at: r.end_at,
                        all_day: r.all_day,
                        start_date: r.start_date,
                        end_date: r.end_date,
                        source_tz: r.source_tz,
                        is_private: r.is_private,
                        cancelled: r.cancelled,
                        origin: parse_enum(&r.origin)?,
                        status: r.status.as_deref().map(parse_enum).transpose()?,
                        confidence: r.confidence,
                        source_ref: r.source_ref,
                        dedupe_key: r.dedupe_key,
                        remote_etag: r.remote_etag,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                    },
                    color: r.color,
                    members,
                    owner_role: r.owner_role.as_deref().map(parse_role).transpose()?,
                })
            })
            .collect()
    }
}
