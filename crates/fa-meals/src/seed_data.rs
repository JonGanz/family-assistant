//! The hand-written default catalogue every household starts with (D-114). Data in `seed/`.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SeedAisle {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SeedIngredient {
    pub canonical_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub aisle: String,
    #[serde(default)]
    pub default_unit: Option<String>,
    #[serde(default)]
    pub staple: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum SingularRule {
    Pair { plural: String, singular: String },
    Keep { keep: String },
}

const AISLES: &str = include_str!("../seed/aisles.json");
const INGREDIENTS: &str = include_str!("../seed/ingredients.json");
const DESCRIPTORS: &str = include_str!("../seed/descriptors.json");
const SINGULAR: &str = include_str!("../seed/singular_exceptions.json");

fn load<T: serde::de::DeserializeOwned>(name: &str, raw: &str) -> Vec<T> {
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::error!(file = name, error = %e, "seed file does not parse");
        Vec::new()
    })
}

/// Aisles in walking order. "Other" is not listed: a null aisle is "Other".
pub fn aisles() -> Vec<SeedAisle> {
    load("aisles.json", AISLES)
}

pub fn ingredients() -> Vec<SeedIngredient> {
    load("ingredients.json", INGREDIENTS)
}

pub fn descriptors() -> Vec<String> {
    load("descriptors.json", DESCRIPTORS)
}

pub fn singular_rules() -> Vec<SingularRule> {
    load("singular_exceptions.json", SINGULAR)
}

/// The raw files, so tests can fail loudly on a parse error instead of an empty list.
pub mod raw {
    pub const AISLES: &str = super::AISLES;
    pub const INGREDIENTS: &str = super::INGREDIENTS;
    pub const DESCRIPTORS: &str = super::DESCRIPTORS;
    pub const SINGULAR: &str = super::SINGULAR;
}
