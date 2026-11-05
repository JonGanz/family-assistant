//! M4 AI repos: LLM endpoint, AI settings, call metrics, stage counts and extraction provenance.

use chrono::{DateTime, Utc};
use fa_domain::{
    AiSettings, ExtractOrigin, HouseholdId, LlmCallStat, LlmEndpoint, LlmEndpointId,
    LlmEndpointInput, MailExtractMode, NewLlmCall, StageCount,
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
    updated_at: DateTime<Utc>,
}

impl TryFrom<SettingsRow> for AiSettings {
    type Error = StoreError;
    fn try_from(r: SettingsRow) -> Result<Self> {
        Ok(AiSettings {
            mail_extract_mode: parse_enum(&r.mail_extract_mode)?,
            max_input_chars: u32::try_from(r.max_input_chars)
                .map_err(|_| StoreError::Corrupt("max_input_chars".into()))?,
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
        let row: Option<SettingsRow> = sqlx::query_as(
            "SELECT mail_extract_mode, max_input_chars, updated_at FROM ai_settings
             WHERE household_id = $1",
        )
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
             RETURNING mail_extract_mode, max_input_chars, updated_at",
        )
        .bind(household.as_uuid())
        .bind(enum_str(&mode))
        .bind(i32::try_from(max_input_chars).unwrap_or(i32::MAX))
        .fetch_one(conn)
        .await?;
        row.try_into()
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
