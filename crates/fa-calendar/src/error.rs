use fa_connectors::ConnectorError;
use fa_domain::TokenError;
use fa_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum CalendarError {
    #[error(transparent)]
    Store(#[from] StoreError),
    /// The remote rejected the credentials (HTTP 401, `invalid_grant`, or a revoked Google account).
    #[error("the credentials were rejected; re-connect the source")]
    Unauthorized,
    /// Message already scrubbed of URLs and credentials.
    #[error("{0}")]
    Remote(String),
    #[error("not found")]
    NotFound,
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("the requested range is longer than {0} days")]
    RangeTooLarge(i64),
}

impl From<ConnectorError> for CalendarError {
    fn from(e: ConnectorError) -> Self {
        match e {
            ConnectorError::Unauthorized => Self::Unauthorized,
            ConnectorError::InsecureUrl => Self::Invalid(e.to_string()),
            other => Self::Remote(scrub(&other.to_string(), &[])),
        }
    }
}

impl From<TokenError> for CalendarError {
    fn from(e: TokenError) -> Self {
        match e {
            TokenError::NeedsReauth => Self::Unauthorized,
            TokenError::Other(m) => Self::Remote(scrub(&m, &[])),
        }
    }
}

pub type Result<T, E = CalendarError> = std::result::Result<T, E>;

/// Removes URLs and the given secrets from a message that may be stored or shown.
pub fn scrub(msg: &str, secrets: &[&str]) -> String {
    let mut s = msg.to_owned();
    for sec in secrets.iter().filter(|x| x.len() >= 4) {
        s = s.replace(sec, "***");
    }
    let mut out = String::with_capacity(s.len());
    for (i, tok) in s.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if tok.contains("://") {
            out.push_str("<url>");
        } else {
            out.push_str(tok);
        }
    }
    out.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_urls_and_secrets() {
        let m = scrub(
            "GET https://example.com/cal/abc123 failed for hunter2 (status)",
            &["hunter2"],
        );
        assert_eq!(m, "GET <url> failed for *** (status)");
    }
}
