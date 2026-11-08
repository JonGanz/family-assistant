pub use fa_domain::recipe::{RecipeDraft, RecipeIngredient, RecipeOrigin};
use fa_llm::LlmError;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    #[error("fetch failed: {0}")]
    Fetch(String),
    #[error("page is not html")]
    NotHtml,
    #[error("no recipe found")]
    NoRecipe,
    #[error("no recipe in the page and the llm is not available")]
    LlmUnavailable,
    #[error(transparent)]
    Llm(#[from] LlmError),
}

pub enum RecipeInput {
    Url(Url),
    Html {
        html: String,
        source_url: Option<Url>,
    },
}
