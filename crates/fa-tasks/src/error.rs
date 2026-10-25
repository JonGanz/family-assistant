use fa_store::StoreError;

pub type Result<T, E = TaskError> = std::result::Result<T, E>;

/// fa-api maps these: `NotFound` 404, `Conflict`/`WrongStatus`/`ClaimedByOther` 409,
/// `Invalid`/`NotCandidate`/`NotPool` 422.
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("not found")]
    NotFound,
    #[error("the version is stale")]
    Conflict,
    #[error("the task is {actual}, expected {expected}")]
    WrongStatus {
        expected: &'static str,
        actual: String,
    },
    #[error("claimed by someone else")]
    ClaimedByOther,
    #[error("only a candidate can claim this task")]
    NotCandidate,
    #[error("only pool tasks can be claimed")]
    NotPool,
    #[error("{field}: {message}")]
    Invalid {
        field: String,
        position: Option<i32>,
        message: String,
    },
    #[error(transparent)]
    Store(StoreError),
}

impl From<StoreError> for TaskError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound => Self::NotFound,
            StoreError::Conflict => Self::Conflict,
            other => Self::Store(other),
        }
    }
}

impl From<sqlx::Error> for TaskError {
    fn from(e: sqlx::Error) -> Self {
        Self::Store(e.into())
    }
}

impl TaskError {
    pub fn invalid(field: &str, message: impl Into<String>) -> Self {
        Self::Invalid {
            field: field.into(),
            position: None,
            message: message.into(),
        }
    }
}
