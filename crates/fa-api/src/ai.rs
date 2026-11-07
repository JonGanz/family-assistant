//! AI routes (M4, D-093, D-094): the LLM endpoint, per-point `llm` stages, call stats and recipe
//! extraction. Adult-only. Mail content, prompts, responses, API keys and recipe URLs are never
//! logged (a URL is logged as its host).

use std::time::Instant;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use fa_domain::{
    Action, AiSettings, AiSettingsInput, DEFAULT_RULES_THRESHOLD, DecisionPointKey, HouseholdId,
    LlmBacklog, LlmCallStat, LlmDialect, LlmEndpoint, LlmEndpointInput, LlmEndpointView,
    MailExtractMode, MailMode, NewLlmCall, Resource, SecretKind, StageCount, StageKind,
};
use fa_llm::{Dialect, EndpointConfig, LlmClient, LlmError};
use fa_pipeline::{Pipeline, PipelineError};
use fa_recipe::RecipeDraft;
use fa_store::{
    AiSettingsRepo, DecisionPointRepo, DecisionRepo, JobRepo, LlmCallRepo, LlmEndpointRepo,
    LlmStateRepo, NewJob, PipelineRepo, SecretRepo,
};
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;
use url::Url;
use utoipa::ToSchema;

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::routes::{ApiResult, audit, audit_ids, household};
use crate::tasks::opt_body;
use crate::{ApiError, AppState, Auth, require};

const OWNER: &str = "llm_endpoint";
const EMAIL_POINTS: [DecisionPointKey; 4] = [
    DecisionPointKey::EmailCategory,
    DecisionPointKey::EmailTimeSensitive,
    DecisionPointKey::EmailMemberLink,
    DecisionPointKey::EmailIsBill,
];
const DEFAULT_THRESHOLD: f32 = 0.7;

fn adult(actor: &fa_domain::Actor) -> ApiResult<HouseholdId> {
    require(actor, Action::ManageHouseholdSettings, &Resource::Household)?;
    household(actor)
}

/// What a request does to the stored API key: absent keeps it, `null` clears it, a string sets it.
#[derive(Debug, Default)]
pub enum KeyUpdate {
    #[default]
    Keep,
    Clear,
    Set(String),
}

impl<'de> Deserialize<'de> for KeyUpdate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(d)? {
            None => Self::Clear,
            Some(k) => Self::Set(k),
        })
    }
}

/// Body of `PUT /ai/llm-endpoint`. `api_key`: absent keeps the stored key, `null` clears it, a
/// string sets it.
#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmEndpointPut {
    pub base_url: String,
    pub model: String,
    pub dialect: LlmDialect,
    pub timeout_ms: u32,
    pub enabled: bool,
    #[serde(default)]
    #[ts(as = "Option<String>", optional = nullable)]
    #[schema(value_type = Option<String>)]
    pub api_key: KeyUpdate,
}

/// Body of `POST /ai/llm-endpoint/test`: unsaved values; a missing field uses the stored one.
#[derive(Debug, Default, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmEndpointTest {
    #[ts(as = "Option<String>", optional = nullable)]
    pub base_url: Option<String>,
    #[ts(as = "Option<String>", optional = nullable)]
    pub model: Option<String>,
    #[ts(as = "Option<LlmDialect>", optional = nullable)]
    pub dialect: Option<LlmDialect>,
    #[ts(as = "Option<u32>", optional = nullable)]
    pub timeout_ms: Option<u32>,
    /// Absent: the stored key. `null`: no key. A string: that key.
    #[serde(default)]
    #[ts(as = "Option<String>", optional = nullable)]
    #[schema(value_type = Option<String>)]
    pub api_key: KeyUpdate,
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmEndpointTestResult {
    pub ok: bool,
    #[ts(as = "Option<u32>", optional = nullable)]
    pub latency_ms: Option<u32>,
    #[ts(as = "Option<bool>", optional = nullable)]
    pub model_found: Option<bool>,
    /// An `LlmError` kind: `unavailable`, `timeout`, `http`, `bad_json`, `schema` or `config`.
    #[ts(as = "Option<String>", optional = nullable)]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct LlmModels {
    pub models: Vec<String>,
}

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AiSettingsPut {
    pub mail_extract_mode: MailExtractMode,
    pub max_input_chars: u32,
    /// Absent keeps the stored value (default `llm_first`).
    #[serde(default)]
    #[ts(as = "Option<MailMode>", optional = nullable)]
    pub mail_mode: Option<MailMode>,
    /// 0 to 365; 0 switches the catch-up off. Absent keeps the stored value (default 30).
    #[serde(default)]
    #[ts(as = "Option<u32>", optional = nullable)]
    pub catchup_days: Option<u32>,
    /// 0.5 to 0.99. Absent keeps the stored value (default 0.8).
    #[serde(default)]
    #[ts(as = "Option<f32>", optional = nullable)]
    pub extract_fallback_cutoff: Option<f32>,
}

