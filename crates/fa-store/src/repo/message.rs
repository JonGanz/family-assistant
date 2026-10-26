//! Messages: parsed mail metadata and content, classification results, member links.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use fa_domain::{
    ContentPolicy, DecidedBy, HouseholdId, MailSourceId, MemberId, Message, MessageId,
    ProcessingState,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{enum_str, parse_enum};
use crate::{Result, StoreError};

pub struct MessageRepo;

/// A freshly synced message (`processing_state = pending`).
#[derive(Debug, Clone)]
pub struct NewMessage {
    pub household_id: HouseholdId,
    pub source_id: MailSourceId,
    pub remote_id: String,
    pub folder: Option<String>,
    pub thread_id: Option<String>,
    pub rfc822_id: Option<String>,
    pub received_at: DateTime<Utc>,
    /// Lower-cased by the caller.
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to_addrs: Vec<String>,
    pub cc_addrs: Vec<String>,
    pub subject: String,
    pub gmail_labels: Vec<String>,
    pub has_attachments: bool,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub snippet: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub attachments: Option<serde_json::Value>,
}

/// Filter of [`MessageRepo::list`]. Results are newest first.
#[derive(Debug, Clone, Default)]
pub struct MessageFilter<'a> {
    /// Only these sources (the caller passes the sources the viewer may read).
    pub sources: &'a [MailSourceId],
    pub category: Option<&'a str>,
    /// Linked to this member.
    pub member: Option<MemberId>,
    pub state: Option<ProcessingState>,
    /// Case-insensitive substring of the subject, sender address or sender name.
    pub q: Option<&'a str>,
    /// Category keys to hide (those with behaviour `ignore`); `None` hides nothing.
    /// Messages without a category are never hidden.
    pub include_ignored_keys: Option<&'a [String]>,
    /// `received_at < before` (cursor).
    pub before: Option<DateTime<Utc>>,
    pub limit: i64,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    household_id: Uuid,
    source_id: Uuid,
    remote_id: String,
    folder: Option<String>,
    thread_id: Option<String>,
    rfc822_id: Option<String>,
    received_at: DateTime<Utc>,
    from_addr: String,
    from_name: Option<String>,
    to_addrs: Vec<String>,
    cc_addrs: Vec<String>,
    subject: String,
    gmail_labels: Vec<String>,
    has_attachments: bool,
    category: Option<String>,
    time_sensitive: Option<bool>,
    is_bill: Option<bool>,
    processing_state: String,
    processed_at: Option<DateTime<Utc>>,
    body_text: Option<String>,
    body_html: Option<String>,
    snippet: Option<String>,
    headers: Option<serde_json::Value>,
    attachments: Option<serde_json::Value>,
    is_learning_example: bool,
    in_eval_sample: bool,
    content_purged_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Everything except the heavy content (body, headers).
const META: &str =
    "id, household_id, source_id, remote_id, folder, thread_id, rfc822_id, received_at,
    from_addr, from_name, to_addrs, cc_addrs, subject, gmail_labels, has_attachments, category,
    time_sensitive, is_bill, processing_state, processed_at,
    NULL::text AS body_text, NULL::text AS body_html, snippet, NULL::jsonb AS headers, attachments,
    is_learning_example, in_eval_sample, content_purged_at, created_at, updated_at";

const FULL: &str =
    "id, household_id, source_id, remote_id, folder, thread_id, rfc822_id, received_at,
    from_addr, from_name, to_addrs, cc_addrs, subject, gmail_labels, has_attachments, category,
    time_sensitive, is_bill, processing_state, processed_at, body_text, body_html, snippet, headers,
    attachments, is_learning_example, in_eval_sample, content_purged_at, created_at, updated_at";

