//! M4 AI repos: LLM endpoint, AI settings, call metrics, stage counts and extraction provenance.

use chrono::{DateTime, Utc};
use fa_domain::{
    AiSettings, AiSettingsInput, ExtractOrigin, HouseholdId, LlmBacklog, LlmCallStat, LlmEndpoint,
    LlmEndpointId, LlmEndpointInput, LlmState, MailExtractMode, MessageId, NewLlmCall, StageCount,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{enum_str, parse_enum};
use crate::{Result, StoreError};

pub struct LlmEndpointRepo;

#[derive(sqlx::FromRow)]
struct EndpointRow {
    id: Uuid,
    household_id: Uuid,
    base_url: String,
    model: String,
    dialect: String,
    timeout_ms: i32,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const ENDPOINT_COLS: &str =
    "id, household_id, base_url, model, dialect, timeout_ms, enabled, created_at, updated_at";

impl TryFrom<EndpointRow> for LlmEndpoint {
    type Error = StoreError;
    fn try_from(r: EndpointRow) -> Result<Self> {
        Ok(LlmEndpoint {
            id: r.id.into(),
            household_id: r.household_id.into(),
            base_url: r.base_url,
            model: r.model,
            dialect: parse_enum(&r.dialect)?,
            timeout_ms: u32::try_from(r.timeout_ms)
                .map_err(|_| StoreError::Corrupt("timeout_ms".into()))?,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

impl LlmEndpointRepo {
    pub async fn get(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<Option<LlmEndpoint>> {
        let row: Option<EndpointRow> = sqlx::query_as(&format!(
            "SELECT {ENDPOINT_COLS} FROM llm_endpoint WHERE household_id = $1"
        ))
        .bind(household.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Creates the household's endpoint or updates it in place (the id is kept).
    pub async fn upsert(
        conn: &mut PgConnection,
        household: HouseholdId,
        i: &LlmEndpointInput,
    ) -> Result<LlmEndpoint> {
        let row: EndpointRow = sqlx::query_as(&format!(
            "INSERT INTO llm_endpoint (id, household_id, base_url, model, dialect, timeout_ms, enabled)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (household_id) DO UPDATE SET
                 base_url = EXCLUDED.base_url, model = EXCLUDED.model, dialect = EXCLUDED.dialect,
                 timeout_ms = EXCLUDED.timeout_ms, enabled = EXCLUDED.enabled
             RETURNING {ENDPOINT_COLS}"
        ))
        .bind(LlmEndpointId::new().as_uuid())
        .bind(household.as_uuid())
        .bind(&i.base_url)
        .bind(&i.model)
        .bind(enum_str(&i.dialect))
        .bind(i32::try_from(i.timeout_ms).unwrap_or(i32::MAX))
        .bind(i.enabled)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }
}

pub struct AiSettingsRepo;

#[derive(sqlx::FromRow)]
struct SettingsRow {
    mail_extract_mode: String,
    max_input_chars: i32,
    mail_mode: String,
    catchup_days: i32,
    extract_fallback_cutoff: f32,
    updated_at: DateTime<Utc>,
}

const SETTINGS_COLS: &str = "mail_extract_mode, max_input_chars, mail_mode, catchup_days, extract_fallback_cutoff, updated_at";

impl TryFrom<SettingsRow> for AiSettings {
    type Error = StoreError;
    fn try_from(r: SettingsRow) -> Result<Self> {
        Ok(AiSettings {
            mail_extract_mode: parse_enum(&r.mail_extract_mode)?,
            max_input_chars: u32::try_from(r.max_input_chars)
                .map_err(|_| StoreError::Corrupt("max_input_chars".into()))?,
            mail_mode: parse_enum(&r.mail_mode)?,
            catchup_days: u32::try_from(r.catchup_days)
                .map_err(|_| StoreError::Corrupt("catchup_days".into()))?,
            extract_fallback_cutoff: r.extract_fallback_cutoff,
            updated_at: r.updated_at,
        })
    }
}

impl AiSettingsRepo {
    /// No row means the defaults.
    pub async fn get_or_default(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<AiSettings> {
        let row: Option<SettingsRow> = sqlx::query_as(&format!(
            "SELECT {SETTINGS_COLS} FROM ai_settings WHERE household_id = $1"
        ))
        .bind(household.as_uuid())
        .fetch_optional(conn)
        .await?;
        match row {
            Some(r) => r.try_into(),
            None => Ok(AiSettings::defaults(Utc::now())),
        }
    }

    pub async fn update(
        conn: &mut PgConnection,
        household: HouseholdId,
        mode: MailExtractMode,
        max_input_chars: u32,
    ) -> Result<AiSettings> {
        let row: SettingsRow = sqlx::query_as(
            "INSERT INTO ai_settings (household_id, mail_extract_mode, max_input_chars)
             VALUES ($1, $2, $3)
             ON CONFLICT (household_id) DO UPDATE SET
                 mail_extract_mode = EXCLUDED.mail_extract_mode,
                 max_input_chars = EXCLUDED.max_input_chars
             RETURNING mail_extract_mode, max_input_chars, mail_mode, catchup_days,
                       extract_fallback_cutoff, updated_at",
        )
        .bind(household.as_uuid())
        .bind(enum_str(&mode))
        .bind(i32::try_from(max_input_chars).unwrap_or(i32::MAX))
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    /// Writes every setting (the other `update` leaves the D-107 columns as they are).
    pub async fn update_all(
        conn: &mut PgConnection,
        household: HouseholdId,
        i: &AiSettingsInput,
    ) -> Result<AiSettings> {
        let row: SettingsRow = sqlx::query_as(&format!(
            "INSERT INTO ai_settings (household_id, mail_extract_mode, max_input_chars, mail_mode,
                                      catchup_days, extract_fallback_cutoff)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (household_id) DO UPDATE SET
                 mail_extract_mode = EXCLUDED.mail_extract_mode,
                 max_input_chars = EXCLUDED.max_input_chars,
                 mail_mode = EXCLUDED.mail_mode,
                 catchup_days = EXCLUDED.catchup_days,
                 extract_fallback_cutoff = EXCLUDED.extract_fallback_cutoff
             RETURNING {SETTINGS_COLS}"
        ))
        .bind(household.as_uuid())
        .bind(enum_str(&i.mail_extract_mode))
        .bind(i32::try_from(i.max_input_chars).unwrap_or(i32::MAX))
        .bind(enum_str(&i.mail_mode))
        .bind(i32::try_from(i.catchup_days).unwrap_or(i32::MAX))
        .bind(i.extract_fallback_cutoff)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }
}

/// `message.llm_state` and the catch-up queue (D-107).
pub struct LlmStateRepo;

impl LlmStateRepo {
    pub async fn get(conn: &mut PgConnection, id: MessageId) -> Result<Option<LlmState>> {
        let s: Option<String> = sqlx::query_scalar("SELECT llm_state FROM message WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        s.map(|s| parse_enum(&s)).transpose()
    }

    /// Sets the state unconditionally.
    pub async fn set(conn: &mut PgConnection, id: MessageId, state: LlmState) -> Result<()> {
        sqlx::query("UPDATE message SET llm_state = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(enum_str(&state))
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Sets `to` only when the message is in one of the `from` states; returns whether it did.
    pub async fn transition(
        conn: &mut PgConnection,
        id: MessageId,
        from: &[LlmState],
        to: LlmState,
    ) -> Result<bool> {
        let from: Vec<&str> = from.iter().map(LlmState::as_str).collect();
        Ok(
            sqlx::query("UPDATE message SET llm_state = $3 WHERE id = $1 AND llm_state = ANY($2)")
                .bind(id.as_uuid())
                .bind(&from)
                .bind(enum_str(&to))
                .execute(conn)
                .await?
                .rows_affected()
                > 0,
        )
    }

    pub async fn backlog(conn: &mut PgConnection, household: HouseholdId) -> Result<LlmBacklog> {
        let (awaiting, queued): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE llm_state = 'awaiting'),
                    count(*) FILTER (WHERE llm_state = 'queued')
             FROM message WHERE household_id = $1 AND llm_state IN ('awaiting', 'queued')",
        )
        .bind(household.as_uuid())
        .fetch_one(conn)
        .await?;
        Ok(LlmBacklog {
            awaiting: u32::try_from(awaiting).unwrap_or(u32::MAX),
            queued: u32::try_from(queued).unwrap_or(u32::MAX),
        })
    }

    /// Households with at least one awaiting message (system transaction).
    pub async fn households_awaiting(conn: &mut PgConnection) -> Result<Vec<HouseholdId>> {
        let rows: Vec<uuid::Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT household_id FROM message WHERE llm_state = 'awaiting'",
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Takes up to `limit` awaiting messages received at or after `since`, newest first, and
    /// marks them `queued` in the same statement (two catch-ups never take the same message).
    pub async fn claim_awaiting(
        conn: &mut PgConnection,
        household: HouseholdId,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<MessageId>> {
        let rows: Vec<(uuid::Uuid, DateTime<Utc>)> = sqlx::query_as(
            "UPDATE message SET llm_state = 'queued'
             WHERE id IN (SELECT id FROM message
                          WHERE household_id = $1 AND llm_state = 'awaiting' AND received_at >= $2
                          ORDER BY received_at DESC, id DESC LIMIT $3 FOR UPDATE SKIP LOCKED)
             RETURNING id, received_at",
        )
        .bind(household.as_uuid())
        .bind(since)
        .bind(limit)
        .fetch_all(conn)
        .await?;
        let mut rows = rows;
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));
        Ok(rows.into_iter().map(|r| r.0.into()).collect())
    }

    /// `(attempts, max_attempts)` of the running `mail.llm` job of `message`, if any (system
    /// transaction). The job runner passes no attempt number to its handlers.
    pub async fn running_attempt(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Option<(i32, i32)>> {
        Ok(sqlx::query_as(
            "SELECT attempts, max_attempts FROM job
             WHERE kind = 'mail.llm' AND status = 'running' AND payload->>'message_id' = $1
             ORDER BY attempts DESC LIMIT 1",
        )
        .bind(message.to_string())
        .fetch_optional(conn)
        .await?)
    }
}

/// Still-pending, rule-made proposals of one message that an LLM answer may refine (D-107).
/// Every query keeps to items nobody has touched: approved, rejected, edited, opened, or made by
/// the LLM already are never returned and never updated.
pub struct ProposalRefineRepo;

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub id: Uuid,
    pub title: String,
    pub start_at: DateTime<Utc>,
    pub all_day: bool,
}

#[derive(Debug, Clone)]
pub struct PendingBill {
    pub id: Uuid,
    pub amount_minor: i64,
    pub due_on: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct PendingTask {
    pub id: Uuid,
    pub title: String,
    /// The rule extractor's kind: `reply` or `action`.
    pub kind: String,
}

impl ProposalRefineRepo {
    pub async fn events(conn: &mut PgConnection, message: MessageId) -> Result<Vec<PendingEvent>> {
        let rows: Vec<(Uuid, String, DateTime<Utc>, bool)> = sqlx::query_as(
            "SELECT id, title, start_at, all_day FROM event
             WHERE message_id = $1 AND origin = 'extracted' AND status IN ('extracted', 'proposed')
               AND decided_at IS NULL AND extracted_by = 'rules'
             ORDER BY start_at, id",
        )
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, title, start_at, all_day)| PendingEvent {
                id,
                title,
                start_at,
                all_day,
            })
            .collect())
    }

    pub async fn bills(conn: &mut PgConnection, message: MessageId) -> Result<Vec<PendingBill>> {
        let rows: Vec<(Uuid, i64, Option<chrono::NaiveDate>)> = sqlx::query_as(
            "SELECT id, amount_minor, due_on FROM bill
             WHERE message_id = $1 AND status = 'proposed' AND extracted_by = 'rules' AND version = 1
             ORDER BY created_at, id",
        )
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, amount_minor, due_on)| PendingBill {
                id,
                amount_minor,
                due_on,
            })
            .collect())
    }

    /// Every bill of the message whatever its status (opened, paid, dismissed, edited): the
    /// model's bill for the same amount is the same bill and is not proposed again.
    pub async fn settled_bills(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Vec<PendingBill>> {
        let rows: Vec<(Uuid, i64, Option<chrono::NaiveDate>)> = sqlx::query_as(
            "SELECT id, amount_minor, due_on FROM bill WHERE message_id = $1
             ORDER BY created_at, id",
        )
        .bind(message.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, amount_minor, due_on)| PendingBill {
                id,
                amount_minor,
                due_on,
            })
            .collect())
    }

    pub async fn tasks(conn: &mut PgConnection, message: MessageId) -> Result<Vec<PendingTask>> {
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, title, source_ref->>'mail_task_kind' FROM task
             WHERE source_ref->>'message_id' = $1 AND origin = 'email' AND status = 'suggested'
               AND extracted_by = 'rules' AND version = 1 AND archived_at IS NULL
             ORDER BY created_at, id",
        )
        .bind(message.to_string())
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, title, kind)| PendingTask {
                id,
                title,
                kind: kind.unwrap_or_default(),
            })
            .collect())
    }

    /// Gives a pending event the LLM's title, location and confidence; returns whether it was
    /// still untouched.
    pub async fn refine_event(
        conn: &mut PgConnection,
        id: Uuid,
        title: &str,
        location: Option<&str>,
        confidence: f32,
        dedupe_key: &str,
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE event SET title = $2, location = COALESCE($3, location), confidence = $4,
                    dedupe_key = $5, extracted_by = 'llm', version = version + 1
             WHERE id = $1 AND origin = 'extracted' AND status IN ('extracted', 'proposed')
               AND decided_at IS NULL AND extracted_by = 'rules'",
        )
        .bind(id)
        .bind(title)
        .bind(location)
        .bind(confidence)
        .bind(dedupe_key)
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    /// The dedupe key moves to the LLM's version unless another bill holds it.
    pub async fn refine_bill(
        conn: &mut PgConnection,
        id: Uuid,
        payee: &str,
        due_on: Option<chrono::NaiveDate>,
        confidence: f32,
        dedupe_key: &str,
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE bill SET payee = $2, due_on = COALESCE($3, due_on), confidence = $4,
                    dedupe_key = CASE WHEN EXISTS (SELECT 1 FROM bill o
                                                   WHERE o.household_id = bill.household_id
                                                     AND o.dedupe_key = $5 AND o.id <> bill.id)
                                      THEN dedupe_key ELSE $5 END,
                    extracted_by = 'llm', version = version + 1
             WHERE id = $1 AND status = 'proposed' AND extracted_by = 'rules' AND version = 1",
        )
        .bind(id)
        .bind(payee)
        .bind(due_on)
        .bind(confidence)
        .bind(dedupe_key)
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn refine_task(
        conn: &mut PgConnection,
        id: Uuid,
        title: &str,
        due_at: Option<DateTime<Utc>>,
        kind: &str,
    ) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE task SET title = $2, due_at = COALESCE($3, due_at),
                    source_ref = COALESCE(source_ref, '{}'::jsonb) || jsonb_build_object('mail_task_kind', $4::text),
                    extracted_by = 'llm', version = version + 1
             WHERE id = $1 AND origin = 'email' AND status = 'suggested' AND extracted_by = 'rules'
               AND version = 1 AND archived_at IS NULL",
        )
        .bind(id)
        .bind(title)
        .bind(due_at)
        .bind(kind)
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }
}