/// The `llm` stage of one email decision point.
#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AiPipelinePoint {
    pub decision_point: String,
    pub llm_enabled: bool,
    /// Whether the active version has an `llm` stage at all. In `llm_first` a point without one is
    /// still asked, so the UI shows it as on unless a stage exists and is switched off (D-107).
    pub llm_stage_present: bool,
    pub llm_threshold: f32,
    /// The `rules` stage's confidence bar (`rules_first`: below it the LLM is asked). 0.5 to 0.99,
    /// default 0.8.
    pub rules_threshold: f32,
    /// The active pipeline version (uuid).
    pub version_id: String,
}

#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AiPipelinePut {
    pub llm_enabled: bool,
    /// 0 to 1.
    pub llm_threshold: f32,
    /// 0.5 to 0.99; absent keeps the stored one. A change is a new pipeline version.
    #[serde(default)]
    #[ts(as = "Option<f32>", optional = nullable)]
    pub rules_threshold: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    days: Option<i64>,
}

#[derive(Debug, Serialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AiStats {
    /// RFC 3339.
    pub since: DateTime<Utc>,
    pub calls: Vec<LlmCallStat>,
    pub stages: Vec<StageCount>,
}

/// Exactly one of `url` and `html`.
#[derive(Debug, Deserialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct RecipeExtractReq {
    #[ts(as = "Option<String>", optional = nullable)]
    pub url: Option<String>,
    #[ts(as = "Option<String>", optional = nullable)]
    pub html: Option<String>,
}

fn field(name: &str, message: &str) -> ApiError {
    ApiError::FieldInvalid {
        field: name.into(),
        position: None,
        message: message.into(),
    }
}

fn not_configured() -> ApiError {
    ApiError::coded(
        StatusCode::NOT_FOUND,
        "not_configured",
        "no LLM endpoint is configured",
    )
}

fn bad_request(msg: &str) -> ApiError {
    ApiError::coded(StatusCode::UNPROCESSABLE_ENTITY, "bad_request", msg)
}

/// Trims a trailing slash; `http` or `https`, a host, no userinfo, query or fragment.
fn clean_base_url(raw: &str) -> ApiResult<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let u = Url::parse(trimmed).map_err(|_| field("base_url", "is not a valid URL"))?;
    if !matches!(u.scheme(), "http" | "https") || u.host_str().is_none() {
        return Err(field("base_url", "must be an http or https URL"));
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err(field("base_url", "must not contain credentials"));
    }
    if u.query().is_some() || u.fragment().is_some() {
        return Err(field("base_url", "must not have a query or fragment"));
    }
    Ok(trimmed.to_owned())
}

fn check_model(m: &str) -> ApiResult<String> {
    let m = m.trim();
    if m.is_empty() {
        return Err(field("model", "must not be empty"));
    }
    Ok(m.to_owned())
}

fn check_timeout(ms: u32) -> ApiResult<u32> {
    if !(1000..=600_000).contains(&ms) {
        return Err(field("timeout_ms", "must be between 1000 and 600000"));
    }
    Ok(ms)
}

fn dialect(d: LlmDialect) -> Dialect {
    match d {
        LlmDialect::OpenaiChat => Dialect::OpenaiChat,
        LlmDialect::OllamaNative => Dialect::OllamaNative,
    }
}

fn http() -> ApiResult<reqwest::Client> {
    fa_llm::default_http().map_err(|e| ApiError::Internal(e.to_string()))
}

