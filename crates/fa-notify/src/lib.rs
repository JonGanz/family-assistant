//! fa-notify (M2): the notification outbox and the ntfy notifier. See arch/Tasks.md section 5.
mod jobs;
mod send;
mod service;

pub use fa_connectors::ntfy::NtfyMessage;
pub use jobs::{default_schedules, handlers};
pub use service::{NotifyConfig, NotifyService};

use async_trait::async_trait;
use fa_connectors::{ConnectorError, Http};

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("ntfy is not configured")]
    NotConfigured,
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("{0}")]
    Invalid(String),
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
}

/// Why a publish failed, as far as retrying is concerned.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("ntfy is not configured")]
    NotConfigured,
    /// The server refused the message (4xx: bad token or topic, bad request). Retrying the same
    /// message will not help, so it counts against the row's attempts.
    #[error("publish failed: {0}")]
    Rejected(String),
    /// A transport error or a 5xx: the server may be back later.
    #[error("publish failed: {0}")]
    Transient(String),
}

impl From<PublishError> for NotifyError {
    fn from(e: PublishError) -> Self {
        match e {
            PublishError::NotConfigured => Self::NotConfigured,
            PublishError::Rejected(m) | PublishError::Transient(m) => Self::Publish(m),
        }
    }
}

/// Delivers one message to a topic. Tests inject a fake.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn publish(&self, topic: &str, msg: &NtfyMessage) -> Result<(), PublishError>;
}

/// The real ntfy client. Plain `http` is allowed regardless of `FA_ALLOW_HTTP_FEEDS`.
pub struct NtfyNotifier {
    http: Http,
    url: String,
    token: Option<String>,
}

impl NtfyNotifier {
    pub fn new(url: String, token: Option<String>) -> Result<Self, NotifyError> {
        let http = Http::new(true).map_err(|e| NotifyError::Publish(e.to_string()))?;
        Ok(Self { http, url, token })
    }
}

#[async_trait]
impl Notifier for NtfyNotifier {
    async fn publish(&self, topic: &str, msg: &NtfyMessage) -> Result<(), PublishError> {
        fa_connectors::ntfy::publish(&self.http, &self.url, topic, self.token.as_deref(), msg)
            .await
            .map(|_| ())
            .map_err(|e| {
                let text = e.to_string();
                match e {
                    // 408 and 429 ask for a later retry; the other 4xx are final.
                    ConnectorError::Http(408 | 429) => PublishError::Transient(text),
                    ConnectorError::Http(s) if (400..500).contains(&s) => {
                        PublishError::Rejected(text)
                    }
                    ConnectorError::Unauthorized | ConnectorError::Parse(_) => {
                        PublishError::Rejected(text)
                    }
                    _ => PublishError::Transient(text),
                }
            })
    }
}
