use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MemberId, Notification, NotificationId, NotifyTrigger};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{enum_str, parse_enum};
use crate::{Result, StoreError};

pub struct NotificationRepo;

/// An outbox row to insert. `not_before` defaults to now.
#[derive(Debug, Clone)]
pub struct NewNotification {
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub trigger: NotifyTrigger,
    pub subject_type: String,
    pub subject_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    pub click_path: Option<String>,
    pub priority: i32,
    pub dedupe_key: String,
    pub not_before: Option<DateTime<Utc>>,
}

struct Row {
    id: Uuid,
    household_id: Uuid,
    member_id: Uuid,
    trigger: String,
    subject_type: String,
    subject_id: Option<Uuid>,
    title: String,
    body: String,
    click_path: Option<String>,
    priority: i32,
    dedupe_key: String,
    status: String,
    not_before: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    attempts: i32,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<Row> for Notification {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Notification {
            id: r.id.into(),
            household_id: r.household_id.into(),
            member_id: r.member_id.into(),
            trigger: parse_enum(&r.trigger)?,
            subject_type: r.subject_type,
            subject_id: r.subject_id,
            title: r.title,
            body: r.body,
            click_path: r.click_path,
            priority: r.priority,
            dedupe_key: r.dedupe_key,
            status: parse_enum(&r.status)?,
            not_before: r.not_before,
            sent_at: r.sent_at,
            attempts: r.attempts,
            last_error: r.last_error,
            created_at: r.created_at,
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

impl NotificationRepo {
    /// Inserts the row unless `(household, dedupe_key)` exists. Returns whether it inserted.
    pub async fn insert_dedupe(conn: &mut PgConnection, n: &NewNotification) -> Result<bool> {
        let rows = sqlx::query!(
            "INSERT INTO notification (id, household_id, member_id, trigger, subject_type, subject_id,
                                       title, body, click_path, priority, dedupe_key, not_before)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12, now()))
             ON CONFLICT (household_id, dedupe_key) DO NOTHING",
            NotificationId::new().as_uuid(),
            n.household_id.as_uuid(),
            n.member_id.as_uuid(),
            enum_str(&n.trigger),
            n.subject_type,
            n.subject_id,
            n.title,
            n.body,
            n.click_path,
            n.priority,
            n.dedupe_key,
            n.not_before
        )
        .execute(conn)
        .await?
        .rows_affected();
        Ok(rows > 0)
    }

    /// One row per non-archived adult of the transaction's household (`app.household_id`, so use
    /// `Store::tx_for`), dedupe key `{key_prefix}:{member}`. Returns how many rows were inserted.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_for_adults(
        conn: &mut PgConnection,
        trigger: NotifyTrigger,
        subject_type: &str,
        subject_id: Option<Uuid>,
        title: &str,
        body: &str,
        click_path: Option<&str>,
        key_prefix: &str,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "INSERT INTO notification (id, household_id, member_id, trigger, subject_type, subject_id,
                                       title, body, click_path, priority, dedupe_key)
             SELECT gen_random_uuid(), m.household_id, m.id, $1, $2, $3, $4, $5, $6, 3,
                    $7 || ':' || m.id::text
             FROM member m
             WHERE m.role = 'adult' AND m.archived_at IS NULL AND m.household_id = app_household()
             ON CONFLICT (household_id, dedupe_key) DO NOTHING",
        )
        .bind(enum_str(&trigger))
        .bind(subject_type)
        .bind(subject_id)
        .bind(title)
        .bind(body)
        .bind(click_path)
        .bind(key_prefix)
        .execute(conn)
        .await?
        .rows_affected())
    }

    pub async fn get(conn: &mut PgConnection, id: NotificationId) -> Result<Option<Notification>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, trigger, subject_type, subject_id, title, body,
                    click_path, priority, dedupe_key, status, not_before, sent_at, attempts,
                    last_error, created_at
             FROM notification WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Pending rows that are due, oldest first. Use a system tx: it spans households.
    pub async fn pending(
        conn: &mut PgConnection,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<Notification>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, trigger, subject_type, subject_id, title, body,
                    click_path, priority, dedupe_key, status, not_before, sent_at, attempts,
                    last_error, created_at
             FROM notification WHERE status = 'pending' AND not_before <= $1
             ORDER BY created_at, id LIMIT $2",
            now,
            limit
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn mark_sent(
        conn: &mut PgConnection,
        ids: &[NotificationId],
        at: DateTime<Utc>,
    ) -> Result<()> {
        let ids: Vec<Uuid> = ids.iter().map(|i| i.as_uuid()).collect();
        sqlx::query!(
            "UPDATE notification SET status = 'sent', sent_at = $2, last_error = NULL WHERE id = ANY($1)",
            &ids,
            at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn mark_dropped(
        conn: &mut PgConnection,
        id: NotificationId,
        reason: &str,
    ) -> Result<()> {
        one(sqlx::query!(
            "UPDATE notification SET status = 'dropped', last_error = $2 WHERE id = $1",
            id.as_uuid(),
            reason
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Keeps the row pending but not before the given time (quiet hours).
    pub async fn defer(
        conn: &mut PgConnection,
        id: NotificationId,
        not_before: DateTime<Utc>,
    ) -> Result<()> {
        one(sqlx::query!(
            "UPDATE notification SET not_before = $2 WHERE id = $1",
            id.as_uuid(),
            not_before
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Counts a failed publish: `attempts + 1` and the error; then `failed` if `give_up`, else
    /// pending again at `next_try`.
    pub async fn record_failure(
        conn: &mut PgConnection,
        id: NotificationId,
        error: &str,
        next_try: DateTime<Utc>,
        give_up: bool,
    ) -> Result<()> {
        one(sqlx::query!(
            "UPDATE notification SET attempts = attempts + 1, last_error = $2, not_before = $3,
                    status = CASE WHEN $4 THEN 'failed' ELSE 'pending' END
             WHERE id = $1",
            id.as_uuid(),
            error,
            next_try,
            give_up
        )
        .execute(conn)
        .await?
        .rows_affected())
    }
}
