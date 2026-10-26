//! Small text helpers: whole-word matching, sentences, normalisation.

use aho_corasick::{AhoCorasick, MatchKind};

/// Case-insensitive whole-word matcher over several needles.
pub(crate) struct WordSet {
    ac: Option<AhoCorasick>,
}

impl WordSet {
    pub fn new<S: AsRef<str>>(needles: &[S]) -> Self {
        let lowered: Vec<String> = needles
            .iter()
            .map(|s| s.as_ref().trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let ac = if lowered.is_empty() {
            None
        } else {
            AhoCorasick::builder()
                .match_kind(MatchKind::Standard)
                .build(&lowered)
                .ok()
        };
        Self { ac }
    }

    pub fn is_match(&self, hay: &str) -> bool {
        let Some(ac) = &self.ac else { return false };
        let lower = hay.to_lowercase();
        ac.find_overlapping_iter(&lower)
            .any(|m| boundary_ok(&lower, m.start(), m.end()))
    }
}

fn boundary_ok(hay: &str, start: usize, end: usize) -> bool {
    let before = hay[..start].chars().next_back();
    let after = hay[end..].chars().next();
    let needle_first = hay[start..end].chars().next();
    let needle_last = hay[start..end].chars().next_back();
    // A needle that starts/ends with punctuation (girls') needs no boundary on that side.
    let left = needle_first.is_none_or(|c| !c.is_alphanumeric())
        || before.is_none_or(|c| !c.is_alphanumeric());
    let right = needle_last.is_none_or(|c| !c.is_alphanumeric())
        || after.is_none_or(|c| !c.is_alphanumeric());
    left && right
}

pub(crate) fn contains_word(hay: &str, needle: &str) -> bool {
    WordSet::new(&[needle]).is_match(hay)
}

pub(crate) fn contains_any_word<S: AsRef<str>>(hay: &str, needles: &[S]) -> bool {
    WordSet::new(needles).is_match(hay)
}

/// Lower-case alphanumeric words.
pub(crate) fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(crate) fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` chars, cut at a word boundary where possible.
pub(crate) fn truncate_chars(s: &str, max: usize) -> String {
    let s = collapse_ws(s);
    if s.chars().count() <= max {
        return s;
    }
    let cut: String = s.chars().take(max).collect();
    match cut.rfind(' ') {
        Some(i) if i > max / 2 => cut[..i].trim_end().to_owned(),
        _ => cut,
    }
}

/// The sentence (or line) around the byte range `start..end`.
pub(crate) fn sentence_around(text: &str, start: usize, end: usize) -> &str {
    let is_stop = |c: char| matches!(c, '.' | '!' | '?' | '\n');
    // A '.' followed by a non-space is not a sentence end (Nov.7, 3.30).
    let from = text[..start]
        .char_indices()
        .rev()
        .find(|&(i, c)| {
            is_stop(c) && (c == '\n' || text[i + c.len_utf8()..].starts_with(char::is_whitespace))
        })
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let to = text[end..]
        .char_indices()
        .find(|&(i, c)| {
            is_stop(c)
                && (c == '\n'
                    || text[end + i + c.len_utf8()..].is_empty()
                    || text[end + i + c.len_utf8()..].starts_with(char::is_whitespace))
        })
        .map(|(i, c)| end + i + if c == '\n' { 0 } else { c.len_utf8() })
        .unwrap_or(text.len());
    text[from..to].trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_words() {
        assert!(contains_word("Girls' volleyball tryouts", "girls'"));
        assert!(contains_word("the girls play", "girls"));
        assert!(contains_word("girl's team", "girl's"));
        assert!(!contains_word("Ava is a gilrs", "girls"));
        assert!(!contains_word("Savannah", "ava"));
        assert!(contains_word("Hi Ava, hello", "ava"));
        assert!(!contains_word("boys", "boy"));
    }

    #[test]
    fn sentences() {
        let t = "Hello there. Please sign the form by Friday, Nov. 7 at 6pm. Thanks!";
        let s = t.find("Friday").unwrap();
        let e = t.find("6pm").unwrap() + 3;
        assert_eq!(
            sentence_around(t, s, e),
            "Please sign the form by Friday, Nov. 7 at 6pm."
        );
    }

    #[test]
    fn truncation() {
        let s = truncate_chars(&"word ".repeat(40), 80);
        assert!(s.chars().count() <= 80);
        assert!(s.ends_with("word"));
    }
}
