pub type Result<T, E = StoreError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Optimistic concurrency check failed (stale `version`).
    #[error("conflict: the record was changed by someone else")]
    Conflict,
    #[error("not found")]
    NotFound,
    /// A unique constraint was violated.
    #[error("already exists")]
    AlreadyExists,
    #[error("database error: {0}")]
    Db(sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("crypto error: {0}")]
    Crypto(&'static str),
    #[error("master key error: {0}")]
    Key(String),
    #[error("corrupt row: {0}")]
    Corrupt(String),
}

impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => StoreError::NotFound,
            sqlx::Error::Database(d) if d.is_unique_violation() => StoreError::AlreadyExists,
            _ => StoreError::Db(e),
        }
    }
}
