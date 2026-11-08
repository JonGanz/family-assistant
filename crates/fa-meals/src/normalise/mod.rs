//! The deterministic ingredient normaliser (D-113, R-007): parse, clean, singularise, convert
//! and display. Pure, no I/O. Matching against the catalogue is in [`crate::matching`].

mod clean;
mod units;

use fa_domain::Units;

use crate::seed_data::{self, SingularRule};

pub use clean::{fold, split_top_level_comma};
pub use units::{Dimension, canonical_unit, dimension_of, display, display_pair, to_base};

/// A line split into its parts before cleaning.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLine {
    pub qty: Option<f64>,
    /// The parser's short form (`cup`, `tbsp`, `g`, `fl oz`, ...).
    pub unit: Option<String>,
    /// The item text as written, not yet cleaned.
    pub item: String,
    /// "to taste" and text after a comma.
    pub note: Option<String>,
}

/// A line after parsing and cleaning.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalisedLine {
    pub qty: Option<f64>,
    pub unit: Option<String>,
    /// Cleaned, singular, lower case. `None` when nothing is left.
    pub item: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Normaliser {
    cleaner: clean::Cleaner,
}

impl Normaliser {
    pub fn new(descriptors: &[String], rules: &[SingularRule]) -> Self {
        Self {
            cleaner: clean::Cleaner::new(descriptors, rules),
        }
    }

    /// The seed descriptor list and singular exceptions.
    pub fn standard() -> Self {
        Self::new(&seed_data::descriptors(), &seed_data::singular_rules())
    }

    /// The same singular rules with the household's own descriptor list (empty means the seed's).
    pub fn with_descriptors(descriptors: &[String]) -> Self {
        if descriptors.is_empty() {
            Self::standard()
        } else {
            Self::new(descriptors, &seed_data::singular_rules())
        }
    }

    /// Wraps the `fa-recipe` rule parser. Text after a top-level comma and a trailing "to taste"
    /// go to `note`. An unparsed line is all item.
    pub fn parse_line(raw: &str) -> ParsedLine {
        let raw = raw.trim();
        let p = fa_recipe::parse_ingredient(raw);
        let mut notes: Vec<String> = Vec::new();
        let (_, tail) = split_top_level_comma(raw);
        if let Some(t) = tail.filter(|t| !t.is_empty()) {
            notes.push(t.to_string());
        }
        let (_, trailing) = clean::take_trailing_note(raw);
        if let Some(t) = trailing
            && !notes.iter().any(|n| n.eq_ignore_ascii_case(t))
        {
            notes.push(t.to_string());
        }
        let item = match p.item {
            Some(i) => i,
            // "salt" alone has no quantity and no item from the parser.
            None if p.quantity.is_none() => raw.to_string(),
            None => String::new(),
        };
        ParsedLine {
            qty: p.quantity,
            unit: p.unit,
            item,
            note: (!notes.is_empty()).then(|| notes.join(", ")),
        }
    }

    pub fn clean_item(&self, item: &str) -> (String, Option<String>) {
        self.cleaner.clean_item(item)
    }

    pub fn singular(&self, phrase: &str) -> String {
        self.cleaner.singular(phrase)
    }

    /// Parse then clean. Notes from both steps are merged.
    pub fn normalise(&self, raw: &str) -> NormalisedLine {
        let p = Self::parse_line(raw);
        let (item, note) = self.clean_item(&p.item);
        let mut notes: Vec<String> = Vec::new();
        for n in [note, p.note].into_iter().flatten() {
            for part in n.split(", ") {
                if !part.is_empty() && !notes.iter().any(|x| x.eq_ignore_ascii_case(part)) {
                    notes.push(part.to_string());
                }
            }
        }
        NormalisedLine {
            qty: p.qty,
            unit: p.unit,
            item: Some(item).filter(|i| !i.is_empty()),
            note: (!notes.is_empty()).then(|| notes.join(", ")),
        }
    }

    /// See [`units::to_base`].
    pub fn to_base(qty: f64, unit: Option<&str>) -> Option<(f64, String)> {
        units::to_base(qty, unit)
    }

    /// See [`units::display`].
    pub fn display(qty: f64, unit: &str, units: Units) -> String {
        units::display(qty, unit, units)
    }
}

/// `clean_item` with the lists passed in, for callers that hold them.
pub fn clean_item(
    item: &str,
    descriptors: &[String],
    singular_rules: &[SingularRule],
) -> (String, Option<String>) {
    Normaliser::new(descriptors, singular_rules).clean_item(item)
}

#[cfg(test)]
mod tests;
