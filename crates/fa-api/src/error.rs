use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use fa_auth::AuthError;
use fa_domain::Denied;
use fa_store::StoreError;

/// Every API failure; rendered as `application/problem+json` (RFC 9457).
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("authentication required")]
    Unauthenticated,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("{0}")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    PreconditionRequired(String),
    #[error("{0}")]
    Unprocessable(String),
    #[error("locked until {0}")]
    Locked(DateTime<Utc>),
    #[error("too many attempts, slow down")]
    RateLimited,
    #[error("internal error")]
    Internal(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthenticated | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::PreconditionRequired(_) => StatusCode::PRECONDITION_REQUIRED,
            Self::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Locked(_) => StatusCode::LOCKED,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let detail = match &self {
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
                "internal error".to_string()
            }
            other => other.to_string(),
        };
        let body = serde_json::json!({
            "type": "about:blank",
            "title": status.canonical_reason().unwrap_or("error"),
            "status": status.as_u16(),
            "detail": detail,
        });
        let mut res = (status, body.to_string()).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        res
    }
}

impl From<AuthError> for ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidCredentials => Self::InvalidCredentials,
            AuthError::Locked { until } => Self::Locked(until),
            AuthError::RateLimited => Self::RateLimited,
            AuthError::Unauthenticated => Self::Unauthenticated,
            AuthError::Forbidden(_) => Self::Forbidden("forbidden".into()),
            AuthError::Password(e) => Self::Unprocessable(e.to_string()),
            AuthError::Pin(e) => Self::Unprocessable(e.to_string()),
            AuthError::NotFound => Self::NotFound,
            // Interim mapping; T-09 (M1) turns OAuth errors into /oauth/done redirects.
            AuthError::NotConfigured => Self::NotFound,
            AuthError::NeedsReauth => Self::Conflict("account needs reconnecting".into()),
            AuthError::InvalidState => Self::Unprocessable("invalid or expired OAuth state".into()),
            AuthError::OAuth(m) => Self::Unprocessable(m),
            AuthError::Store(e) => e.into(),
            AuthError::Internal(m) => Self::Internal(m),
        }
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::Conflict => Self::Conflict("the record was changed by someone else".into()),
            StoreError::NotFound => Self::NotFound,
            StoreError::AlreadyExists => Self::Conflict("already exists".into()),
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<Denied> for ApiError {
    fn from(_: Denied) -> Self {
        Self::Forbidden("forbidden".into())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        Self::Unprocessable(r.body_text())
    }
}
impl From<QueryRejection> for ApiError {
    fn from(r: QueryRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}
impl From<PathRejection> for ApiError {
    fn from(r: PathRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}
