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
}

pub type Result<T> = std::result::Result<T, ConnectorError>;
