use chrono::{DateTime, Utc};
use fa_domain::{Denied, PasswordError, PinError};
use fa_store::StoreError;

pub type Result<T, E = AuthError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account or PIN locked until {until}")]
    Locked { until: DateTime<Utc> },
    #[error("too many attempts, slow down")]
    RateLimited,
    #[error("not authenticated")]
    Unauthenticated,
    #[error("forbidden")]
    Forbidden(#[from] Denied),
    #[error(transparent)]
    Password(#[from] PasswordError),
    #[error(transparent)]
    Pin(#[from] PinError),
    #[error("not found")]
    NotFound,
    #[error("storage error: {0}")]
    Store(#[from] StoreError),
    #[error("internal error: {0}")]
    Internal(String),
}
