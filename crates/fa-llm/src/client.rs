use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::schema::json_schema_for;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    OpenaiChat,
    OllamaNative,
}

impl Dialect {
    fn as_str(self) -> &'static str {
        match self {
            Dialect::OpenaiChat => "openai_chat",
            Dialect::OllamaNative => "ollama_native",
        }
    }
}

#[derive(Clone)]
pub struct EndpointConfig {
    pub base_url: url::Url,
    pub model: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub dialect: Dialect,
}

impl std::fmt::Debug for EndpointConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointConfig")
            .field("host", &self.base_url.host_str())
            .field("model", &self.model)
            .field("dialect", &self.dialect)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub system: String,
    pub user: String,
    pub schema_name: &'static str,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct Extracted<T> {
    pub value: T,
    pub latency: Duration,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Ping {
    pub latency: Duration,
    pub model_found: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("llm endpoint unavailable: {0}")]
    Unavailable(String),
    #[error("llm request timed out")]
    Timeout,
    #[error("llm endpoint returned HTTP {0}")]
    Http(u16),
    #[error("llm returned invalid JSON: {0}")]
    BadJson(String),
    #[error("llm JSON does not match the schema: {0}")]
    Schema(String),
    #[error("llm endpoint config: {0}")]
    Config(String),
}

impl LlmError {
    /// The `llm_call.error_kind` values.
    pub fn kind(&self) -> &'static str {
        match self {
            LlmError::Unavailable(_) => "unavailable",
            LlmError::Timeout => "timeout",
            LlmError::Http(_) => "http",
            LlmError::BadJson(_) => "bad_json",
            LlmError::Schema(_) => "schema",
            LlmError::Config(_) => "config",
        }
    }
}

/// Redirects off (the endpoint is set by an adult; private hosts are allowed), no global timeout.
pub fn default_http() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

#[derive(Clone)]
pub struct LlmClient {
    cfg: EndpointConfig,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(cfg: EndpointConfig, http: reqwest::Client) -> Self {
        Self { cfg, http }
    }

    fn validate(&self) -> Result<(), LlmError> {
        if self.cfg.model.trim().is_empty() {
            return Err(LlmError::Config("model is empty".into()));
        }
        match self.cfg.base_url.scheme() {
            "http" | "https" if self.cfg.base_url.host_str().is_some() => Ok(()),
            _ => Err(LlmError::Config("base URL must be http or https".into())),
        }
    }

    fn url(&self, path: &str) -> String {
        let mut u = self.cfg.base_url.clone();
        u.set_query(None);
        u.set_fragment(None);
        let _ = u.set_username("");
        let _ = u.set_password(None);
        format!(
            "{}/{}",
            u.as_str().trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let rb = rb.timeout(self.cfg.timeout);
        match self.cfg.api_key.as_deref().filter(|k| !k.is_empty()) {
            Some(k) => rb.bearer_auth(k),
            None => rb,
        }
    }

    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<Value, LlmError> {
        let resp = self.auth(rb).send().await.map_err(map_err)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(LlmError::Http(status.as_u16()));
        }
        let bytes = resp.bytes().await.map_err(map_err)?;
        serde_json::from_slice(&bytes).map_err(|e| LlmError::BadJson(e.to_string()))
    }

    pub async fn extract<T: DeserializeOwned + JsonSchema>(
        &self,
        req: &ExtractRequest,
    ) -> Result<Extracted<T>, LlmError> {
        let start = Instant::now();
        let res = self.extract_inner::<T>(req, start).await;
        match &res {
            Ok(e) => tracing::info!(dialect = self.cfg.dialect.as_str(), model = %self.cfg.model,
                latency_ms = e.latency.as_millis() as u64, "llm extract ok"),
            Err(e) => tracing::warn!(dialect = self.cfg.dialect.as_str(), model = %self.cfg.model,
                latency_ms = start.elapsed().as_millis() as u64, error_kind = e.kind(), "llm extract failed"),
        }
        res
    }

