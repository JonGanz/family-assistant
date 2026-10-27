use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use fa_auth::AuthError;
use fa_calendar::CalendarError;
use fa_domain::Denied;
use fa_store::StoreError;
use fa_weather::WeatherError;
use utoipa::ToSchema;

/// The `application/problem+json` body (RFC 9457). Invalid weather rules add `position` and `message`.
#[derive(serde::Serialize, ToSchema)]
pub struct Problem {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    /// Weather-rule errors only: char offset into the condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// Weather-rule errors only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Task and routine validation errors: the offending input field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// Machine-readable code of the setup and system routes (`setup_required`, `setup_closed`,
    /// `setting_locked`, `invalid_setting`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

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
    Gone(String),
    #[error("{0}")]
    PreconditionRequired(String),
    #[error("{0}")]
    Unprocessable(String),
    #[error("{0}")]
    PayloadTooLarge(String),
    #[error("{0}")]
    UnsupportedMedia(String),
    /// An invalid weather-rule condition: 422 with `{position, message}`.
    #[error("{message}")]
    RuleInvalid { position: u32, message: String },
    /// A field-level validation error: 422 with `{field, position?, message}`.
    #[error("{message}")]
    FieldInvalid {
        field: String,
        position: Option<u32>,
        message: String,
    },
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    BadGateway(String),
    #[error("{0}")]
    GatewayTimeout(String),
    /// A conflict or validation failure with a stable `code`; `field` names the setting key.
    #[error("{message}")]
    Coded {
        status: StatusCode,
        code: &'static str,
        field: Option<String>,
        message: String,
    },
    #[error("locked until {0}")]
    Locked(DateTime<Utc>),
    #[error("too many attempts, slow down")]
    RateLimited,
    #[error("internal error")]
    Internal(String),
}

impl ApiError {
    pub(crate) fn coded(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self::Coded {
            status,
            code,
            field: None,
            message: message.into(),
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthenticated | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Gone(_) => StatusCode::GONE,
            Self::PreconditionRequired(_) => StatusCode::PRECONDITION_REQUIRED,
            Self::Unprocessable(_) | Self::RuleInvalid { .. } | Self::FieldInvalid { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UnsupportedMedia(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::GatewayTimeout(_) => StatusCode::GATEWAY_TIMEOUT,
            Self::Coded { status, .. } => *status,
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
        let code = match &self {
            Self::Coded { code, .. } => Some((*code).to_string()),
            _ => None,
        };
        let (position, message, field) = match &self {
            Self::Coded { field, .. } => (None, None, field.clone()),
            Self::RuleInvalid { position, message } => {
                (Some(*position), Some(message.clone()), None)
            }
            Self::FieldInvalid {
                field,
                position,
                message,
            } => (*position, Some(message.clone()), Some(field.clone())),
            _ => (None, None, None),
        };
        let body = Problem {
            r#type: "about:blank".into(),
            title: status.canonical_reason().unwrap_or("error").into(),
            status: status.as_u16(),
            detail,
            position,
            message,
            field,
            code,
        };
        let json = serde_json::to_string(&body).unwrap_or_default();
        let mut res = (status, json).into_response();
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
            AuthError::SetupClosed => Self::coded(
                StatusCode::CONFLICT,
                "setup_closed",
                "setup has already been done",
            ),
            AuthError::Invalid(m) => Self::Unprocessable(m),
            AuthError::Password(e) => Self::Unprocessable(e.to_string()),
            AuthError::Pin(e) => Self::Unprocessable(e.to_string()),
            AuthError::NotFound => Self::NotFound,
            // The browser OAuth routes turn InvalidState/OAuth into /oauth/done redirects before
            // this mapping is reached; it only serves callers outside those routes.
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

impl From<CalendarError> for ApiError {
    fn from(e: CalendarError) -> Self {
        match e {
            CalendarError::Store(s) => s.into(),
            CalendarError::Unauthorized => {
                Self::Unprocessable("the credentials were rejected; check them or reconnect".into())
            }
            CalendarError::Remote(m) => Self::BadGateway(m),
            CalendarError::NotFound => Self::NotFound,
            CalendarError::Invalid(m) => Self::Unprocessable(m),
            CalendarError::State(m) => Self::Conflict(m),
            e @ CalendarError::RangeTooLarge(_) => Self::BadRequest(e.to_string()),
        }
    }
}

impl From<WeatherError> for ApiError {
    fn from(e: WeatherError) -> Self {
        match e {
            WeatherError::Store(s) => s.into(),
            WeatherError::Connector(_) => Self::BadGateway("the weather service failed".into()),
            WeatherError::Job(m) => Self::Internal(m),
        }
    }
}

impl From<fa_tasks::TaskError> for ApiError {
    fn from(e: fa_tasks::TaskError) -> Self {
        use fa_tasks::TaskError as T;
        match e {
            T::NotFound => Self::NotFound,
            T::Conflict => Self::Conflict("the record was changed by someone else".into()),
            e @ (T::WrongStatus { .. } | T::ClaimedByOther) => Self::Conflict(e.to_string()),
            e @ (T::NotCandidate | T::NotPool) => Self::Unprocessable(e.to_string()),
            T::Invalid {
                field,
                position,
                message,
            } => Self::FieldInvalid {
                field,
                position: position.and_then(|p| u32::try_from(p).ok()),
                message,
            },
            T::Store(s) => s.into(),
        }
    }
}

impl From<fa_notify::NotifyError> for ApiError {
    fn from(e: fa_notify::NotifyError) -> Self {
        use fa_notify::NotifyError as N;
        match e {
            N::NotConfigured => Self::Unavailable("ntfy is not configured on this server".into()),
            N::Publish(m) => Self::BadGateway(m),
            N::Invalid(m) => Self::Unprocessable(m),
            N::NotFound => Self::NotFound,
            N::Store(s) => s.into(),
        }
    }
}

impl From<fa_mail::MailError> for ApiError {
    fn from(e: fa_mail::MailError) -> Self {
        use fa_mail::MailError as M;
        match e {
            M::NotFound => Self::NotFound,
            M::Forbidden => Self::Forbidden("forbidden".into()),
            M::Invalid(m) => Self::Unprocessable(m),
            M::Conflict => Self::Conflict("the record was changed by someone else".into()),
            e @ (M::ReadOnly | M::NeedsReauth) => Self::Conflict(e.to_string()),
            M::Connector(m) => Self::BadGateway(m),
            M::Store(s) => s.into(),
            M::Pipeline(p) => Self::Internal(p.to_string()),
        }
    }
}