fn view(ep: &LlmEndpoint, has_api_key: bool) -> LlmEndpointView {
    LlmEndpointView {
        base_url: ep.base_url.clone(),
        model: ep.model.clone(),
        dialect: ep.dialect,
        has_api_key,
        timeout_ms: ep.timeout_ms,
        enabled: ep.enabled,
        updated_at: ep.updated_at,
    }
}

async fn stored_key(
    s: &AppState,
    conn: &mut sqlx::PgConnection,
    hid: HouseholdId,
    ep: &LlmEndpoint,
) -> ApiResult<Option<String>> {
    let b = SecretRepo::get(
        conn,
        &s.services.mail.key,
        hid,
        OWNER,
        ep.id.as_uuid(),
        SecretKind::ApiKey,
    )
    .await?;
    Ok(b.and_then(|b| String::from_utf8(b).ok()))
}

fn invalidate(s: &AppState, hid: HouseholdId) {
    if let Some(p) = &s.services.mail.llm {
        p.invalidate(hid);
    }
}

#[utoipa::path(get, path = "/api/v1/ai/llm-endpoint", tag = "ai", security(("cookie" = [])),
    responses((status = 200, body = LlmEndpointView), (status = 403, body = Problem),
              (status = 404, body = Problem, description = "code `not_configured`")))]
