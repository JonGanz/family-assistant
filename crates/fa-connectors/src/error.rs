#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectorError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("gone")]
    Gone,
    #[error("not modified")]
    NotModified,
    #[error("response body too large")]
    TooLarge,
    #[error("http status {0}")]
    Http(u16),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("insecure url (http not allowed)")]
    InsecureUrl,
    /// Gmail `history.list` 404: the start history id is too old.
    #[error("history id expired")]
    HistoryExpired,
    /// 403 because the access token lacks a required scope.
    #[error("insufficient scope")]
    InsufficientScope,
    /// 429 or 5xx (or a rate-limit 403): retry with backoff.
    #[error("retryable http status {0}")]
    Retryable(u16),
    /// IMAP login refused (wrong credentials or revoked app password).
    #[error("authentication failed")]
    AuthFailed,
    #[error("imap error: {0}")]
    Imap(String),
}

pub type Result<T> = std::result::Result<T, ConnectorError>;
