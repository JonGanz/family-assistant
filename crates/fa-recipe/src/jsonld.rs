use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;

use crate::{RecipeDraft, RecipeOrigin, parse_ingredient};

/// ISO 8601 duration to minutes (seconds round up). Anything unsupported is `None`.
pub fn parse_iso_duration(s: &str) -> Option<u32> {
    let s = s.trim().strip_prefix(['P', 'p'])?;
    let (date, time) = match s.split_once(['T', 't']) {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut secs: u64 = 0;
    let mut any = false;
    for (part, units) in [
        (date, [("W", 604_800u64), ("D", 86_400), ("", 0)]),
        (time.unwrap_or(""), [("H", 3600), ("M", 60), ("S", 1)]),
    ] {
        let mut num = String::new();
        for c in part.chars() {
            if c.is_ascii_digit() {
                num.push(c);
                continue;
            }
            let mult = units
                .iter()
                .find(|(u, m)| *m > 0 && u.eq_ignore_ascii_case(&c.to_string()))?
                .1;
            secs += num.parse::<u64>().ok()? * mult;
            num.clear();
            any = true;
        }
        if !num.is_empty() {
            return None;
        }
    }
    if !any || time == Some("") {
        return None;
    }
    u32::try_from(secs.div_ceil(60)).ok()
}

pub(crate) fn dedup(v: Vec<String>) -> Vec<String> {
    let mut seen = Vec::<String>::new();
    let mut out = Vec::new();
    for s in v {
        let k = s.to_lowercase();
        if !s.is_empty() && !seen.contains(&k) {
            seen.push(k);
            out.push(s);
        }
    }
    out
}

fn is_recipe(v: &Value) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s == "Recipe",
        Some(Value::Array(a)) => a.iter().any(|t| t.as_str() == Some("Recipe")),
        _ => false,
    }
}

fn collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
        Value::Object(_) => {
            if is_recipe(v) {
                out.push(v);
            }
            if let Some(g) = v.get("@graph") {
                collect(g, out);
            }
        }
        _ => {}
    }
}

fn text(v: &Value) -> Option<String> {
    let s = match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    Some(s).filter(|s| !s.is_empty())
}

/// A string or an array of strings, split on commas.
fn list(v: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    match v {
        Some(Value::String(s)) => out.extend(s.split(',').map(|p| p.trim().to_string())),
        Some(Value::Array(a)) => {
            for x in a {
                out.extend(list(Some(x)));
            }
        }
        _ => {}
    }
    out.retain(|s| !s.is_empty());
    out
}

fn steps(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.extend(
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty()),
        ),
        Value::Array(a) => a.iter().for_each(|x| steps(x, out)),
        Value::Object(o) => {
            if let Some(items) = o.get("itemListElement") {
                steps(items, out);
            } else if let Some(t) = o.get("text").or_else(|| o.get("name")).and_then(text) {
                out.push(t);
            }
        }
        _ => {}
    }
}

fn image(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Object(o) => o.get("url").and_then(image),
        Value::Array(a) => a.iter().find_map(image),
        _ => None,
    }
}

fn minutes(v: Option<&Value>) -> Option<u32> {
    v.and_then(Value::as_str).and_then(parse_iso_duration)
}

fn leading_int(s: &str) -> Option<u32> {
    let t = s.trim_start();
    let n = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    t[..n].parse().ok()
}

fn yield_text(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Array(a) => {
            let all: Vec<String> = a.iter().filter_map(text).collect();
            all.iter()
                .find(|s| s.chars().any(|c| c.is_ascii_digit()))
                .or(all.first())
                .cloned()
        }
        other => text(other),
    }
}

fn draft(r: &Value, source_url: Option<&Url>) -> Option<RecipeDraft> {
    let name = r.get("name").and_then(text)?;
    let ingredients: Vec<_> = list_lines(r.get("recipeIngredient"))
        .iter()
        .map(|l| parse_ingredient(l))
        .collect();
    let mut st = Vec::new();
    if let Some(i) = r.get("recipeInstructions") {
        steps(i, &mut st);
    }
    let mut tags = list(r.get("keywords"));
    tags.extend(list(r.get("recipeCategory")));
    tags.extend(list(r.get("recipeCuisine")));
    let yt = yield_text(r.get("recipeYield"));
    Some(RecipeDraft {
        name,
        source_url: source_url.map(|u| u.to_string()),
        origin: RecipeOrigin::Jsonld,
        servings: yt.as_deref().and_then(leading_int),
        yield_text: yt,
        prep_minutes: minutes(r.get("prepTime")),
        cook_minutes: minutes(r.get("cookTime")),
        total_minutes: minutes(r.get("totalTime")),
        ingredients,
        steps: st,
        tags: dedup(tags),
        image_url: r.get("image").and_then(image),
    })
}

/// Ingredient lines are not comma-split.
fn list_lines(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a.iter().filter_map(text).collect(),
        Some(Value::String(s)) => s
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

/// Parses schema.org `Recipe` JSON-LD (object, array or `@graph`); no I/O.
pub fn from_html(html: &str, source_url: Option<&Url>) -> Option<RecipeDraft> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    let mut best: Option<RecipeDraft> = None;
    for script in doc.select(&sel) {
        let raw: String = script.text().collect();
        let Ok(v) = serde_json::from_str::<Value>(raw.trim()) else {
            continue;
        };
        let mut found = Vec::new();
        collect(&v, &mut found);
        for r in found {
            if let Some(d) = draft(r, source_url)
                && best
                    .as_ref()
                    .is_none_or(|b| d.ingredients.len() > b.ingredients.len())
            {
                best = Some(d);
            }
        }
    }
    best
}
