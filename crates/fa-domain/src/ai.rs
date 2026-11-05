//! M4 AI types: LLM endpoint, AI settings, call statistics (plan/M4.md, arch/AI.md).
//!
//! Wire formats: instants RFC 3339 (`DateTime<Utc>`); `timeout_ms` and `max_input_chars` integers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::entities::{ParseEnumError, db_enum, text_enum};
use crate::ids::{HouseholdId, LlmEndpointId};

db_enum!(LlmDialect { OpenaiChat => "openai_chat", OllamaNative => "ollama_native" });
db_enum!(MailExtractMode { Off => "off", Fallback => "fallback", Always => "always" });
db_enum!(
    /// Who produced an extracted event, bill or task.
    ExtractOrigin { Rules => "rules", Llm => "llm" }
);

#[allow(clippy::derivable_impls)]
impl Default for LlmDialect {
    fn default() -> Self {
        Self::OpenaiChat
    }
}

#[allow(clippy::derivable_impls)]
impl Default for MailExtractMode {
    fn default() -> Self {
        Self::Fallback
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ExtractOrigin {
    fn default() -> Self {
        Self::Rules
    }
}

pub const DEFAULT_LLM_TIMEOUT_MS: u32 = 120_000;
pub const DEFAULT_MAX_INPUT_CHARS: u32 = 12_000;

/// The endpoint row. The API key is a `secret` row (owner type `llm_endpoint`), never a column.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmEndpoint {
    pub id: LlmEndpointId,
    pub household_id: HouseholdId,
    pub base_url: String,
    pub model: String,
    pub dialect: LlmDialect,
    pub timeout_ms: u32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What `PUT /ai/llm-endpoint` writes (the key travels separately).
#[derive(Debug, Clone, PartialEq)]
pub struct LlmEndpointInput {
    pub base_url: String,
    pub model: String,
    pub dialect: LlmDialect,
    pub timeout_ms: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmEndpointView {
    pub base_url: String,
    pub model: String,
    pub dialect: LlmDialect,
    pub has_api_key: bool,
    pub timeout_ms: u32,
    pub enabled: bool,
    /// RFC 3339.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AiSettings {
    pub mail_extract_mode: MailExtractMode,
    pub max_input_chars: u32,
    /// RFC 3339.
    pub updated_at: DateTime<Utc>,
}

impl AiSettings {
    /// The settings of a household with no row yet.
    pub fn defaults(now: DateTime<Utc>) -> Self {
        Self {
            mail_extract_mode: MailExtractMode::default(),
            max_input_chars: DEFAULT_MAX_INPUT_CHARS,
            updated_at: now,
        }
    }
}

/// One `llm_call` row to insert (metrics only, never content).
#[derive(Debug, Clone)]
pub struct NewLlmCall {
    pub household_id: HouseholdId,
    pub purpose: String,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub latency_ms: i32,
    pub ok: bool,
    pub error_kind: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmCallStat {
    pub purpose: String,
    pub total: i64,
    pub ok: i64,
    #[ts(as = "Option<i64>", optional = nullable)]
    pub p50_ms: Option<i64>,
    #[ts(as = "Option<i64>", optional = nullable)]
    pub p95_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct StageCount {
    pub decision_point: String,
    pub rules: i64,
    pub llm: i64,
    pub human: i64,
}
