#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Invalid(String),
    #[error("conflict")]
    Conflict,
    #[error("the source is read-only")]
    ReadOnly,
    #[error("the source needs to be reconnected")]
    NeedsReauth,
    #[error("connector: {0}")]
    Connector(String),
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
    #[error(transparent)]
    Pipeline(#[from] fa_pipeline::PipelineError),
}

pub type Result<T, E = MailError> = std::result::Result<T, E>;