pub async fn get_endpoint(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<LlmEndpointView>> {
    let hid = adult(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let ep = LlmEndpointRepo::get(&mut tx, hid)
        .await?
        .ok_or_else(not_configured)?;
    let has_key = stored_key(&s, &mut tx, hid, &ep).await?.is_some();
    tx.rollback().await?;
    Ok(Json(view(&ep, has_key)))
}

#[utoipa::path(put, path = "/api/v1/ai/llm-endpoint", tag = "ai", request_body = LlmEndpointPut,
    security(("cookie" = [])),
    responses((status = 200, body = LlmEndpointView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_endpoint(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<LlmEndpointPut>,
) -> ApiResult<Json<LlmEndpointView>> {
    let hid = adult(&auth.actor)?;
    let input = LlmEndpointInput {
        base_url: clean_base_url(&req.base_url)?,
        model: check_model(&req.model)?,
        dialect: req.dialect,
        timeout_ms: check_timeout(req.timeout_ms)?,
        enabled: req.enabled,
    };
    let key_change = match &req.api_key {
        KeyUpdate::Keep => "kept",
        KeyUpdate::Clear => "cleared",
        KeyUpdate::Set(_) => "set",
    };
    let mut tx = s.store.tx_for(hid).await?;
    let ep = LlmEndpointRepo::upsert(&mut tx, hid, &input).await?;
    match req.api_key {
        KeyUpdate::Keep => {}
        KeyUpdate::Clear => {
            SecretRepo::delete(&mut tx, hid, OWNER, ep.id.as_uuid(), SecretKind::ApiKey).await?;
        }
        KeyUpdate::Set(k) if k.trim().is_empty() => {
            return Err(field("api_key", "must not be empty (use null to clear it)"));
        }
        KeyUpdate::Set(k) => {
            SecretRepo::put(
                &mut tx,
                &s.services.mail.key,
                hid,
                OWNER,
                ep.id.as_uuid(),
                SecretKind::ApiKey,
                k.trim().as_bytes(),
                audit_ids(&auth.actor).0,
            )
            .await?;
        }
    }
    let has_key = stored_key(&s, &mut tx, hid, &ep).await?.is_some();
    audit(
        &mut tx,
        &auth.actor,
        "ai.llm_endpoint.set",
        "llm_endpoint",
        Some(ep.id.to_string()),
        Some(serde_json::json!({
            "base_url": ep.base_url, "model": ep.model, "dialect": ep.dialect,
            "timeout_ms": ep.timeout_ms, "enabled": ep.enabled, "api_key": key_change,
        })),
    )
    .await?;
    if ep.enabled {
        // Messages that waited for an LLM are queued once the endpoint answers (D-107).
        let mut job = NewJob::new("mail.llm.catchup");
        job.household_id = Some(hid);
        job.payload = serde_json::json!({"household": hid});
        job.idempotency_key = Some(format!(
            "mail.llm.catchup:{hid}:{}",
            Utc::now().timestamp_millis()
        ));
        JobRepo::enqueue(&mut tx, &job).await?;
    }
    tx.commit().await?;
    invalidate(&s, hid);
    Ok(Json(view(&ep, has_key)))
}

#[utoipa::path(post, path = "/api/v1/ai/llm-endpoint/test", tag = "ai",
    request_body(content = Option<LlmEndpointTest>, description = "unsaved values; an empty body tests the stored ones"),
    security(("cookie" = [])),
    responses((status = 200, body = LlmEndpointTestResult, description = "`ok: false` when the server is unreachable"),
              (status = 403, body = Problem), (status = 422, body = Problem)))]
pub async fn test_endpoint(
    State(s): State<AppState>,
    auth: Auth,
    body: Bytes,
) -> ApiResult<Json<LlmEndpointTestResult>> {
    let hid = adult(&auth.actor)?;
    let req: LlmEndpointTest = opt_body(&body)?.unwrap_or_default();
    let mut tx = s.store.tx_for(hid).await?;
    let stored = LlmEndpointRepo::get(&mut tx, hid).await?;
    let stored_api_key = match (&stored, &req.api_key) {
        (Some(ep), KeyUpdate::Keep) => stored_key(&s, &mut tx, hid, ep).await?,
        _ => None,
    };
    tx.rollback().await?;
    let base = match (&req.base_url, &stored) {
        (Some(b), _) => b.clone(),
        (None, Some(ep)) => ep.base_url.clone(),
        (None, None) => return Err(bad_request("no endpoint to test")),
    };
    let model = match (&req.model, &stored) {
        (Some(m), _) => m.clone(),
        (None, Some(ep)) => ep.model.clone(),
        (None, None) => return Err(bad_request("no endpoint to test")),
    };
    let model = check_model(&model)?;
    let base_url =
        Url::parse(&clean_base_url(&base)?).map_err(|_| field("base_url", "is not a valid URL"))?;
    let timeout_ms = check_timeout(
        req.timeout_ms
            .or(stored.as_ref().map(|e| e.timeout_ms))
            .unwrap_or(fa_domain::DEFAULT_LLM_TIMEOUT_MS),
    )?;
    let d = req
        .dialect
        .or(stored.as_ref().map(|e| e.dialect))
        .unwrap_or_default();
    let api_key = match req.api_key {
        KeyUpdate::Set(k) => Some(k).filter(|k| !k.trim().is_empty()),
        KeyUpdate::Clear => None,
        KeyUpdate::Keep => stored_api_key,
    };
    let client = LlmClient::new(
        EndpointConfig {
            base_url,
            model: model.clone(),
            api_key,
            timeout: std::time::Duration::from_millis(u64::from(timeout_ms)),
            dialect: dialect(d),
        },
        http()?,
    );
    let started_at = Utc::now();
    let t0 = Instant::now();
    let res = client.ping().await;
    let latency = t0.elapsed();
    record_call(
        &s,
        hid,
        "endpoint.test",
        &model,
        started_at,
        latency,
        res.as_ref().err(),
    )
    .await;
    let out = match res {
        Ok(p) => LlmEndpointTestResult {
            ok: true,
            latency_ms: Some(u32::try_from(p.latency.as_millis()).unwrap_or(u32::MAX)),
            model_found: Some(p.model_found),
            error: None,
        },
        Err(e) => LlmEndpointTestResult {
            ok: false,
            latency_ms: None,
            model_found: None,
            error: Some(e.kind().to_owned()),
        },
    };
    Ok(Json(out))
}

/// Writes one `llm_call` row; a failure is logged and ignored.
async fn record_call(
    s: &AppState,
    hid: HouseholdId,
    purpose: &str,
    model: &str,
    started_at: DateTime<Utc>,
    latency: std::time::Duration,
    err: Option<&LlmError>,
) {
    let rec = NewLlmCall {
        household_id: hid,
        purpose: purpose.to_owned(),
        model: model.to_owned(),
        started_at,
        latency_ms: i32::try_from(latency.as_millis()).unwrap_or(i32::MAX),
        ok: err.is_none(),
        error_kind: err.map(|e| e.kind().to_owned()),
        prompt_tokens: None,
        completion_tokens: None,
    };
    let res = async {
        let mut tx = s.store.tx_for(hid).await?;
        LlmCallRepo::insert(&mut tx, &rec).await?;
        tx.commit().await
    }
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, "llm_call not recorded");
    }
}

