//! Bills, mail actions (with undo) and the approval inbox counts.

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{
    Bill, BillId, BillStatus, InboxCounts, MailAction, MailActionId, MailActionKind, MemberId,
    MessageId, TaskId,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

fn one(n: u64) -> Result<()> {
    if n == 0 {
        Err(StoreError::NotFound)
    } else {
        Ok(())
    }
}

// ---- bills ----

pub struct BillRepo;

#[derive(sqlx::FromRow)]
struct BillRow {
    id: Uuid,
    household_id: Uuid,
    message_id: Option<Uuid>,
    org_id: Option<Uuid>,
    payee: String,
    amount_minor: i64,
    currency: String,
    due_on: Option<NaiveDate>,
    account_hint: Option<String>,
    statement_period: Option<String>,
    status: String,
    confidence: f32,
    task_id: Option<Uuid>,
    source_ref: serde_json::Value,
    dedupe_key: String,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const BILL_COLS: &str =
    "id, household_id, message_id, org_id, payee, amount_minor, currency, due_on,
    account_hint, statement_period, status, confidence, task_id, source_ref, dedupe_key, version,
    created_at, updated_at";

impl TryFrom<BillRow> for Versioned<Bill> {
    type Error = StoreError;
    fn try_from(r: BillRow) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Bill {
                id: r.id.into(),
                household_id: r.household_id.into(),
                message_id: r.message_id.map(Into::into),
                org_id: r.org_id.map(Into::into),
                payee: r.payee,
                amount_minor: r.amount_minor,
                currency: r.currency,
                due_on: r.due_on,
                account_hint: r.account_hint,
                statement_period: r.statement_period,
                status: parse_enum(&r.status)?,
                confidence: r.confidence,
                task_id: r.task_id.map(Into::into),
                source_ref: r.source_ref,
                dedupe_key: r.dedupe_key,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl BillRepo {
    /// Inserts the bill unless `(household, dedupe_key)` exists (then `None`).
    pub async fn insert_if_new(
        conn: &mut PgConnection,
        b: &Bill,
    ) -> Result<Option<Versioned<Bill>>> {
        let row: Option<BillRow> = sqlx::query_as(&format!(
            "INSERT INTO bill (id, household_id, message_id, org_id, payee, amount_minor, currency,
                               due_on, account_hint, statement_period, status, confidence, task_id,
                               source_ref, dedupe_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (household_id, dedupe_key) DO NOTHING RETURNING {BILL_COLS}"
        ))
        .bind(b.id.as_uuid())
        .bind(b.household_id.as_uuid())
        .bind(b.message_id.map(|m| m.as_uuid()))
        .bind(b.org_id.map(|m| m.as_uuid()))
        .bind(&b.payee)
        .bind(b.amount_minor)
        .bind(&b.currency)
        .bind(b.due_on)
        .bind(&b.account_hint)
        .bind(&b.statement_period)
        .bind(enum_str(&b.status))
        .bind(b.confidence)
        .bind(b.task_id.map(|m| m.as_uuid()))
        .bind(&b.source_ref)
        .bind(&b.dedupe_key)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn get(conn: &mut PgConnection, id: BillId) -> Result<Option<Versioned<Bill>>> {
        let row: Option<BillRow> =
            sqlx::query_as(&format!("SELECT {BILL_COLS} FROM bill WHERE id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Open bills first by due date, then the rest newest first.
    pub async fn list(
        conn: &mut PgConnection,
        status: Option<BillStatus>,
    ) -> Result<Vec<Versioned<Bill>>> {
        let rows: Vec<BillRow> = sqlx::query_as(&format!(
            "SELECT {BILL_COLS} FROM bill WHERE $1::text IS NULL OR status = $1
             ORDER BY due_on NULLS LAST, created_at DESC, id"
        ))
        .bind(status.as_ref().map(enum_str))
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Updates payee, amount, currency, due date, account hint, statement period and status.
    pub async fn update(conn: &mut PgConnection, b: &Bill, expected_version: i32) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE bill SET payee = $2, amount_minor = $3, currency = $4, due_on = $5,
                    account_hint = $6, statement_period = $7, status = $8, version = version + 1
             WHERE id = $1 AND version = $9 RETURNING version",
        )
        .bind(b.id.as_uuid())
        .bind(&b.payee)
        .bind(b.amount_minor)
        .bind(&b.currency)
        .bind(b.due_on)
        .bind(&b.account_hint)
        .bind(&b.statement_period)
        .bind(enum_str(&b.status))
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "bill", b.id.as_uuid()).await),
        }
    }

    pub async fn set_status(conn: &mut PgConnection, id: BillId, status: BillStatus) -> Result<()> {
        one(
            sqlx::query("UPDATE bill SET status = $2, version = version + 1 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(enum_str(&status))
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    pub async fn set_task(conn: &mut PgConnection, bill: BillId, task: TaskId) -> Result<()> {
        one(
            sqlx::query("UPDATE bill SET task_id = $2, version = version + 1 WHERE id = $1")
                .bind(bill.as_uuid())
                .bind(task.as_uuid())
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    /// The bill a pay task belongs to.
    pub async fn by_task(conn: &mut PgConnection, task: TaskId) -> Result<Option<Versioned<Bill>>> {
        let row: Option<BillRow> =
            sqlx::query_as(&format!("SELECT {BILL_COLS} FROM bill WHERE task_id = $1"))
                .bind(task.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// The pay task was completed: `open` becomes `paid`. Returns whether a bill changed.
    pub async fn set_paid_by_task(conn: &mut PgConnection, task: TaskId) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE bill SET status = 'paid', version = version + 1
             WHERE task_id = $1 AND status = 'open'",
        )
        .bind(task.as_uuid())
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    /// The pay task was reopened: `paid` becomes `open`. Returns whether a bill changed.
    pub async fn reopen_by_task(conn: &mut PgConnection, task: TaskId) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE bill SET status = 'open', version = version + 1
             WHERE task_id = $1 AND status = 'paid'",
        )
        .bind(task.as_uuid())
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn for_message(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Vec<Versioned<Bill>>> {
        let rows: Vec<BillRow> = sqlx::query_as(&format!(
            "SELECT {BILL_COLS} FROM bill WHERE message_id = $1 ORDER BY created_at, id"
        ))
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

// ---- mail actions ----

pub struct MailActionRepo;

/// A mail action to record.
#[derive(Debug, Clone)]
pub struct NewMailAction {
    pub household_id: fa_domain::HouseholdId,
    pub message_id: MessageId,
    pub action: MailActionKind,
    pub label: Option<String>,
    pub automatic: bool,
    pub done_by: Option<MemberId>,
    pub error: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ActionRow {
    id: Uuid,
    household_id: Uuid,
    message_id: Uuid,
    action: String,
    label: Option<String>,
    automatic: bool,
    done_by: Option<Uuid>,
    done_at: DateTime<Utc>,
    undone_at: Option<DateTime<Utc>>,
    undone_by: Option<Uuid>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const ACTION_COLS: &str =
    "id, household_id, message_id, action, label, automatic, done_by, done_at,
    undone_at, undone_by, error, created_at, updated_at";

impl TryFrom<ActionRow> for MailAction {
    type Error = StoreError;
    fn try_from(r: ActionRow) -> Result<Self> {
        Ok(MailAction {
            id: r.id.into(),
            household_id: r.household_id.into(),
            message_id: r.message_id.into(),
            action: parse_enum(&r.action)?,
            label: r.label,
            automatic: r.automatic,
            done_by: r.done_by.map(Into::into),
            done_at: r.done_at,
            undone_at: r.undone_at,
            undone_by: r.undone_by.map(Into::into),
            error: r.error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

impl MailActionRepo {
    pub async fn insert(conn: &mut PgConnection, a: &NewMailAction) -> Result<MailAction> {
        let row: ActionRow = sqlx::query_as(&format!(
            "INSERT INTO mail_action (id, household_id, message_id, action, label, automatic, done_by,
                                      error)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {ACTION_COLS}"
        ))
        .bind(MailActionId::new().as_uuid())
        .bind(a.household_id.as_uuid())
        .bind(a.message_id.as_uuid())
        .bind(enum_str(&a.action))
        .bind(&a.label)
        .bind(a.automatic)
        .bind(a.done_by.map(|m| m.as_uuid()))
        .bind(&a.error)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(conn: &mut PgConnection, id: MailActionId) -> Result<Option<MailAction>> {
        let row: Option<ActionRow> = sqlx::query_as(&format!(
            "SELECT {ACTION_COLS} FROM mail_action WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list_for_message(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Vec<MailAction>> {
        let rows: Vec<ActionRow> = sqlx::query_as(&format!(
            "SELECT {ACTION_COLS} FROM mail_action WHERE message_id = $1 ORDER BY done_at, id"
        ))
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Marks the action undone. Returns `false` when it was already undone.
    pub async fn mark_undone(
        conn: &mut PgConnection,
        id: MailActionId,
        by: Option<MemberId>,
    ) -> Result<bool> {
        let n = sqlx::query(
            "UPDATE mail_action SET undone_at = now(), undone_by = $2
             WHERE id = $1 AND undone_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(by.map(|m| m.as_uuid()))
        .execute(&mut *conn)
        .await?
        .rows_affected();
        if n == 0 {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM mail_action WHERE id = $1)")
                    .bind(id.as_uuid())
                    .fetch_one(conn)
                    .await?;
            if !exists {
                return Err(StoreError::NotFound);
            }
        }
        Ok(n > 0)
    }

    pub async fn set_error(
        conn: &mut PgConnection,
        id: MailActionId,
        error: Option<&str>,
    ) -> Result<()> {
        one(
            sqlx::query("UPDATE mail_action SET error = $2 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(error)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}

// ---- inbox ----

pub struct InboxRepo;

impl InboxRepo {
    /// Items waiting for approval: events `proposed`, written with `pending_cancel` or approved with a
    /// failed write; bills
    /// `proposed`, email tasks `suggested`, contact suggestions `proposed`.
    pub async fn counts(conn: &mut PgConnection) -> Result<InboxCounts> {
        let (event, bill, task, contact): (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
               (SELECT count(*) FROM event
                 WHERE origin = 'extracted' AND (status = 'proposed' OR (status = 'written' AND pending_cancel)
                   OR (status = 'approved' AND write_error IS NOT NULL))),
               (SELECT count(*) FROM bill WHERE status = 'proposed'),
               (SELECT count(*) FROM task
                 WHERE status = 'suggested' AND origin = 'email' AND archived_at IS NULL),
               (SELECT count(*) FROM contact_suggestion WHERE status = 'proposed')",
        )
        .fetch_one(conn)
        .await?;
        Ok(InboxCounts::new(event, bill, task, contact))
    }

    /// Pending items per active adult of the transaction's household, scoped by D-078: an item
    /// reaches its source through `message_id` (events, bills, contact suggestions) or
    /// `task.source_ref->>'message_id'` (email tasks), then `message.source_id`. The adults
    /// who may see it are the source's owners, or all adults for a `household` source. An item
    /// without a source (message deleted) or with an archived one counts for every adult.
    pub async fn pending_by_adult(conn: &mut PgConnection) -> Result<Vec<(MemberId, i64)>> {
        let rows: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
            "WITH item AS (
               SELECT message_id FROM event
                WHERE origin = 'extracted' AND (status = 'proposed' OR (status = 'written' AND pending_cancel)
                   OR (status = 'approved' AND write_error IS NOT NULL))
               UNION ALL SELECT message_id FROM bill WHERE status = 'proposed'
               UNION ALL SELECT message_id FROM contact_suggestion WHERE status = 'proposed'
               UNION ALL SELECT CASE WHEN source_ref->>'message_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                                     THEN (source_ref->>'message_id')::uuid END
                           FROM task
                          WHERE status = 'suggested' AND origin = 'email' AND archived_at IS NULL
             ), scoped AS (
               SELECT s.id AS source_id, s.visibility, s.archived_at
                 FROM item i
                 LEFT JOIN message m ON m.id = i.message_id
                 LEFT JOIN mail_source s ON s.id = m.source_id
             )
             SELECT a.id, count(sc.*) FILTER (WHERE
                        sc.source_id IS NULL OR sc.archived_at IS NOT NULL OR sc.visibility = 'household'
                        OR EXISTS (SELECT 1 FROM mail_source_owner o
                                    WHERE o.source_id = sc.source_id AND o.member_id = a.id))
               FROM member a
               LEFT JOIN scoped sc ON true
              WHERE a.role = 'adult' AND a.archived_at IS NULL AND a.household_id = app_household()
              GROUP BY a.id, a.created_at ORDER BY a.created_at, a.id",
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(|(id, n)| (id.into(), n)).collect())
    }
}