impl TryFrom<Row> for Message {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Message {
            id: r.id.into(),
            household_id: r.household_id.into(),
            source_id: r.source_id.into(),
            remote_id: r.remote_id,
            folder: r.folder,
            thread_id: r.thread_id,
            rfc822_id: r.rfc822_id,
            received_at: r.received_at,
            from_addr: r.from_addr,
            from_name: r.from_name,
            to_addrs: r.to_addrs,
            cc_addrs: r.cc_addrs,
            subject: r.subject,
            gmail_labels: r.gmail_labels,
            has_attachments: r.has_attachments,
            category: r.category,
            time_sensitive: r.time_sensitive,
            is_bill: r.is_bill,
            processing_state: parse_enum(&r.processing_state)?,
            processed_at: r.processed_at,
            body_text: r.body_text,
            body_html: r.body_html,
            snippet: r.snippet,
            headers: r.headers,
            attachments: r.attachments,
            is_learning_example: r.is_learning_example,
            in_eval_sample: r.in_eval_sample,
            content_purged_at: r.content_purged_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
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

fn uuids(ids: &[MessageId]) -> Vec<Uuid> {
    ids.iter().map(|i| i.as_uuid()).collect()
}

/// `%q%` for ILIKE with the wildcards of `q` escaped.
fn like(q: &str) -> String {
    let mut out = String::from("%");
    for c in q.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

impl MessageRepo {
    /// Inserts a pending message. `None` when `(source, remote_id)` already exists.
    pub async fn insert_parsed(
        conn: &mut PgConnection,
        m: &NewMessage,
    ) -> Result<Option<MessageId>> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO message (id, household_id, source_id, remote_id, folder, thread_id, rfc822_id,
                    received_at, from_addr, from_name, to_addrs, cc_addrs, subject, gmail_labels,
                    has_attachments, body_text, body_html, snippet, headers, attachments)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                     $19, $20)
             ON CONFLICT (source_id, remote_id) DO NOTHING RETURNING id",
        )
        .bind(MessageId::new().as_uuid())
        .bind(m.household_id.as_uuid())
        .bind(m.source_id.as_uuid())
        .bind(&m.remote_id)
        .bind(&m.folder)
        .bind(&m.thread_id)
        .bind(&m.rfc822_id)
        .bind(m.received_at)
        .bind(&m.from_addr)
        .bind(&m.from_name)
        .bind(&m.to_addrs)
        .bind(&m.cc_addrs)
        .bind(&m.subject)
        .bind(&m.gmail_labels)
        .bind(m.has_attachments)
        .bind(&m.body_text)
        .bind(&m.body_html)
        .bind(&m.snippet)
        .bind(&m.headers)
        .bind(&m.attachments)
        .fetch_optional(conn)
        .await?;
        Ok(id.map(Into::into))
    }

