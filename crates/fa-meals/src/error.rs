//! The one error type of the crate. `Store` errors that have a meaning for the caller are mapped
//! to their own variants, so the API layer only matches on this enum.

use fa_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum MealsError {
    #[error(transparent)]
    Store(StoreError),
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("{field}: {message}")]
    Invalid {
        field: &'static str,
        message: String,
    },
    #[error("a slot to remove still has plan entries or template cells")]
    SlotInUse,
    #[error("no recipe was found")]
    NoRecipe,
    #[error("fetch failed: {0}")]
    FetchFailed(String),
    #[error("the llm is not available")]
    LlmUnavailable,
    #[error("llm failed: {0}")]
    LlmFailed(String),
    #[error("the undo window has passed")]
    Expired,
    #[error("already undone")]
    AlreadyUndone,
    #[error("image: {0}")]
    Image(String),
}

pub type Result<T> = std::result::Result<T, MealsError>;

impl MealsError {
    pub fn invalid(field: &'static str, message: impl Into<String>) -> Self {
        Self::Invalid {
            field,
            message: message.into(),
        }
    }
}

impl From<StoreError> for MealsError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound => Self::NotFound,
            StoreError::Conflict => Self::Conflict("the record was changed by someone else".into()),
            StoreError::AlreadyExists => Self::Conflict("already exists".into()),
            other => Self::Store(other),
        }
    }
}

impl From<sqlx::Error> for MealsError {
    fn from(e: sqlx::Error) -> Self {
        StoreError::from(e).into()
    }
}

impl From<MealsError> for fa_jobs::JobError {
    fn from(e: MealsError) -> Self {
        match e {
            MealsError::Store(s) => fa_jobs::JobError::Store(s),
            other => fa_jobs::JobError::Handler(other.to_string()),
        }
    }
}
