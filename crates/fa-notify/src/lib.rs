//! fa-notify (M2): the notification outbox and the ntfy notifier. See arch/Tasks.md section 5.
mod jobs;
mod send;
mod service;

pub use fa_connectors::ntfy::NtfyMessage;
pub use jobs::{default_schedules, handlers};
pub use service::{NotifyConfig, NotifyService};

use async_trait::async_trait;
use fa_connectors::Http;

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

/// Delivers one message to a topic. Tests inject a fake.
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn publish(&self, topic: &str, msg: &NtfyMessage) -> Result<(), NotifyError>;
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
    async fn publish(&self, topic: &str, msg: &NtfyMessage) -> Result<(), NotifyError> {
        fa_connectors::ntfy::publish(&self.http, &self.url, topic, self.token.as_deref(), msg)
            .await
            .map(|_| ())
            .map_err(|e| NotifyError::Publish(e.to_string()))
    }
}