#[utoipa::path(get, path = "/api/v1/ai/llm-endpoint/models", tag = "ai", security(("cookie" = [])),
    responses((status = 200, body = LlmModels), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 502, body = Problem, description = "code `llm_unavailable`")))]
pub async fn list_models(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<LlmModels>> {
    let hid = adult(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let ep = LlmEndpointRepo::get(&mut tx, hid)
        .await?
        .ok_or_else(not_configured)?;
    let api_key = stored_key(&s, &mut tx, hid, &ep).await?;
    tx.rollback().await?;
    let base_url = Url::parse(&ep.base_url).map_err(|_| field("base_url", "is not a valid URL"))?;
    let client = LlmClient::new(
        EndpointConfig {
            base_url,
            model: ep.model,
            api_key,
            timeout: std::time::Duration::from_millis(u64::from(ep.timeout_ms)),
            dialect: dialect(ep.dialect),
        },
        http()?,
    );
    match client.models().await {
        Ok(models) => Ok(Json(LlmModels { models })),
        Err(e) => Err(llm_error(&e)),
    }
}

fn llm_error(e: &LlmError) -> ApiError {
    match e {
        LlmError::Unavailable(_) | LlmError::Timeout => ApiError::coded(
            StatusCode::BAD_GATEWAY,
            "llm_unavailable",
            "the LLM server is not reachable",
        ),
        _ => ApiError::coded(
            StatusCode::BAD_GATEWAY,
            "llm_failed",
            format!("the LLM server failed ({})", e.kind()),
        ),
    }
}

#[utoipa::path(get, path = "/api/v1/ai/settings", tag = "ai", security(("cookie" = [])),
    responses((status = 200, body = AiSettings), (status = 403, body = Problem)))]
