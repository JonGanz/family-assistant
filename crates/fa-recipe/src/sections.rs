//! Recipe sections (D-124): heading lines in ingredient lists, JSON-LD `HowToSection`s,
//! recipe-plugin ingredient groups, and pairing of ingredient groups with step sections.
//!
//! Ordering rule (`assemble`): ingredient groups are the backbone in their page order. A group that
//! pairs with a step section (equal normalised names) is merged with it; before it is emitted, every
//! not-yet-emitted step section that comes earlier in step order is emitted. Unpaired ingredient
//! groups are emitted where they stand; step sections left over are appended at the end.

use fa_domain::recipe::{DraftSection, RecipeIngredient};
use scraper::{ElementRef, Html, Selector};

use crate::parse_ingredient;

pub(crate) struct IngGroup {
    pub name: String,
    pub items: Vec<RecipeIngredient>,
}

const MAX_HEADING: usize = 60;

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The group name when `line` is a heading line (no quantity, short, "For the ..." or ends with `:`).
pub(crate) fn heading_name(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.chars().count() > MAX_HEADING || t.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let lower = t.to_lowercase();
    let for_the = lower.starts_with("for the ");
    if !(t.ends_with(':') || for_the) || parse_ingredient(t).quantity.is_some() {
        return None;
    }
    let mut name = t;
    if for_the {
        name = &name["for the ".len()..];
    }
    let name = name.trim().trim_end_matches(':').trim();
    (!name.is_empty()).then(|| capitalise(name))
}

/// Lower case, without "for the" / "make the" / "to make the" / "the", trailing punctuation and a
/// trailing plural "s".
pub(crate) fn norm(name: &str) -> String {
    let mut s = name.trim().to_lowercase();
    loop {
        let before = s.len();
        for p in ["to make the ", "make the ", "for the ", "the "] {
            if let Some(r) = s.strip_prefix(p) {
                s = r.trim_start().to_string();
            }
        }
        if s.len() == before {
            break;
        }
    }
    let s = s.trim_matches(|c: char| !c.is_alphanumeric());
    match s.strip_suffix('s') {
        Some(r) if r.len() > 2 && !r.ends_with('s') => r.to_string(),
        _ => s.to_string(),
    }
}

/// Splits lines into shared ones (before any heading) and named groups; headings are dropped.
pub(crate) fn group_lines(lines: &[String]) -> (Vec<RecipeIngredient>, Vec<IngGroup>) {
    let mut shared = Vec::new();
    let mut groups: Vec<IngGroup> = Vec::new();
    for l in lines {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(name) = heading_name(l) {
            groups.push(IngGroup {
                name,
                items: vec![],
            });
        } else if let Some(g) = groups.last_mut() {
            g.items.push(parse_ingredient(l));
        } else {
            shared.push(parse_ingredient(l));
        }
    }
    groups.retain(|g| !g.items.is_empty());
    (shared, groups)
}

pub(crate) fn has_heading(lines: &[String]) -> bool {
    lines.iter().any(|l| heading_name(l).is_some())
}

/// Pairs ingredient groups with step sections by normalised name; see the module docs for order.
pub(crate) fn assemble(
    groups: Vec<IngGroup>,
    step_sections: Vec<(String, Vec<String>)>,
) -> Vec<DraftSection> {
    let mut steps: Vec<Option<(String, Vec<String>)>> =
        step_sections.into_iter().map(Some).collect();
    let mut out = Vec::new();
    for g in groups {
        let key = norm(&g.name);
        let pos = steps
            .iter()
            .position(|s| s.as_ref().is_some_and(|(n, _)| norm(n) == key));
        match pos {
            Some(k) => {
                for s in steps.iter_mut().take(k) {
                    if let Some((n, st)) = s.take() {
                        out.push(DraftSection {
                            name: n,
                            ingredients: vec![],
                            steps: st,
                        });
                    }
                }
                let (n, st) = steps[k].take().unwrap_or_default();
                out.push(DraftSection {
                    name: if n.is_empty() { g.name } else { n },
                    ingredients: g.items,
                    steps: st,
                });
            }
            None => out.push(DraftSection {
                name: g.name,
                ingredients: g.items,
                steps: vec![],
            }),
        }
    }
    for (n, st) in steps.into_iter().flatten() {
        out.push(DraftSection {
            name: n,
            ingredients: vec![],
            steps: st,
        });
    }
    out.retain(|s| !(s.ingredients.is_empty() && s.steps.is_empty()));
    out
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn loose(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn el_text(e: ElementRef<'_>) -> String {
    squash(&e.text().collect::<Vec<_>>().join(" "))
}

/// Ingredient groups of recipe-plugin markup, `(group name, lines)`; the name is empty when unnamed.
///
/// - WP Recipe Maker: `.wprm-recipe-ingredient-group` holding `.wprm-recipe-group-name` and
///   `li.wprm-recipe-ingredient` (verified in the plugin source, class-wprm-sc-ingredients.php).
/// - Tasty Recipes: the card body `.tasty-recipes-ingredients-body` (verified in the plugin
///   template) holds free editor HTML; group names are its headings (h2 to h6) and lines its `li`.
pub(crate) fn plugin_groups(html: &str) -> Vec<(String, Vec<String>)> {
    let doc = Html::parse_document(html);
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let sel = |s: &str| Selector::parse(s).ok();
    if let (Some(g), Some(n), Some(li)) = (
        sel(".wprm-recipe-ingredient-group"),
        sel(".wprm-recipe-group-name"),
        sel("li.wprm-recipe-ingredient"),
    ) {
        for grp in doc.select(&g) {
            let name = grp.select(&n).next().map(el_text).unwrap_or_default();
            let lines: Vec<String> = grp.select(&li).map(el_text).collect();
            if !lines.is_empty() {
                out.push((name, lines));
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    if let (Some(body), Some(any)) = (
        sel(".tasty-recipes-ingredients-body"),
        sel("h2, h3, h4, h5, h6, li"),
    ) {
        for b in doc.select(&body) {
            let mut cur: (String, Vec<String>) = (String::new(), vec![]);
            for e in b.select(&any) {
                if e.value().name() == "li" {
                    cur.1.push(el_text(e));
                } else {
                    if !cur.1.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur.0 = el_text(e).trim_end_matches(':').trim().to_string();
                }
            }
            if !cur.1.is_empty() {
                out.push(cur);
            }
        }
    }
    out
}

/// Splits JSON-LD ingredient lines by plugin groups: a line whose text matches a group line goes
/// into that group; the rest stay shared. `None` when no named group matched.
pub(crate) fn assign_plugin(
    lines: &[String],
    plugin: &[(String, Vec<String>)],
) -> Option<(Vec<RecipeIngredient>, Vec<IngGroup>)> {
    let mut shared = Vec::new();
    let mut groups: Vec<IngGroup> = Vec::new();
    for l in lines {
        let key = loose(l);
        let hit = plugin
            .iter()
            .find(|(n, ls)| !n.is_empty() && ls.iter().any(|x| loose(x) == key));
        match hit {
            Some((n, _)) => {
                let name = capitalise(n);
                match groups.iter_mut().find(|g| g.name == name) {
                    Some(g) => g.items.push(parse_ingredient(l)),
                    None => groups.push(IngGroup {
                        name,
                        items: vec![parse_ingredient(l)],
                    }),
                }
            }
            None => shared.push(parse_ingredient(l)),
        }
    }
    (!groups.is_empty()).then_some((shared, groups))
}