    async fn extract_inner<T: DeserializeOwned + JsonSchema>(
        &self,
        req: &ExtractRequest,
        start: Instant,
    ) -> Result<Extracted<T>, LlmError> {
        self.validate()?;
        let schema = json_schema_for::<T>();
        let messages = json!([
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user},
        ]);
        let (rb, content_ptr, pt_ptr, ct_ptr) = match self.cfg.dialect {
            Dialect::OpenaiChat => {
                let body = json!({
                    "model": self.cfg.model,
                    "messages": messages,
                    "response_format": {"type": "json_schema",
                        "json_schema": {"name": req.schema_name, "schema": schema, "strict": true}},
                    "max_tokens": req.max_tokens,
                    "temperature": 0,
                    "stream": false,
                });
                (
                    self.http.post(self.url("chat/completions")).json(&body),
                    "/choices/0/message/content",
                    "/usage/prompt_tokens",
                    "/usage/completion_tokens",
                )
            }
            Dialect::OllamaNative => {
                let body = json!({
                    "model": self.cfg.model,
                    "messages": messages,
                    "format": schema,
                    "stream": false,
                    "options": {"temperature": 0, "num_predict": req.max_tokens},
                });
                (
                    self.http.post(self.url("api/chat")).json(&body),
                    "/message/content",
                    "/prompt_eval_count",
                    "/eval_count",
                )
            }
        };
        let v = self.send(rb).await?;
        let content = v
            .pointer(content_ptr)
            .and_then(Value::as_str)
            .ok_or_else(|| LlmError::BadJson("response has no message content".into()))?;
        let raw: Value =
            serde_json::from_str(content).map_err(|e| LlmError::BadJson(e.to_string()))?;
        let value: T = serde_json::from_value(raw).map_err(|e| LlmError::Schema(e.to_string()))?;
        let tok = |p: &str| {
            v.pointer(p)
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
        };
        Ok(Extracted {
            value,
            latency: start.elapsed(),
            prompt_tokens: tok(pt_ptr),
            completion_tokens: tok(ct_ptr),
        })
    }

    pub async fn models(&self) -> Result<Vec<String>, LlmError> {
        self.validate_url()?;
        let (path, list, key) = match self.cfg.dialect {
            Dialect::OpenaiChat => ("models", "/data", "id"),
            Dialect::OllamaNative => ("api/tags", "/models", "name"),
        };
        let v = self.send(self.http.get(self.url(path))).await?;
        Ok(v.pointer(list)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|m| m.get(key)?.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn validate_url(&self) -> Result<(), LlmError> {
        match self.cfg.base_url.scheme() {
            "http" | "https" if self.cfg.base_url.host_str().is_some() => Ok(()),
            _ => Err(LlmError::Config("base URL must be http or https".into())),
        }
    }

    pub async fn ping(&self) -> Result<Ping, LlmError> {
        self.validate()?;
        let start = Instant::now();
        let models = self.models().await;
        let latency = start.elapsed();
        match models {
            Ok(m) => {
                let want = self.cfg.model.as_str();
                let latest = format!("{want}:latest");
                let model_found = m
                    .iter()
                    .any(|x| x == want || (!want.contains(':') && *x == latest));
                tracing::info!(
                    dialect = self.cfg.dialect.as_str(),
                    model = want,
                    latency_ms = latency.as_millis() as u64,
                    "llm ping ok"
                );
                Ok(Ping {
                    latency,
                    model_found,
                })
            }
            Err(e) => {
                tracing::warn!(
                    dialect = self.cfg.dialect.as_str(),
                    error_kind = e.kind(),
                    "llm ping failed"
                );
                Err(e)
            }
        }
    }
}

fn map_err(e: reqwest::Error) -> LlmError {
    let e = e.without_url();
    if e.is_timeout() {
        LlmError::Timeout
    } else if e.is_connect() {
        LlmError::Unavailable("connection failed".into())
    } else if e.is_decode() || e.is_body() {
        LlmError::BadJson("unreadable response body".into())
    } else {
        LlmError::Unavailable("request failed".into())
    }
}
