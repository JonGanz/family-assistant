use std::time::Duration;

use bytes::{Bytes, BytesMut};
use reqwest::{Method, RequestBuilder, Response, StatusCode, redirect};
use url::Url;

use crate::error::{ConnectorError, Result};

pub const MAX_BODY: usize = 10 * 1024 * 1024;

/// Shared outbound client: rustls, 20 s timeout, gzip, capped body, https-only unless allowed.
#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
    allow_http: bool,
    max_body: usize,
}

impl Http {
    pub fn new(allow_http: bool) -> Result<Self> {
        Self::with_max_body(allow_http, MAX_BODY)
    }

    pub fn with_max_body(allow_http: bool, max_body: usize) -> Result<Self> {
        let policy = redirect::Policy::custom(move |attempt| {
            let ok = attempt.url().scheme() == "https" || allow_http;
            if attempt.previous().len() >= 5 || !ok {
                attempt.stop()
            } else {
                attempt.follow()
            }
        });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .gzip(true)
            .redirect(policy)
            .user_agent(concat!("family-assistant/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ConnectorError::Network(e.without_url().to_string()))?;
        Ok(Self {
            client,
            allow_http,
            max_body,
        })
    }

    pub fn parse_url(&self, url: &str) -> Result<Url> {
        let url = Url::parse(url).map_err(|e| ConnectorError::Parse(format!("bad url: {e}")))?;
        self.check_url(&url)?;
        Ok(url)
    }

    pub fn check_url(&self, url: &Url) -> Result<()> {
        match url.scheme() {
            "https" => Ok(()),
            "http" if self.allow_http => Ok(()),
            "http" => Err(ConnectorError::InsecureUrl),
            other => Err(ConnectorError::Parse(format!("unsupported scheme {other}"))),
        }
    }

    pub fn request(&self, method: Method, url: &Url) -> Result<RequestBuilder> {
        self.check_url(url)?;
        Ok(self.client.request(method, url.clone()))
    }

    pub fn get(&self, url: &Url) -> Result<RequestBuilder> {
        self.request(Method::GET, url)
    }

    /// Sends the request. Errors never carry the URL (it may hold secrets); only the host is logged.
    pub async fn send(&self, rb: RequestBuilder) -> Result<Response> {
        let resp = rb.send().await.map_err(|e| {
            let e = e.without_url();
            tracing::warn!(error = %e, "outbound request failed");
            ConnectorError::Network(e.to_string())
        })?;
        tracing::debug!(
            host = resp.url().host_str().unwrap_or(""),
            status = resp.status().as_u16(),
            "outbound response"
        );
        Ok(resp)
    }

    /// Maps non-success statuses to errors.
    pub fn check_status(status: StatusCode) -> Result<()> {
        match status.as_u16() {
            401 => Err(ConnectorError::Unauthorized),
            410 => Err(ConnectorError::Gone),
            304 => Err(ConnectorError::NotModified),
            _ if status.is_success() => Ok(()),
            s => Err(ConnectorError::Http(s)),
        }
    }

    /// Reads the body while enforcing the size cap.
    pub async fn body(&self, mut resp: Response) -> Result<Bytes> {
        if resp
            .content_length()
            .is_some_and(|l| l > self.max_body as u64)
        {
            return Err(ConnectorError::TooLarge);
        }
        let mut buf = BytesMut::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| ConnectorError::Network(e.without_url().to_string()))?
        {
            if buf.len() + chunk.len() > self.max_body {
                return Err(ConnectorError::TooLarge);
            }
            buf.extend_from_slice(&chunk);
        }
        Ok(buf.freeze())
    }

    /// Status check, then capped body.
    pub async fn ok_body(&self, resp: Response) -> Result<Bytes> {
        Self::check_status(resp.status())?;
        self.body(resp).await
    }

    pub async fn get_json(&self, rb: RequestBuilder) -> Result<serde_json::Value> {
        let resp = self.send(rb).await?;
        let bytes = self.ok_body(resp).await?;
        serde_json::from_slice(&bytes).map_err(|e| ConnectorError::Parse(e.to_string()))
    }
}