pub async fn get_settings(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<AiSettings>> {
    let hid = adult(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let v = AiSettingsRepo::get_or_default(&mut tx, hid).await?;
    tx.rollback().await?;
    Ok(Json(v))
}

#[utoipa::path(put, path = "/api/v1/ai/settings", tag = "ai", request_body = AiSettingsPut,
    security(("cookie" = [])),
    responses((status = 200, body = AiSettings), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_settings(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<AiSettingsPut>,
) -> ApiResult<Json<AiSettings>> {
    let hid = adult(&auth.actor)?;
    if !(1000..=100_000).contains(&req.max_input_chars) {
        return Err(field("max_input_chars", "must be between 1000 and 100000"));
    }
    if req.catchup_days.is_some_and(|d| d > 365) {
        return Err(field("catchup_days", "must be between 0 and 365"));
    }
    if req
        .extract_fallback_cutoff
        .is_some_and(|c| !c.is_finite() || !(0.5..=0.99).contains(&c))
    {
        return Err(field(
            "extract_fallback_cutoff",
            "must be between 0.5 and 0.99",
        ));
    }
    let mut tx = s.store.tx_for(hid).await?;
    let cur = AiSettingsRepo::get_or_default(&mut tx, hid).await?;
    let v = AiSettingsRepo::update_all(
        &mut tx,
        hid,
        &AiSettingsInput {
            mail_extract_mode: req.mail_extract_mode,
            max_input_chars: req.max_input_chars,
            mail_mode: req.mail_mode.unwrap_or(cur.mail_mode),
            catchup_days: req.catchup_days.unwrap_or(cur.catchup_days),
            extract_fallback_cutoff: req
                .extract_fallback_cutoff
                .unwrap_or(cur.extract_fallback_cutoff),
        },
    )
    .await?;
    audit(
        &mut tx,
        &auth.actor,
        "ai.settings.set",
        "ai_settings",
        None,
        Some(serde_json::json!({
            "mail_extract_mode": v.mail_extract_mode, "max_input_chars": v.max_input_chars,
            "mail_mode": v.mail_mode, "catchup_days": v.catchup_days,
            "extract_fallback_cutoff": v.extract_fallback_cutoff,
        })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(v))
}

/// Messages waiting for the LLM (`awaiting`) and queued for it.
#[utoipa::path(get, path = "/api/v1/ai/backlog", tag = "ai", security(("cookie" = [])),
    responses((status = 200, body = LlmBacklog), (status = 403, body = Problem)))]
pub async fn get_backlog(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<LlmBacklog>> {
    let hid = adult(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    let v = LlmStateRepo::backlog(&mut tx, hid).await?;
    tx.rollback().await?;
    Ok(Json(v))
}

fn pipeline_err(e: PipelineError) -> ApiError {
    match e {
        PipelineError::Invalid(m) => ApiError::Unprocessable(m),
        PipelineError::Store(s) => s.into(),
    }
}

async fn point_view(
    conn: &mut sqlx::PgConnection,
    dp: DecisionPointKey,
) -> ApiResult<AiPipelinePoint> {
    let point = DecisionPointRepo::get_by_key(conn, dp)
        .await?
        .ok_or_else(|| ApiError::Internal(format!("decision point {} missing", dp.as_str())))?;
    let (version, stages) = PipelineRepo::active_version(conn, point.id)
        .await?
        .ok_or_else(|| ApiError::Internal(format!("no active pipeline for {}", dp.as_str())))?;
    let llm = stages.iter().find(|s| s.kind == StageKind::Llm);
    Ok(AiPipelinePoint {
        decision_point: dp.as_str().to_owned(),
        llm_enabled: llm.is_some_and(|s| s.enabled),
        llm_stage_present: llm.is_some(),
        llm_threshold: llm.map_or(DEFAULT_THRESHOLD, |s| s.accept_threshold),
        rules_threshold: stages
            .iter()
            .find(|s| s.kind == StageKind::Rules)
            .map_or(DEFAULT_RULES_THRESHOLD, |s| s.accept_threshold),
        version_id: version.id.to_string(),
    })
}

#[utoipa::path(get, path = "/api/v1/ai/pipeline", tag = "ai", security(("cookie" = [])),
    responses((status = 200, body = Vec<AiPipelinePoint>), (status = 403, body = Problem)))]
pub async fn get_pipeline(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<AiPipelinePoint>>> {
    let hid = adult(&auth.actor)?;
    let mut tx = s.store.tx_for(hid).await?;
    Pipeline::ensure_defaults(&mut tx, hid)
        .await
        .map_err(pipeline_err)?;
    let mut out = Vec::new();
    for dp in EMAIL_POINTS {
        out.push(point_view(&mut tx, dp).await?);
    }
    tx.commit().await?;
    Ok(Json(out))
}

#[utoipa::path(put, path = "/api/v1/ai/pipeline/{decision_point}", tag = "ai",
    request_body = AiPipelinePut,
    params(("decision_point" = String, Path, description = "one of the four email decision points")),
    security(("cookie" = [])),
    responses((status = 200, body = AiPipelinePoint), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn put_pipeline(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(key): ApiPath<String>,
    ApiJson(req): ApiJson<AiPipelinePut>,
) -> ApiResult<Json<AiPipelinePoint>> {
    let hid = adult(&auth.actor)?;
    let dp: DecisionPointKey = key.parse().map_err(|_| ApiError::NotFound)?;
    if !EMAIL_POINTS.contains(&dp) {
        return Err(ApiError::NotFound);
    }
    if !req.llm_threshold.is_finite() || !(0.0..=1.0).contains(&req.llm_threshold) {
        return Err(field("llm_threshold", "must be between 0 and 1"));
    }
    if req
        .rules_threshold
        .is_some_and(|t| !t.is_finite() || !(0.5..=0.99).contains(&t))
    {
        return Err(field("rules_threshold", "must be between 0.5 and 0.99"));
    }
    let mut tx = s.store.tx_for(hid).await?;
    Pipeline::ensure_defaults(&mut tx, hid)
        .await
        .map_err(pipeline_err)?;
    Pipeline::set_stages(
        &mut tx,
        hid,
        dp,
        req.llm_enabled,
        req.llm_threshold,
        req.rules_threshold,
    )
    .await
    .map_err(pipeline_err)?;
    let out = point_view(&mut tx, dp).await?;
    audit(
        &mut tx,
        &auth.actor,
        "ai.pipeline.set",
        "decision_point",
        Some(dp.as_str().to_owned()),
        Some(serde_json::json!({
            "llm_enabled": out.llm_enabled, "llm_threshold": out.llm_threshold,
            "rules_threshold": out.rules_threshold,
        })),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(out))
}

#[utoipa::path(get, path = "/api/v1/ai/stats", tag = "ai",
    params(("days" = Option<i64>, Query, description = "default 7, 1 to 90")),
    security(("cookie" = [])),
    responses((status = 200, body = AiStats), (status = 403, body = Problem)))]
pub async fn get_stats(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<StatsQuery>,
) -> ApiResult<Json<AiStats>> {
    let hid = adult(&auth.actor)?;
    let days = q.days.unwrap_or(7).clamp(1, 90);
    let since = Utc::now() - Duration::days(days);
    let mut tx = s.store.tx_for(hid).await?;
    let calls = LlmCallRepo::stats_since(&mut tx, hid, since).await?;
    let stages = DecisionRepo::stage_counts_since(&mut tx, hid, since).await?;
    tx.rollback().await?;
    Ok(Json(AiStats {
        since,
        calls,
        stages,
    }))
}

fn fetch_failed() -> ApiError {
    ApiError::coded(
        StatusCode::UNPROCESSABLE_ENTITY,
        "fetch_failed",
        "the page could not be fetched",
    )
}

#[utoipa::path(post, path = "/api/v1/ai/recipes/extract", tag = "ai", request_body = RecipeExtractReq,
    security(("cookie" = [])),
    responses((status = 200, body = RecipeDraft), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "codes `bad_request`, `fetch_failed`, `no_recipe`"),
              (status = 502, body = Problem), (status = 503, body = Problem, description = "code `llm_unavailable`")))]
