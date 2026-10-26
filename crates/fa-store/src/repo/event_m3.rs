//! Extracted events: insertion, routing state, approval state, provenance (M3).

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{CalendarId, Event, EventId, EventStatus, HouseholdId, MemberId, MessageId, OrgId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{EventRepo, enum_str, parse_enum};
use crate::{Result, StoreError};

/// An event extracted from a message (`origin = extracted`, `status = extracted`, no calendar yet).
#[derive(Debug, Clone)]
pub struct NewExtractedEvent {
    pub household_id: HouseholdId,
    pub message_id: Option<MessageId>,
    pub org_id: Option<OrgId>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub source_tz: Option<String>,
    /// `METHOD:CANCEL` in the source: a cancellation notice, not an event to create.
    pub cancelled: bool,
    pub confidence: f32,
    pub source_ref: Option<serde_json::Value>,
    pub dedupe_key: String,
    /// The ICS UID of the source, when it had one.
    pub remote_uid: Option<String>,
    pub members: Vec<MemberId>,
}

/// An event with its M3 columns.
#[derive(Debug, Clone)]
pub struct EventFull {
    pub event: Event,
    pub version: i32,
    pub message_id: Option<MessageId>,
    pub org_id: Option<OrgId>,
    pub target_calendar_id: Option<CalendarId>,
    pub route_confidence: Option<f32>,
    pub pending_cancel: bool,
    pub write_error: Option<String>,
    pub decided_by: Option<MemberId>,
    pub decided_at: Option<DateTime<Utc>>,
    /// The event's own members (`event_member`).
    pub members: Vec<MemberId>,
}

/// A proposal as the inbox shows it.
#[derive(Debug, Clone)]
pub struct EventProposalRow {
    pub event: EventFull,
    pub message_subject: Option<String>,
    /// Sender name, else address.
    pub message_from: Option<String>,
    pub org_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EvRow {
    id: Uuid,
    household_id: Uuid,
    calendar_id: Option<Uuid>,
    remote_uid: Option<String>,
    instance_key: String,
    title: String,
    description: Option<String>,
    location: Option<String>,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    all_day: bool,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    source_tz: Option<String>,
    is_private: bool,
    cancelled: bool,
    origin: String,
    status: Option<String>,
    confidence: Option<f32>,
    source_ref: Option<serde_json::Value>,
    dedupe_key: String,
    remote_etag: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i32,
    message_id: Option<Uuid>,
    org_id: Option<Uuid>,
    target_calendar_id: Option<Uuid>,
    route_confidence: Option<f32>,
    pending_cancel: bool,
    write_error: Option<String>,
    decided_by: Option<Uuid>,
    decided_at: Option<DateTime<Utc>>,
    member_ids: Vec<Uuid>,
}

#[derive(sqlx::FromRow)]
struct ProposalRow {
    #[sqlx(flatten)]
    ev: EvRow,
    message_subject: Option<String>,
    message_from: Option<String>,
    org_name: Option<String>,
}

/// Columns of [`EvRow`] for alias `e`.
const EV: &str = "e.id, e.household_id, e.calendar_id, e.remote_uid, e.instance_key, e.title,
    e.description, e.location, e.start_at, e.end_at, e.all_day, e.start_date, e.end_date, e.source_tz,
    e.is_private, e.cancelled, e.origin, e.status, e.confidence, e.source_ref, e.dedupe_key,
    e.remote_etag, e.created_at, e.updated_at, e.version, e.message_id, e.org_id,
    e.target_calendar_id, e.route_confidence, e.pending_cancel, e.write_error, e.decided_by,
    e.decided_at,
    ARRAY(SELECT member_id FROM event_member WHERE event_id = e.id ORDER BY member_id) AS member_ids";

impl TryFrom<EvRow> for EventFull {
    type Error = StoreError;
    fn try_from(r: EvRow) -> Result<Self> {
        Ok(EventFull {
            version: r.version,
            message_id: r.message_id.map(Into::into),
            org_id: r.org_id.map(Into::into),
            target_calendar_id: r.target_calendar_id.map(Into::into),
            route_confidence: r.route_confidence,
            pending_cancel: r.pending_cancel,
            write_error: r.write_error,
            decided_by: r.decided_by.map(Into::into),
            decided_at: r.decided_at,
            members: r.member_ids.into_iter().map(Into::into).collect(),
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
        })
    }
}

fn one(n: u64) -> Result<()> {
    if n == 0 {
        Err(StoreError::NotFound)
    } else {
        Ok(())
    }
}

impl EventRepo {
    /// Inserts an extracted event (status `extracted`) with its members. Returns the new id.
    pub async fn insert_extracted(
        conn: &mut PgConnection,
        e: &NewExtractedEvent,
    ) -> Result<EventId> {
        let id = EventId::new();
        sqlx::query(
            "INSERT INTO event (id, household_id, calendar_id, remote_uid, instance_key, title,
                    description, location, start_at, end_at, all_day, start_date, end_date, source_tz,
                    is_private, cancelled, origin, status, confidence, source_ref, dedupe_key,
                    message_id, org_id)
             VALUES ($1, $2, NULL, $3, '', $4, $5, $6, $7, $8, $9, $10, $11, $12, false, $13,
                     'extracted', 'extracted', $14, $15, $16, $17, $18)",
        )
        .bind(id.as_uuid())
        .bind(e.household_id.as_uuid())
        .bind(&e.remote_uid)
        .bind(&e.title)
        .bind(&e.description)
        .bind(&e.location)
        .bind(e.start_at)
        .bind(e.end_at)
        .bind(e.all_day)
        .bind(e.start_date)
        .bind(e.end_date)
        .bind(&e.source_tz)
        .bind(e.cancelled)
        .bind(e.confidence)
        .bind(&e.source_ref)
        .bind(&e.dedupe_key)
        .bind(e.message_id.map(|m| m.as_uuid()))
        .bind(e.org_id.map(|o| o.as_uuid()))
        .execute(&mut *conn)
        .await?;
        let members: Vec<Uuid> = e.members.iter().map(|m| m.as_uuid()).collect();
        sqlx::query(
            "INSERT INTO event_member (event_id, member_id, household_id)
             SELECT $1, m, $2 FROM unnest($3::uuid[]) AS m ON CONFLICT DO NOTHING",
        )
        .bind(id.as_uuid())
        .bind(e.household_id.as_uuid())
        .bind(&members)
        .execute(conn)
        .await?;
        Ok(id)
    }

    pub async fn get_full(conn: &mut PgConnection, id: EventId) -> Result<Option<EventFull>> {
        let row: Option<EvRow> =
            sqlx::query_as(&format!("SELECT {EV} FROM event e WHERE e.id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Events of any origin starting in `[start_from, start_to)` that could be the same event as a
    /// new one: everything except `rejected` and `expired` rows.
    pub async fn dedupe_candidates(
        conn: &mut PgConnection,
        household: HouseholdId,
        start_from: DateTime<Utc>,
        start_to: DateTime<Utc>,
    ) -> Result<Vec<EventFull>> {
        let rows: Vec<EvRow> = sqlx::query_as(&format!(
            "SELECT {EV} FROM event e
             WHERE e.household_id = $1 AND e.start_at >= $2 AND e.start_at < $3
               AND (e.status IS NULL OR e.status NOT IN ('rejected', 'expired'))
             ORDER BY e.start_at, e.id"
        ))
        .bind(household.as_uuid())
        .bind(start_from)
        .bind(start_to)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Stores the routing outcome.
    pub async fn set_route(
        conn: &mut PgConnection,
        id: EventId,
        target: Option<CalendarId>,
        route_confidence: Option<f32>,
    ) -> Result<()> {
        one(sqlx::query(
            "UPDATE event SET target_calendar_id = $2, route_confidence = $3, version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(target.map(|c| c.as_uuid()))
        .bind(route_confidence)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Sets the status. `by` records who decided; `approved` and `rejected` stamp `decided_at`.
    pub async fn set_status(
        conn: &mut PgConnection,
        id: EventId,
        status: EventStatus,
        by: Option<MemberId>,
    ) -> Result<()> {
        one(sqlx::query(
            "UPDATE event SET status = $2, decided_by = COALESCE($3::uuid, decided_by),
                    decided_at = CASE WHEN $2 IN ('approved', 'rejected') THEN now() ELSE decided_at END,
                    version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(enum_str(&status))
        .bind(by.map(|m| m.as_uuid()))
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// The event was written to its provider: status `written`, calendar and identity set, error
    /// cleared. A synced row that the sync imported for the same remote event first is removed.
    pub async fn set_written(
        conn: &mut PgConnection,
        id: EventId,
        calendar: CalendarId,
        remote_uid: &str,
        etag: Option<&str>,
        external_url: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM event WHERE calendar_id = $1 AND remote_uid = $2 AND instance_key = ''
               AND origin = 'synced' AND id <> $3",
        )
        .bind(calendar.as_uuid())
        .bind(remote_uid)
        .bind(id.as_uuid())
        .execute(&mut *conn)
        .await?;
        one(sqlx::query(
            "UPDATE event SET status = 'written', calendar_id = $2, remote_uid = $3, remote_etag = $4,
                    write_error = NULL,
                    source_ref = CASE WHEN $5::text IS NULL THEN source_ref
                                      ELSE COALESCE(source_ref, '{}'::jsonb)
                                           || jsonb_build_object('external_url', $5::text) END,
                    version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(calendar.as_uuid())
        .bind(remote_uid)
        .bind(etag)
        .bind(external_url)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Records (or with `None` clears) the last write error.
    pub async fn set_write_error(
        conn: &mut PgConnection,
        id: EventId,
        error: Option<&str>,
    ) -> Result<()> {
        one(
            sqlx::query("UPDATE event SET write_error = $2, version = version + 1 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(error)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    pub async fn set_pending_cancel(conn: &mut PgConnection, id: EventId, on: bool) -> Result<()> {
        one(sqlx::query(
            "UPDATE event SET pending_cancel = $2, version = version + 1 WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(on)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Events waiting for a decision: `proposed`, written with `pending_cancel`, or approved with a
    /// failed write; soonest first.
    pub async fn proposals(conn: &mut PgConnection) -> Result<Vec<EventProposalRow>> {
        let rows: Vec<ProposalRow> = sqlx::query_as(&format!(
            "SELECT {EV}, m.subject AS message_subject, COALESCE(m.from_name, m.from_addr) AS message_from,
                    o.name AS org_name
             FROM event e
             LEFT JOIN message m ON m.id = e.message_id
             LEFT JOIN org o ON o.id = e.org_id
             WHERE e.origin = 'extracted'
               AND (e.status = 'proposed' OR (e.status = 'written' AND e.pending_cancel)
                    OR (e.status = 'approved' AND e.write_error IS NOT NULL))
             ORDER BY e.start_at, e.id"
        ))
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(EventProposalRow {
                    event: r.ev.try_into()?,
                    message_subject: r.message_subject,
                    message_from: r.message_from,
                    org_name: r.org_name,
                })
            })
            .collect()
    }

    /// Records that `message` also mentioned `event` (a deduped duplicate).
    pub async fn link_message(
        conn: &mut PgConnection,
        event: EventId,
        message: MessageId,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO event_message (event_id, message_id, household_id)
             SELECT id, $2, household_id FROM event WHERE id = $1
             ON CONFLICT DO NOTHING",
        )
        .bind(event.as_uuid())
        .bind(message.as_uuid())
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Deletes the event (members and provenance cascade). Returns whether a row existed.
    pub async fn delete(conn: &mut PgConnection, id: EventId) -> Result<bool> {
        Ok(sqlx::query("DELETE FROM event WHERE id = $1")
            .bind(id.as_uuid())
            .execute(conn)
            .await?
            .rows_affected()
            > 0)
    }

    /// Proposed events that start before `now` become `expired`.
    pub async fn expire_past(conn: &mut PgConnection, now: DateTime<Utc>) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE event SET status = 'expired', version = version + 1
             WHERE origin = 'extracted' AND status = 'proposed' AND start_at < $1",
        )
        .bind(now)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Events extracted from the message, or that it also mentioned.
    pub async fn derived_for_message(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Vec<EventFull>> {
        let rows: Vec<EvRow> = sqlx::query_as(&format!(
            "SELECT {EV} FROM event e
             WHERE e.message_id = $1
                OR EXISTS (SELECT 1 FROM event_message em WHERE em.event_id = e.id AND em.message_id = $1)
             ORDER BY e.start_at, e.id"
        ))
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