pub struct LlmCallRepo;

#[derive(sqlx::FromRow)]
struct StatRow {
    purpose: String,
    total: i64,
    ok: i64,
    p50: Option<f64>,
    p95: Option<f64>,
}

impl LlmCallRepo {
    pub async fn insert(conn: &mut PgConnection, c: &NewLlmCall) -> Result<()> {
        sqlx::query(
            "INSERT INTO llm_call (id, household_id, purpose, model, started_at, latency_ms, ok,
                                   error_kind, prompt_tokens, completion_tokens)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(Uuid::now_v7())
        .bind(c.household_id.as_uuid())
        .bind(&c.purpose)
        .bind(&c.model)
        .bind(c.started_at)
        .bind(c.latency_ms)
        .bind(c.ok)
        .bind(&c.error_kind)
        .bind(c.prompt_tokens)
        .bind(c.completion_tokens)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Deletes calls started before `cutoff` (within the transaction's household, by RLS).
    pub async fn prune_before(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM llm_call WHERE started_at < $1")
            .bind(cutoff)
            .execute(conn)
            .await?
            .rows_affected())
    }

    pub async fn stats_since(
        conn: &mut PgConnection,
        household: HouseholdId,
        since: DateTime<Utc>,
    ) -> Result<Vec<LlmCallStat>> {
        let rows: Vec<StatRow> = sqlx::query_as(
            "SELECT purpose, count(*) AS total, count(*) FILTER (WHERE ok) AS ok,
                    percentile_cont(0.5) WITHIN GROUP (ORDER BY latency_ms) AS p50,
                    percentile_cont(0.95) WITHIN GROUP (ORDER BY latency_ms) AS p95
             FROM llm_call WHERE household_id = $1 AND started_at >= $2
             GROUP BY purpose ORDER BY purpose",
        )
        .bind(household.as_uuid())
        .bind(since)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| LlmCallStat {
                purpose: r.purpose,
                total: r.total,
                ok: r.ok,
                p50_ms: r.p50.map(|v| v.round() as i64),
                p95_ms: r.p95.map(|v| v.round() as i64),
            })
            .collect())
    }
}

