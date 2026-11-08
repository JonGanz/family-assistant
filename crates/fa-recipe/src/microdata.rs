use scraper::{ElementRef, Html, Selector};
use url::Url;

use crate::jsonld::{dedup, leading_int, parse_iso_duration};
use crate::{RecipeDraft, RecipeOrigin, parse_ingredient};

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_of(el: ElementRef<'_>) -> String {
    collapse(&el.text().collect::<String>())
}

/// `content`, then `src`, `href`, `datetime`, then the element text.
fn value(el: ElementRef<'_>) -> String {
    let e = el.value();
    for a in ["content", "src", "href", "datetime"] {
        if let Some(v) = e.attr(a) {
            return collapse(v);
        }
    }
    text_of(el)
}

fn has_type(el: ElementRef<'_>, name: &str) -> bool {
    el.value().attr("itemtype").is_some_and(|t| {
        t.split_whitespace().any(|t| {
            let t = t.trim_end_matches('/');
            ["http://", "https://"].iter().any(|p| {
                t.strip_prefix(p).is_some_and(|r| {
                    let r = r.strip_prefix("www.").unwrap_or(r);
                    r.strip_prefix("schema.org/") == Some(name)
                })
            })
        })
    })
}

/// Direct properties of `root`: nested `itemscope` elements are not entered.
fn props<'a>(root: ElementRef<'a>, out: &mut Vec<(String, ElementRef<'a>)>) {
    for child in root.children().filter_map(ElementRef::wrap) {
        let e = child.value();
        if let Some(p) = e.attr("itemprop") {
            for name in p.split_whitespace() {
                out.push((name.to_string(), child));
            }
            continue;
        }
        if e.attr("itemscope").is_none() {
            props(child, out);
        }
    }
}

fn find_steps(el: ElementRef<'_>, out: &mut Vec<String>) -> bool {
    let mut found = false;
    for d in el.descendants().filter_map(ElementRef::wrap) {
        if has_type(d, "HowToStep") {
            found = true;
            out.push(step_text(d));
        }
    }
    found
}

fn step_text(step: ElementRef<'_>) -> String {
    let mut inner = Vec::new();
    props(step, &mut inner);
    inner
        .iter()
        .find(|(n, _)| n == "text")
        .map(|(_, e)| text_of(*e))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| text_of(step))
}

fn instructions(el: ElementRef<'_>, out: &mut Vec<String>) {
    if has_type(el, "HowToStep") {
        out.push(step_text(el));
    } else if !find_steps(el, out) {
        let li = Selector::parse("li").ok();
        let items: Vec<String> = li
            .iter()
            .flat_map(|s| el.select(s))
            .map(text_of)
            .filter(|t| !t.is_empty())
            .collect();
        if items.is_empty() {
            let t = value(el);
            if !t.is_empty() {
                out.push(t);
            }
        } else {
            out.extend(items);
        }
    }
}

fn commas(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
}

fn minutes(s: &str) -> Option<u32> {
    parse_iso_duration(s)
}

/// Parses a schema.org `Recipe` marked up with microdata; no I/O.
pub fn from_microdata(html: &str, source_url: Option<&Url>) -> Option<RecipeDraft> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("[itemscope][itemtype]").ok()?;
    let mut best: Option<RecipeDraft> = None;
    for root in doc.select(&sel).filter(|e| has_type(*e, "Recipe")) {
        let mut ps = Vec::new();
        props(root, &mut ps);
        let first = |name: &str| {
            ps.iter()
                .filter(|(n, _)| n == name)
                .map(|(_, e)| value(*e))
                .find(|v| !v.is_empty())
        };
        let Some(name) = first("name") else { continue };
        let mut ingredients = Vec::new();
        let mut st = Vec::new();
        let mut tags = Vec::new();
        for (n, e) in &ps {
            match n.as_str() {
                "recipeIngredient" | "ingredients" => {
                    let v = value(*e);
                    if !v.is_empty() {
                        ingredients.push(parse_ingredient(&v));
                    }
                }
                "recipeInstructions" => instructions(*e, &mut st),
                "keywords" | "recipeCategory" | "recipeCuisine" => tags.extend(commas(&value(*e))),
                _ => {}
            }
        }
        let yt = first("recipeYield");
        let d = RecipeDraft {
            name,
            source_url: source_url.map(|u| u.to_string()),
            origin: RecipeOrigin::Microdata,
            servings: yt.as_deref().and_then(leading_int),
            yield_text: yt,
            prep_minutes: first("prepTime").as_deref().and_then(minutes),
            cook_minutes: first("cookTime").as_deref().and_then(minutes),
            total_minutes: first("totalTime").as_deref().and_then(minutes),
            ingredients,
            steps: st,
            tags: dedup(tags),
            image_url: first("image"),
        };
        if best
            .as_ref()
            .is_none_or(|b| d.ingredients.len() > b.ingredients.len())
        {
            best = Some(d);
        }
    }
    best
}
