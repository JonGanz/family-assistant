//! `Provider`: the household's LLM client (endpoint row and sealed key, 30 s cache), one call in
//! flight per household, and an `llm_call` metrics row per call (never content).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use chrono::Utc;
use fa_domain::{HouseholdId, LlmDialect, NewLlmCall, SecretKind};
use fa_store::{LlmCallRepo, LlmEndpointRepo, MasterKey, SecretRepo, Store};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use tokio::sync::Semaphore;

use crate::{Dialect, EndpointConfig, ExtractRequest, Extracted, LlmClient, LlmError};

const CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct Cached {
    client: LlmClient,
    model: String,
}

pub struct Provider {
    store: Store,
    key: Arc<MasterKey>,
    http: reqwest::Client,
    cache: Mutex<HashMap<HouseholdId, (Instant, Option<Cached>)>>,
    gates: Mutex<HashMap<HouseholdId, Arc<Semaphore>>>,
}

fn unavailable(what: &str, e: impl std::fmt::Display) -> LlmError {
    LlmError::Unavailable(format!("{what}: {e}"))
}

impl Provider {
    pub fn new(store: Store, key: Arc<MasterKey>, http: reqwest::Client) -> Self {
        Self {
            store,
            key,
            http,
            cache: Mutex::default(),
            gates: Mutex::default(),
        }
    }

    /// `None` when the household has no endpoint row or it is disabled.
    pub async fn client_for(&self, household: HouseholdId) -> Result<Option<LlmClient>, LlmError> {
        Ok(self.cached(household).await?.map(|c| c.client))
    }

    async fn cached(&self, household: HouseholdId) -> Result<Option<Cached>, LlmError> {
        {
            let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((at, c)) = cache.get(&household)
                && at.elapsed() < CACHE_TTL
            {
                return Ok(c.clone());
            }
        }
        let loaded = self.load(household).await?;
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(household, (Instant::now(), loaded.clone()));
        Ok(loaded)
    }

    async fn load(&self, household: HouseholdId) -> Result<Option<Cached>, LlmError> {
        let mut tx = self
            .store
            .tx_for(household)
            .await
            .map_err(|e| unavailable("store", e))?;
        let Some(ep) = LlmEndpointRepo::get(&mut tx, household)
            .await
            .map_err(|e| unavailable("endpoint lookup", e))?
        else {
            return Ok(None);
        };
        if !ep.enabled {
            return Ok(None);
        }
        let api_key = SecretRepo::get(
            &mut tx,
            &self.key,
            household,
            "llm_endpoint",
            ep.id.as_uuid(),
            SecretKind::ApiKey,
        )
        .await
        .map_err(|e| unavailable("secret", e))?
        .and_then(|b| String::from_utf8(b).ok());
        let base_url = url::Url::parse(&ep.base_url)
            .map_err(|_| LlmError::Config("base URL is not a valid URL".into()))?;
        let cfg = EndpointConfig {
            base_url,
            model: ep.model.clone(),
            api_key,
            timeout: Duration::from_millis(u64::from(ep.timeout_ms)),
            thinking: ep.thinking,
            dialect: match ep.dialect {
                LlmDialect::OpenaiChat => Dialect::OpenaiChat,
                LlmDialect::OllamaNative => Dialect::OllamaNative,
            },
        };
        Ok(Some(Cached {
            client: LlmClient::new(cfg, self.http.clone()),
            model: ep.model,
        }))
    }

    /// Takes the household's permit, calls the model and records the call's metrics.
    pub async fn call<T: DeserializeOwned + JsonSchema>(
        &self,
        household: HouseholdId,
        purpose: &'static str,
        req: &ExtractRequest,
    ) -> Result<Extracted<T>, LlmError> {
        let Some(c) = self.cached(household).await? else {
            return Err(LlmError::Config("disabled".into()));
        };
        let gate = self
            .gates
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(household)
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone();
        let _permit = gate
            .acquire_owned()
            .await
            .map_err(|_| LlmError::Config("provider closed".into()))?;
        let started_at = Utc::now();
        let t0 = Instant::now();
        let res = c.client.extract::<T>(req).await;
        let (latency, tokens) = match &res {
            Ok(x) => (x.latency, (x.prompt_tokens, x.completion_tokens)),
            Err(_) => (t0.elapsed(), (None, None)),
        };
        let rec = NewLlmCall {
            household_id: household,
            purpose: purpose.to_owned(),
            model: c.model,
            started_at,
            latency_ms: i32::try_from(latency.as_millis()).unwrap_or(i32::MAX),
            ok: res.is_ok(),
            error_kind: res.as_ref().err().map(|e| e.kind().to_owned()),
            prompt_tokens: tokens.0.and_then(|n| i32::try_from(n).ok()),
            completion_tokens: tokens.1.and_then(|n| i32::try_from(n).ok()),
        };
        if let Err(e) = self.record(household, &rec).await {
            tracing::warn!(error = %e, "llm_call not recorded");
        }
        res
    }

    async fn record(&self, household: HouseholdId, c: &NewLlmCall) -> fa_store::Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        LlmCallRepo::insert(&mut tx, c).await?;
        tx.commit().await
    }

    /// Drops the cached endpoint (after the endpoint row or its key changed).
    pub fn invalidate(&self, household: HouseholdId) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&household);
    }
}