impl crate::DecisionRepo {
    /// Decisions per decision point and stage kind (rules, llm, human) since `since`.
    pub async fn stage_counts_since(
        conn: &mut PgConnection,
        household: HouseholdId,
        since: DateTime<Utc>,
    ) -> Result<Vec<StageCount>> {
        let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            "SELECT p.key,
                    count(*) FILTER (WHERE d.stage_kind = 'rules'),
                    count(*) FILTER (WHERE d.stage_kind = 'llm'),
                    count(*) FILTER (WHERE d.stage_kind = 'human')
             FROM decision d JOIN decision_point p ON p.id = d.decision_point_id
             WHERE d.household_id = $1 AND d.created_at >= $2
             GROUP BY p.key ORDER BY p.key",
        )
        .bind(household.as_uuid())
        .bind(since)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(decision_point, rules, llm, human)| StageCount {
                decision_point,
                rules,
                llm,
                human,
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractTarget {
    Event,
    Bill,
    Task,
}

impl ExtractTarget {
    fn table(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Bill => "bill",
            Self::Task => "task",
        }
    }
}

pub struct ExtractedByRepo;

impl ExtractedByRepo {
    pub async fn set(
        conn: &mut PgConnection,
        target: ExtractTarget,
        id: Uuid,
        by: ExtractOrigin,
    ) -> Result<()> {
        let n = sqlx::query(&format!(
            "UPDATE {} SET extracted_by = $2 WHERE id = $1",
            target.table()
        ))
        .bind(id)
        .bind(by.as_str())
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub async fn get(
        conn: &mut PgConnection,
        target: ExtractTarget,
        id: Uuid,
    ) -> Result<ExtractOrigin> {
        let s: Option<String> = sqlx::query_scalar(&format!(
            "SELECT extracted_by FROM {} WHERE id = $1",
            target.table()
        ))
        .bind(id)
        .fetch_optional(conn)
        .await?;
        s.map(|s| parse_enum(&s))
            .transpose()?
            .ok_or(StoreError::NotFound)
    }
}
