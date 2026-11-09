//! Operator reset of mail processing (`fa-server mail reset`): forget every synced message and what the
//! pipeline derived from it, and rewind the sync cursors, so the next sync backfills as if the sources
//! were just added. Configuration stays: sources, orgs and contacts, rules, categories, pipelines and
//! the AI settings. Events already approved or written to a calendar stay too (detached from their
//! message): re-extraction dedupes against them instead of writing duplicates.

use fa_domain::HouseholdId;
use sqlx::PgConnection;

use crate::Result;

/// Job kinds that act on a message or on something derived from one.
pub const MAIL_DERIVED_JOB_KINDS: &[&str] = &[
    "mail.process",
    "mail.llm",
    "mail.llm.catchup",
    "mail.auto_actions",
    "tasks.from_mail",
    "tasks.from_bill",
    "calendar.route",
];

/// What a reset removed (or, in a dry run, would remove).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MailResetCounts {
    pub messages: u64,
    pub decisions: u64,
    pub events_removed: u64,
    pub events_kept: u64,
    pub tasks: u64,
    pub bills: u64,
    pub contact_suggestions: u64,
    pub notifications: u64,
    pub jobs: u64,
    pub folder_cursors: u64,
    pub sources_rewound: u64,
}

pub struct MailResetRepo;

impl MailResetRepo {
    /// Mail jobs a worker holds right now (a reset must not race them).
    pub async fn running_jobs(conn: &mut PgConnection, household: HouseholdId) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM job
              WHERE household_id = $1 AND status = 'running' AND locked_until > now()
                AND (kind = ANY($2) OR kind LIKE 'mail.%')",
        )
        .bind(household.as_uuid())
        .bind(MAIL_DERIVED_JOB_KINDS)
        .fetch_one(conn)
        .await?;
        Ok(n)
    }

    /// Runs the reset in the caller's household transaction; the caller commits or rolls back.
    pub async fn reset(conn: &mut PgConnection, household: HouseholdId) -> Result<MailResetCounts> {
        let mut c = MailResetCounts::default();
        // Hold the sources so a sync in another process waits for this transaction.
        sqlx::query("SELECT id FROM mail_source FOR UPDATE")
            .execute(&mut *conn)
            .await?;

        const DROP_EVENTS: &str =
            "origin = 'extracted' AND COALESCE(status, '') NOT IN ('approved', 'written')";
        c.events_kept = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT count(*) FROM event WHERE origin = 'extracted' AND NOT ({DROP_EVENTS})"
        ))
        .fetch_one(&mut *conn)
        .await? as u64;
        c.decisions += sqlx::query(&format!(
            "DELETE FROM decision WHERE subject_type = 'event'
                AND subject_id IN (SELECT id FROM event WHERE {DROP_EVENTS})"
        ))
        .execute(&mut *conn)
        .await?
        .rows_affected();
        c.events_removed = sqlx::query(&format!("DELETE FROM event WHERE {DROP_EVENTS}"))
            .execute(&mut *conn)
            .await?
            .rows_affected();

        c.tasks = sqlx::query("DELETE FROM task WHERE origin = 'email'")
            .execute(&mut *conn)
            .await?
            .rows_affected();
        c.bills = sqlx::query("DELETE FROM bill")
            .execute(&mut *conn)
            .await?
            .rows_affected();
        c.contact_suggestions = sqlx::query("DELETE FROM contact_suggestion")
            .execute(&mut *conn)
            .await?
            .rows_affected();
        c.notifications = sqlx::query(
            "DELETE FROM notification
              WHERE status = 'pending' AND trigger IN ('approval_needed', 'mail_time_sensitive')",
        )
        .execute(&mut *conn)
        .await?
        .rows_affected();
        c.decisions += sqlx::query("DELETE FROM decision WHERE subject_type = 'message'")
            .execute(&mut *conn)
            .await?
            .rows_affected();
        // Cascades to message_member, mail_action and event_message.
        c.messages = sqlx::query("DELETE FROM message")
            .execute(&mut *conn)
            .await?
            .rows_affected();

        c.folder_cursors = sqlx::query("DELETE FROM mail_folder_state")
            .execute(&mut *conn)
            .await?
            .rows_affected();
        c.sources_rewound = sqlx::query(
            "UPDATE mail_source
                SET history_id = NULL, last_synced_at = NULL, last_error = NULL,
                    state = CASE WHEN state = 'error' THEN 'ok' ELSE state END
              WHERE archived_at IS NULL",
        )
        .execute(&mut *conn)
        .await?
        .rows_affected();
        c.jobs = sqlx::query(
            "DELETE FROM job WHERE household_id = $1 AND status IN ('queued', 'dead') AND kind = ANY($2)",
        )
        .bind(household.as_uuid())
        .bind(MAIL_DERIVED_JOB_KINDS)
        .execute(&mut *conn)
        .await?
        .rows_affected();
        Ok(c)
    }
}