pub async fn extract_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<RecipeExtractReq>,
) -> ApiResult<Json<RecipeDraft>> {
    let hid = adult(&auth.actor)?;
    let (html, source) = match (req.url.as_deref(), req.html) {
        (Some(u), None) => {
            let url = Url::parse(u.trim()).map_err(|_| bad_request("url is not a valid URL"))?;
            tracing::info!(host = url.host_str().unwrap_or(""), "recipe fetch");
            let http = fa_connectors::Http::with_policy(true, false)
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            let html = fa_recipe::fetch_page(&http, &url).await.map_err(|e| {
                tracing::info!(kind = ?std::mem::discriminant(&e), "recipe fetch failed");
                fetch_failed()
            })?;
            (html, Some(url))
        }
        (None, Some(h)) if !h.trim().is_empty() => (h, None),
        _ => return Err(bad_request("send exactly one of url and html")),
    };
    if let Some(d) = fa_recipe::from_html(&html, source.as_ref()) {
        return Ok(Json(d));
    }
    let client = match &s.services.mail.llm {
        Some(p) => p.client_for(hid).await.map_err(|e| llm_error(&e))?,
        None => None,
    };
    let Some(client) = client else {
        return Err(llm_unavailable());
    };
    let mut tx = s.store.tx_for(hid).await?;
    let max = AiSettingsRepo::get_or_default(&mut tx, hid)
        .await?
        .max_input_chars as usize;
    let model = LlmEndpointRepo::get(&mut tx, hid)
        .await?
        .map(|e| e.model)
        .unwrap_or_default();
    tx.rollback().await?;
    let text: String = fa_recipe::visible_text(&html).chars().take(max).collect();
    let started_at = Utc::now();
    let t0 = Instant::now();
    let res = fa_recipe::from_llm(&client, &text, source.as_ref()).await;
    let latency = t0.elapsed();
    // "no recipe" is a model answer, not a transport failure.
    let transport_err = res
        .as_ref()
        .err()
        .filter(|e| !matches!(e, LlmError::Schema(m) if m == "no recipe"));
    record_call(
        &s,
        hid,
        "recipe.extract",
        &model,
        started_at,
        latency,
        transport_err,
    )
    .await;
    match res {
        Ok(d) => Ok(Json(d)),
        Err(LlmError::Schema(m)) if m == "no recipe" => Err(ApiError::coded(
            StatusCode::UNPROCESSABLE_ENTITY,
            "no_recipe",
            "no recipe was found on the page",
        )),
        Err(e) => Err(llm_error(&e)),
    }
}

fn llm_unavailable() -> ApiError {
    ApiError::coded(
        StatusCode::SERVICE_UNAVAILABLE,
        "llm_unavailable",
        "the page has no recipe data and the LLM is off",
    )
}
