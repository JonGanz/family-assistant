//! fa-recipe: recipe extraction: schema.org JSON-LD, then microdata, then LLM fallback (M4, M5, D-094, D-109).
//!
//! Stores nothing; M5 adds storage and moves the types to `fa-domain`.

mod ingredient;
mod jsonld;
mod llm;
mod microdata;
mod types;

pub use ingredient::parse_ingredient;
pub use jsonld::{from_html, parse_iso_duration};
pub use llm::from_llm;
pub use microdata::from_microdata;
pub use types::{RecipeDraft, RecipeError, RecipeIngredient, RecipeInput, RecipeOrigin};

use fa_connectors::http::Http;
use scraper::{ElementRef, Html, node::Node};
use url::Url;

/// Fetches a page through the guarded client and returns its text.
pub async fn fetch_page(http: &Http, url: &Url) -> Result<String, RecipeError> {
    let rb = http
        .get(url)
        .map_err(|e| RecipeError::Fetch(e.to_string()))?;
    let resp = http
        .send(rb)
        .await
        .map_err(|e| RecipeError::Fetch(e.to_string()))?;
    let is_html = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("html") || v.contains("xml")
        })
        // A missing content type is treated as html.
        .unwrap_or(true);
    let bytes = http
        .ok_body(resp)
        .await
        .map_err(|e| RecipeError::Fetch(e.to_string()))?;
    if !is_html {
        return Err(RecipeError::NotHtml);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Text of the page without `script`, `style`, `nav` (and `noscript`), whitespace collapsed.
pub fn visible_text(html: &str) -> String {
    fn walk(el: ElementRef<'_>, out: &mut String) {
        for child in el.children() {
            match child.value() {
                Node::Text(t) => {
                    out.push_str(t);
                    out.push(' ');
                }
                Node::Element(e) => {
                    if matches!(e.name(), "script" | "style" | "nav" | "noscript") {
                        continue;
                    }
                    if let Some(c) = ElementRef::wrap(child) {
                        walk(c, out);
                    }
                }
                _ => {}
            }
        }
    }
    let doc = Html::parse_document(html);
    let mut raw = String::new();
    walk(doc.root_element(), &mut raw);
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// JSON-LD first, then microdata, then the LLM when a client is present.
pub async fn extract(
    http: &Http,
    llm: Option<&fa_llm::LlmClient>,
    input: RecipeInput,
) -> Result<RecipeDraft, RecipeError> {
    let (html, source_url) = match input {
        RecipeInput::Url(u) => (fetch_page(http, &u).await?, Some(u)),
        RecipeInput::Html { html, source_url } => (html, source_url),
    };
    if let Some(d) = from_html(&html, source_url.as_ref()) {
        return Ok(d);
    }
    if let Some(d) = from_microdata(&html, source_url.as_ref()) {
        return Ok(d);
    }
    let Some(client) = llm else {
        return Err(RecipeError::LlmUnavailable);
    };
    match from_llm(client, &visible_text(&html), source_url.as_ref()).await {
        Ok(d) => Ok(d),
        Err(fa_llm::LlmError::Schema(m)) if m == "no recipe" => Err(RecipeError::NoRecipe),
        Err(e) => Err(RecipeError::Llm(e)),
    }
}
