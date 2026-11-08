//! The recipe draft: what an import (JSON-LD, microdata or the LLM) hands to the editor.
//! Moved here from `fa-recipe` so `fa-domain`, `fa-meals` and the API share one type (M5, T-02).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, utoipa::ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
#[serde(rename_all = "lowercase")]
pub enum RecipeOrigin {
    Jsonld,
    Microdata,
    Llm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, utoipa::ToSchema)]
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