    /// Metadata only: `body_text`, `body_html` and `headers` are `None`.
    pub async fn get(conn: &mut PgConnection, id: MessageId) -> Result<Option<Message>> {
        let row: Option<Row> = sqlx::query_as(&format!("SELECT {META} FROM message WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn get_with_content(
        conn: &mut PgConnection,
        id: MessageId,
    ) -> Result<Option<Message>> {
        let row: Option<Row> = sqlx::query_as(&format!("SELECT {FULL} FROM message WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// The message of a source by provider id, with content.
    pub async fn get_by_remote(
        conn: &mut PgConnection,
        source: MailSourceId,
        remote_id: &str,
    ) -> Result<Option<Message>> {
        let row: Option<Row> = sqlx::query_as(&format!(
            "SELECT {FULL} FROM message WHERE source_id = $1 AND remote_id = $2"
        ))
        .bind(source.as_uuid())
        .bind(remote_id)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Metadata only (see [`Self::get`]), newest first.
    pub async fn list(conn: &mut PgConnection, f: &MessageFilter<'_>) -> Result<Vec<Message>> {
        let sources: Vec<Uuid> = f.sources.iter().map(|s| s.as_uuid()).collect();
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {META} FROM message
             WHERE source_id = ANY($1)
               AND ($2::text IS NULL OR category = $2)
               AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM message_member mm
                                                WHERE mm.message_id = message.id AND mm.member_id = $3))
               AND ($4::text IS NULL OR processing_state = $4)
               AND ($5::text IS NULL OR subject ILIKE $5 OR from_addr ILIKE $5 OR from_name ILIKE $5)
               AND ($6::text[] IS NULL OR category IS NULL OR NOT (category = ANY($6)))
               AND ($7::timestamptz IS NULL OR received_at < $7)
             ORDER BY received_at DESC, id DESC LIMIT $8"
        ))
        .bind(&sources)
        .bind(f.category)
        .bind(f.member.map(|m| m.as_uuid()))
        .bind(f.state.as_ref().map(enum_str))
        .bind(f.q.map(like))
        .bind(f.include_ignored_keys)
        .bind(f.before)
        .bind(f.limit.clamp(1, 500))
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn set_labels(
        conn: &mut PgConnection,
        id: MessageId,
        gmail_labels: &[String],
    ) -> Result<()> {
        one(
            sqlx::query("UPDATE message SET gmail_labels = $2 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(gmail_labels)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    /// Stores the classification. `None` leaves a column unchanged.
    pub async fn set_classification(
        conn: &mut PgConnection,
        id: MessageId,
        category: Option<&str>,
        time_sensitive: Option<bool>,
        is_bill: Option<bool>,
    ) -> Result<()> {
        one(sqlx::query(
            "UPDATE message SET category = COALESCE($2, category),
                    time_sensitive = COALESCE($3, time_sensitive), is_bill = COALESCE($4, is_bill)
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(category)
        .bind(time_sensitive)
        .bind(is_bill)
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Replaces the member links.
    pub async fn set_members(
        conn: &mut PgConnection,
        id: MessageId,
        members: &[(MemberId, DecidedBy, f32)],
    ) -> Result<()> {
        sqlx::query("DELETE FROM message_member WHERE message_id = $1")
            .bind(id.as_uuid())
            .execute(&mut *conn)
            .await?;
        let ids: Vec<Uuid> = members.iter().map(|m| m.0.as_uuid()).collect();
        let by: Vec<String> = members.iter().map(|m| enum_str(&m.1)).collect();
        let conf: Vec<f32> = members.iter().map(|m| m.2).collect();
        sqlx::query(
            "INSERT INTO message_member (message_id, member_id, household_id, decided_by, confidence)
             SELECT $1, t.m, (SELECT household_id FROM message WHERE id = $1), t.d, t.c
             FROM unnest($2::uuid[], $3::text[], $4::real[]) AS t(m, d, c)
             ON CONFLICT (message_id, member_id) DO NOTHING",
        )
        .bind(id.as_uuid())
        .bind(&ids)
        .bind(&by)
        .bind(&conf)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn members(
        conn: &mut PgConnection,
        id: MessageId,
    ) -> Result<Vec<(MemberId, DecidedBy, f32)>> {
        let rows: Vec<(Uuid, String, f32)> = sqlx::query_as(
            "SELECT member_id, decided_by, confidence FROM message_member
             WHERE message_id = $1 ORDER BY member_id",
        )
        .bind(id.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|(m, d, c)| Ok((m.into(), parse_enum(&d)?, c)))
            .collect()
    }

    /// Member ids linked to each of the messages (for list views).
    pub async fn members_of(
        conn: &mut PgConnection,
        ids: &[MessageId],
    ) -> Result<HashMap<MessageId, Vec<MemberId>>> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT message_id, member_id FROM message_member WHERE message_id = ANY($1)
             ORDER BY member_id",
        )
        .bind(uuids(ids))
        .fetch_all(conn)
        .await?;
        let mut out: HashMap<MessageId, Vec<MemberId>> = HashMap::new();
        for (m, member) in rows {
            out.entry(m.into()).or_default().push(member.into());
        }
        Ok(out)
    }

    /// Sets the state; `processed` also stamps `processed_at`.
    pub async fn set_state(
        conn: &mut PgConnection,
        id: MessageId,
        state: ProcessingState,
    ) -> Result<()> {
        one(sqlx::query(
            "UPDATE message SET processing_state = $2,
                    processed_at = CASE WHEN $2 = 'processed' THEN now() ELSE processed_at END
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(enum_str(&state))
        .execute(conn)
        .await?
        .rows_affected())
    }

    pub async fn set_eval_sample(conn: &mut PgConnection, id: MessageId, on: bool) -> Result<()> {
        one(
            sqlx::query("UPDATE message SET in_eval_sample = $2 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(on)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    pub async fn set_learning_example(
        conn: &mut PgConnection,
        id: MessageId,
        on: bool,
    ) -> Result<()> {
        one(
            sqlx::query("UPDATE message SET is_learning_example = $2 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(on)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    /// Replaces the attachment list (`[AttachmentMeta]` as jsonb).
    pub async fn attachments_update(
        conn: &mut PgConnection,
        id: MessageId,
        attachments: &serde_json::Value,
    ) -> Result<()> {
        one(
            sqlx::query("UPDATE message SET attachments = $2 WHERE id = $1")
                .bind(id.as_uuid())
                .bind(attachments)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }

    /// Drops body, snippet, headers and attachment text (`ics`, `pdf_text`); keeps the metadata.
    /// Returns the number of messages purged.
    pub async fn purge_content(conn: &mut PgConnection, ids: &[MessageId]) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE message SET body_text = NULL, body_html = NULL, snippet = NULL, headers = NULL,
                    attachments = CASE WHEN jsonb_typeof(attachments) = 'array'
                        THEN (SELECT COALESCE(jsonb_agg(a - 'ics' - 'pdf_text'), '[]'::jsonb)
                              FROM jsonb_array_elements(attachments) a)
                        ELSE attachments END,
                    content_purged_at = now()
             WHERE id = ANY($1) AND content_purged_at IS NULL",
        )
        .bind(uuids(ids))
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Processed or expired messages whose content may be purged under the policy:
    /// `purge_after_processing` all of them; `keep_for_learning` all except eval-sample messages and
    /// the newest `examples_per_category` learning examples per category; `keep_all` none.
    pub async fn retention_candidates(
        conn: &mut PgConnection,
        policy: ContentPolicy,
        examples_per_category: i32,
        limit: i64,
    ) -> Result<Vec<MessageId>> {
        let sql = match policy {
            ContentPolicy::KeepAll => return Ok(Vec::new()),
            ContentPolicy::PurgeAfterProcessing => {
                "SELECT id FROM message
                 WHERE content_purged_at IS NULL AND processing_state IN ('processed', 'expired')
                   AND $1::bigint >= 0
                 ORDER BY received_at, id LIMIT $2"
            }
            ContentPolicy::KeepForLearning => {
                "SELECT id FROM message
                 WHERE content_purged_at IS NULL AND processing_state IN ('processed', 'expired')
                   AND NOT in_eval_sample
                   AND id NOT IN (
                       SELECT id FROM (
                           SELECT id, row_number() OVER (PARTITION BY coalesce(category, '')
                                                         ORDER BY received_at DESC, id) AS rn
                           FROM message WHERE is_learning_example) k
                       WHERE rn <= $1)
                 ORDER BY received_at, id LIMIT $2"
            }
        };
        let rows: Vec<Uuid> = sqlx::query_scalar(sql)
            .bind(examples_per_category as i64)
            .bind(limit)
            .fetch_all(conn)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Pending messages created before `cutoff`.
    pub async fn pending_older_than(
        conn: &mut PgConnection,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<MessageId>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM message WHERE processing_state = 'pending' AND created_at < $1
             ORDER BY created_at, id",
        )
        .bind(cutoff)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Pending messages, oldest first (the `mail.process` backlog).
    pub async fn pending(conn: &mut PgConnection, limit: i64) -> Result<Vec<MessageId>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM message WHERE processing_state = 'pending' ORDER BY created_at, id LIMIT $1",
        )
        .bind(limit)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Next messages to label: processed messages with content that no member has labelled,
    /// round-robin over categories, preferring those whose latest decisions are not final.
    /// `sources` are the readable sources; `exclude` skips ids (the "after" cursor).
    pub async fn label_queue(
        conn: &mut PgConnection,
        household: HouseholdId,
        sources: &[MailSourceId],
        exclude: &[MessageId],
        limit: i64,
    ) -> Result<Vec<MessageId>> {
        let sources: Vec<Uuid> = sources.iter().map(|s| s.as_uuid()).collect();
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "WITH cand AS (
                 SELECT m.id, coalesce(m.category, '') AS cat, m.received_at,
                        EXISTS (SELECT 1 FROM (
                                    SELECT DISTINCT ON (d.decision_point_id) d.is_final
                                    FROM decision d
                                    WHERE d.subject_type = 'message' AND d.subject_id = m.id
                                    ORDER BY d.decision_point_id, d.created_at DESC, d.id DESC) l
                                WHERE NOT l.is_final) AS nonfinal
                 FROM message m
                 WHERE m.household_id = $1 AND m.source_id = ANY($2)
                   AND m.processing_state = 'processed' AND m.content_purged_at IS NULL
                   AND NOT (m.id = ANY($3))
                   AND NOT EXISTS (SELECT 1 FROM decision d
                                   WHERE d.subject_type = 'message' AND d.subject_id = m.id
                                     AND d.decided_by IS NOT NULL)
             ), ranked AS (
                 SELECT id, cat, received_at,
                        row_number() OVER (PARTITION BY cat
                                           ORDER BY nonfinal DESC, received_at DESC, id) AS rn
                 FROM cand
             )
             SELECT id FROM ranked ORDER BY rn, cat, received_at DESC, id LIMIT $4",
        )
        .bind(household.as_uuid())
        .bind(&sources)
        .bind(uuids(exclude))
        .bind(limit)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// `(messages in the eval sample, of those labelled by a member)`.
    pub async fn eval_progress(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<(i64, i64)> {
        let (in_sample, labelled): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE m.in_eval_sample),
                    count(*) FILTER (WHERE m.in_eval_sample AND EXISTS (
                        SELECT 1 FROM decision d WHERE d.subject_type = 'message'
                          AND d.subject_id = m.id AND d.decided_by IS NOT NULL))
             FROM message m WHERE m.household_id = $1",
        )
        .bind(household.as_uuid())
        .fetch_one(conn)
        .await?;
        Ok((in_sample, labelled))
    }
}
