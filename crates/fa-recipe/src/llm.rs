use fa_llm::{ExtractRequest, LlmClient, LlmError};
use schemars::JsonSchema;
use serde::Deserialize;
use url::Url;

use crate::{RecipeDraft, RecipeOrigin, parse_ingredient};

/// Schema-friendly subset: every field required (strict mode); empty or 0 means absent.
#[derive(Debug, Deserialize, JsonSchema)]
struct LlmRecipe {
    name: String,
    yield_text: String,
    servings: u32,
    prep_minutes: u32,
    cook_minutes: u32,
    total_minutes: u32,
    ingredients: Vec<String>,
    steps: Vec<String>,
    tags: Vec<String>,
}

const SYSTEM: &str = "Extract one recipe from the page text. Return only what is on the page. \
Use empty strings, 0 and empty arrays for anything that is absent. Ingredients are one line each, \
copied as written.";

/// Max page characters sent to the model.
const MAX_CHARS: usize = 24_000;

pub async fn from_llm(
    client: &LlmClient,
    text: &str,
    source_url: Option<&Url>,
) -> Result<RecipeDraft, LlmError> {
    let user: String = text.chars().take(MAX_CHARS).collect();
    let req = ExtractRequest {
        system: SYSTEM.into(),
        user,
        schema_name: "recipe",
        max_tokens: 2048,
    };
    let r = client.extract::<LlmRecipe>(&req).await?.value;
    let name = r.name.trim().to_string();
    let ingredients: Vec<_> = r
        .ingredients
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(parse_ingredient)
        .collect();
    if name.is_empty() || ingredients.is_empty() {
        return Err(LlmError::Schema("no recipe".into()));
    }
    let opt_s = |s: String| Some(s.trim().to_string()).filter(|s| !s.is_empty());
    let opt_n = |n: u32| Some(n).filter(|n| *n > 0);
    let clean = |v: Vec<String>| -> Vec<String> {
        v.into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    Ok(RecipeDraft {
        name,
        source_url: source_url.map(|u| u.to_string()),
        origin: RecipeOrigin::Llm,
        yield_text: opt_s(r.yield_text),
        servings: opt_n(r.servings),
        prep_minutes: opt_n(r.prep_minutes),
        cook_minutes: opt_n(r.cook_minutes),
        total_minutes: opt_n(r.total_minutes),
        ingredients,
        steps: clean(r.steps),
        tags: crate::jsonld::dedup(clean(r.tags)),
        image_url: None,
    })
}
