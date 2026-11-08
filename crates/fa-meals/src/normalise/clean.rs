//! Cleaning an ingredient item: ASCII folding, descriptors, parentheticals, preparation notes
//! and singularising. Pure functions, no I/O.

use std::collections::{HashMap, HashSet};

use crate::seed_data::SingularRule;

/// Descriptor words that describe how the ingredient is prepared. When one of them is removed
/// from the item it is kept in the note.
const PREPARATION: &[&str] = &[
    "chopped", "diced", "minced", "sliced", "grated", "shredded", "crushed", "cubed", "peeled",
    "trimmed", "rinsed", "drained", "melted", "softened", "divided", "packed", "chilled", "finely",
    "roughly", "coarsely", "thinly", "freshly",
];

/// Phrases at the end of a line that mean "no quantity needed".
const TRAILING_NOTES: &[&str] = &[
    "to taste",
    "as needed",
    "for serving",
    "for garnish",
    "optional",
];

fn fold_char(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => "a",
        'ç' | 'ć' | 'č' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ī' => "i",
        'ñ' | 'ń' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ø' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' => "u",
        'ý' | 'ÿ' => "y",
        'š' | 'ś' => "s",
        'ž' | 'ź' | 'ż' => "z",
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        '’' | '‘' | '`' => "'",
        '‐' | '‑' | '–' | '—' => "-",
        _ => return None,
    })
}

/// Lower case with the common accented letters folded to ASCII.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        match fold_char(c) {
            Some(r) => out.push_str(r),
            None => out.push(c),
        }
    }
    out
}

/// Splits `(...)` groups out of `s`. Returns the text without them and their contents.
fn take_parentheticals(s: &str) -> (String, Vec<String>) {
    let mut rest = String::new();
    let mut notes = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' => {
                if depth == 0 {
                    rest.push(' ');
                } else {
                    cur.push(c);
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let t = cur.trim();
                    if !t.is_empty() {
                        notes.push(t.to_string());
                    }
                    cur.clear();
                } else {
                    cur.push(c);
                }
            }
            _ if depth > 0 => cur.push(c),
            _ => rest.push(c),
        }
    }
    (rest, notes)
}

/// Splits at the first comma that is not inside parentheses.
pub fn split_top_level_comma(s: &str) -> (&str, Option<&str>) {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            ',' if depth == 0 => return (&s[..i], Some(s[i + 1..].trim())),
            _ => {}
        }
    }
    (s, None)
}

/// Removes a trailing "to taste" style phrase. Returns the rest and the phrase.
pub fn take_trailing_note(s: &str) -> (String, Option<&'static str>) {
    let lower = s.trim_end().to_lowercase();
    for phrase in TRAILING_NOTES {
        if let Some(head) = lower.strip_suffix(phrase) {
            let keep = head.trim_end().trim_end_matches([',', ';', '-']).trim_end();
            let n = keep.chars().count();
            let cut: String = s.trim_end().chars().take(n).collect();
            return (cut, Some(phrase));
        }
    }
    (s.to_string(), None)
}

#[derive(Debug, Clone, Default)]
pub struct Cleaner {
    descriptors: HashSet<String>,
    pairs: HashMap<String, String>,
    keep: HashSet<String>,
}

impl Cleaner {
    pub fn new(descriptors: &[String], rules: &[SingularRule]) -> Self {
        let mut pairs = HashMap::new();
        let mut keep = HashSet::new();
        for r in rules {
            match r {
                SingularRule::Pair { plural, singular } => {
                    pairs.insert(fold(plural), fold(singular));
                }
                SingularRule::Keep { keep: k } => {
                    keep.insert(fold(k));
                }
            }
        }
        Self {
            descriptors: descriptors.iter().map(|d| fold(d)).collect(),
            pairs,
            keep,
        }
    }

    /// The singular of a (possibly multi-word) item. Only the last word changes.
    pub fn singular(&self, phrase: &str) -> String {
        let phrase = phrase.trim();
        if phrase.is_empty() {
            return String::new();
        }
        let lower = fold(phrase);
        if let Some(s) = self.pairs.get(&lower) {
            return s.clone();
        }
        let (head, last) = match lower.rsplit_once(' ') {
            Some((h, l)) => (Some(h), l),
            None => (None, lower.as_str()),
        };
        if let Some(h) = head {
            // "cherry tomatoes": the pair rule for "tomatoes" applies to the last word.
            if let Some(s) = self.pairs.get(last) {
                return format!("{h} {s}");
            }
        }
        if self.keep.contains(last) || last.chars().any(|c| c.is_ascii_digit()) {
            return lower;
        }
        let one = pluralizer::pluralize(last, 1, false);
        match head {
            Some(h) => format!("{h} {one}"),
            None => one,
        }
    }

    /// Cleans an ingredient item into `(item, note)`: parentheticals and text after a comma
    /// go to the note, descriptors are dropped (preparation words go to the note), the rest is
    /// folded, lower case and singular. `item` is empty when nothing is left.
    pub fn clean_item(&self, item: &str) -> (String, Option<String>) {
        let mut notes: Vec<String> = Vec::new();
        let (without_paren, parens) = take_parentheticals(item);
        notes.extend(parens);
        let (head, tail) = split_top_level_comma(&without_paren);
        if let Some(t) = tail.filter(|t| !t.is_empty()) {
            notes.push(t.to_string());
        }
        let (head, trailing) = take_trailing_note(head);
        if let Some(t) = trailing {
            notes.push(t.to_string());
        }
        let folded = fold(&head);
        let tokens: Vec<String> = folded
            .split_whitespace()
            .map(|t| {
                t.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
                    .to_string()
            })
            .filter(|t| !t.is_empty())
            .collect();
        let mut kept: Vec<&str> = Vec::new();
        let mut removed_prep: Vec<&str> = Vec::new();
        for t in &tokens {
            if self.descriptors.contains(t.as_str()) {
                if PREPARATION.contains(&t.as_str()) {
                    removed_prep.push(t);
                }
            } else {
                kept.push(t);
            }
        }
        if kept.first() == Some(&"of") {
            kept.remove(0);
        }
        // Never end up with nothing: "large" alone stays "large".
        let phrase = if kept.is_empty() {
            tokens.join(" ")
        } else {
            kept.join(" ")
        };
        notes.extend(removed_prep.iter().map(|s| s.to_string()));
        let note = {
            let mut seen = Vec::new();
            for n in notes {
                if !seen.iter().any(|s: &String| s.eq_ignore_ascii_case(&n)) {
                    seen.push(n);
                }
            }
            (!seen.is_empty()).then(|| seen.join(", "))
        };
        (self.singular(&phrase), note)
    }
}
