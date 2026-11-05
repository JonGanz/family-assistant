use fa_llm::LlmError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS, utoipa::ToSchema,
)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
#[serde(rename_all = "lowercase")]
pub enum RecipeOrigin {
    Jsonld,
    Llm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct RecipeIngredient {
    /// The source line, always present.
    pub text: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub quantity: Option<f64>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub unit: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub item: Option<String>,
}

/// A recipe extracted from a page. Minutes are integers; `source_url` is an absolute URL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct RecipeDraft {
    pub name: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub source_url: Option<String>,
    pub origin: RecipeOrigin,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub yield_text: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub servings: Option<u32>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub prep_minutes: Option<u32>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub cook_minutes: Option<u32>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub total_minutes: Option<u32>,
    pub ingredients: Vec<RecipeIngredient>,
    pub steps: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub image_url: Option<String>,
}

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
