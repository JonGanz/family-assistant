use fa_llm::{ExtractRequest, LlmClient, LlmError};
use schemars::JsonSchema;
use serde::Deserialize;
use url::Url;

use crate::sections::{self, IngGroup};
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
    ingredients: Vec<LlmItem>,
    steps: Vec<LlmItem>,
    tags: Vec<String>,
}

fn nullable_string(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": ["string", "null"]})
}

/// An ingredient line or a step; `section` is the heading it sits under on the page, or null.
#[derive(Debug, Deserialize, JsonSchema)]
struct LlmItem {
    text: String,
    #[schemars(schema_with = "nullable_string")]
    section: Option<String>,
}

const SYSTEM: &str = "Extract one recipe from the page text. Return only what is on the page. \
Use empty strings, 0 and empty arrays for anything that is absent. Ingredients are one line each, \
copied as written. Give each ingredient and step the section heading it appears under \
(for example \"Sauce\"), or null when it has none.";

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
        max_tokens: 4096,
    };
    let r = client.extract::<LlmRecipe>(&req).await?.value;
    let name = r.name.trim().to_string();
    let sec = |i: &LlmItem| {
        i.section
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    // Ingredients: explicit section, else the last heading line, else shared.
    let mut ingredients = Vec::new();
    let mut groups: Vec<IngGroup> = Vec::new();
    let mut heading: Option<String> = None;
    for it in &r.ingredients {
        let line = it.text.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(h) = sections::heading_name(line) {
            heading = Some(h);
            continue;
        }
        let ing = parse_ingredient(line);
        match sec(it).or_else(|| heading.clone()) {
            None => ingredients.push(ing),
            Some(n) => match groups
                .iter_mut()
                .find(|g| sections::norm(&g.name) == sections::norm(&n))
            {
                Some(g) => g.items.push(ing),
                None => groups.push(IngGroup {
                    name: n,
                    items: vec![ing],
                }),
            },
        }
    }
    let mut st = Vec::new();
    let mut step_secs: Vec<(String, Vec<String>)> = Vec::new();
    for it in &r.steps {
        let t = it.text.trim().to_string();
        if t.is_empty() {
            continue;
        }
        match sec(it) {
            None => st.push(t),
            Some(n) => match step_secs
                .iter_mut()
                .find(|(m, _)| sections::norm(m) == sections::norm(&n))
            {
                Some((_, v)) => v.push(t),
                None => step_secs.push((n, vec![t])),
            },
        }
    }
    let total = ingredients.len() + groups.iter().map(|g| g.items.len()).sum::<usize>();
    if name.is_empty() || total == 0 {
        return Err(LlmError::Schema("no recipe".into()));
    }
    let sections = sections::assemble(groups, step_secs);
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
        steps: st,
        sections,
        tags: crate::jsonld::dedup(clean(r.tags)),
        image_url: None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_requires_nullable_section() {
        let v = fa_llm::json_schema_for::<super::LlmRecipe>().to_string();
        let item = fa_llm::json_schema_for::<super::LlmItem>();
        assert!(v.contains("section"));
        let req = item["required"].as_array().unwrap();
        assert!(req.iter().any(|r| r == "section") && req.iter().any(|r| r == "text"));
        assert!(item["properties"]["section"].to_string().contains("null"));
    }
}
