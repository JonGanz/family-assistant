use bytes::Bytes;
use reqwest::Response;

use crate::{ConnectorError, Http, error::Result};

pub mod calendar;
pub mod gmail;
pub mod oauth;

pub const DEFAULT_API_BASE: &str = "https://www.googleapis.com";

/// Status mapping shared by the Gmail and Calendar clients. 401 → `Unauthorized`, 403 for a
/// missing scope → `InsufficientScope`, 429/5xx and rate-limit 403s → `Retryable`.
pub(crate) async fn ok_body(http: &Http, resp: Response) -> Result<Bytes> {
    let status = resp.status().as_u16();
    if resp.status().is_success() {
        return http.body(resp).await;
    }
    match status {
        401 => Err(ConnectorError::Unauthorized),
        429 | 500..=599 => Err(ConnectorError::Retryable(status)),
        403 => {
            let body = http.body(resp).await.unwrap_or_default();
            let text = String::from_utf8_lossy(&body).to_ascii_lowercase();
            if text.contains("ratelimit") {
                Err(ConnectorError::Retryable(403))
            } else if text.contains("insufficient") || text.contains("scope") {
                Err(ConnectorError::InsufficientScope)
            } else {
                Err(ConnectorError::Http(403))
            }
        }
        s => Err(ConnectorError::Http(s)),
    }
}

pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| ConnectorError::Parse(e.to_string()))
}

pub(crate) fn api_url(base: &str, segments: &[&str]) -> Result<url::Url> {
    let mut url = url::Url::parse(base).map_err(|e| ConnectorError::Parse(e.to_string()))?;
    url.path_segments_mut()
        .map_err(|_| ConnectorError::Parse("bad base url".into()))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}
