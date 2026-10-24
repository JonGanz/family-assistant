//! Access-token provider seam: fa-auth implements it, fa-calendar consumes it.

use async_trait::async_trait;

use crate::ids::{HouseholdId, OAuthAccountId};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("the account needs to be re-connected")]
    NeedsReauth,
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait AccessTokenProvider: Send + Sync {
    /// A currently valid access token (refreshing it when needed).
    async fn access_token(
        &self,
        household: HouseholdId,
        account: OAuthAccountId,
    ) -> Result<String, TokenError>;
}
