use crate::RecipeIngredient;

fn unicode_fraction(c: char) -> Option<f64> {
    Some(match c {
        '½' => 0.5,
        '¼' => 0.25,
        '¾' => 0.75,
        '⅓' => 1.0 / 3.0,
        '⅔' => 2.0 / 3.0,
        '⅛' => 0.125,
        _ => return None,
    })
}

fn digits(s: &str) -> (&str, &str) {
    let n = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    s.split_at(n)
}

/// `n/d` at the start of `s` (digits, slash, digits).
fn fraction(s: &str) -> Option<(f64, &str)> {
    let (n, rest) = digits(s);
    let rest = rest.strip_prefix('/')?;
    let (d, rest) = digits(rest);
    let (n, d): (f64, f64) = (n.parse().ok()?, d.parse().ok()?);
    (d > 0.0).then_some((n / d, rest))
}

/// One number: `3`, `1.5`, `1/2`, `1 1/2`, `½`, `1½`, `1 ½`.
fn number(s: &str) -> Option<(f64, &str)> {
    let first = s.chars().next()?;
    if let Some(f) = unicode_fraction(first) {
        return Some((f, &s[first.len_utf8()..]));
    }
    if !first.is_ascii_digit() {
        return None;
    }
    if let Some(r) = fraction(s) {
        return Some(r);
    }
    let (int, mut rest) = digits(s);
    let mut text = int.to_string();
    if let Some(after) = rest.strip_prefix('.') {
        let (frac, r) = digits(after);
        if !frac.is_empty() {
            text = format!("{int}.{frac}");
            rest = r;
        }
    }
    let v: f64 = text.parse().ok()?;
    if !text.contains('.') {
        let trimmed = rest.trim_start();
        if let Some(c) = trimmed.chars().next().and_then(unicode_fraction_char) {
            return Some((v + c.0, &trimmed[c.1..]));
        }
        if rest.starts_with(char::is_whitespace)
            && let Some((f, r)) = fraction(trimmed)
        {
            return Some((v + f, r));
        }
    }
    Some((v, rest))
}

fn unicode_fraction_char(c: char) -> Option<(f64, usize)> {
    unicode_fraction(c).map(|f| (f, c.len_utf8()))
}

/// Quantity with an optional range; the low end wins.
fn quantity(s: &str) -> Option<(f64, &str)> {
    let (v, rest) = number(s)?;
    let t = rest.trim_start();
    let after = t
        .strip_prefix(['-', '–', '—'])
        .or_else(|| t.strip_prefix("to ").or_else(|| t.strip_prefix("TO ")));
    if let Some(a) = after
        && let Some((_, r)) = number(a.trim_start())
    {
        return Some((v, r));
    }
    Some((v, rest))
}

fn normalise_unit(w: &str) -> Option<&'static str> {
    Some(match w.to_ascii_lowercase().as_str() {
        "cup" | "cups" | "c" => "cup",
        "tbsp" | "tbsps" | "tablespoon" | "tablespoons" | "tbs" => "tbsp",
        "tsp" | "tsps" | "teaspoon" | "teaspoons" => "tsp",
        "g" | "gram" | "grams" => "g",
        "kg" | "kilogram" | "kilograms" => "kg",
        "ml" | "milliliter" | "milliliters" => "ml",
        "l" | "liter" | "liters" | "litre" | "litres" => "l",
        "pint" | "pints" | "pt" => "pint",
        "quart" | "quarts" | "qt" => "quart",
        "gallon" | "gallons" | "gal" => "gallon",
        "piece" | "pieces" => "piece",
        "stick" | "sticks" => "stick",
        "bunch" | "bunches" => "bunch",
        "head" | "heads" => "head",
        "sprig" | "sprigs" => "sprig",
        "oz" | "ounce" | "ounces" => "oz",
        "lb" | "lbs" | "pound" | "pounds" => "lb",
        "clove" | "cloves" => "clove",
        "can" | "cans" => "can",
        "pinch" | "pinches" => "pinch",
        "slice" | "slices" => "slice",
        _ => return None,
    })
}

/// `fl oz` / `fluid ounce(s)` at the start of `s`: the unit and the byte length consumed.
fn fluid_ounce(s: &str) -> Option<usize> {
    let lower = s.to_ascii_lowercase();
    for p in ["fluid ounces", "fluid ounce", "fl. oz.", "fl. oz", "fl oz."] {
        if lower.starts_with(p) {
            return Some(p.len());
        }
    }
    if let Some(r) = lower.strip_prefix("fl oz")
        && !r.starts_with(char::is_alphabetic)
    {
        return Some(5);
    }
    None
}

/// Drops a leading `(...)` such as `(14 oz)`.
fn skip_parenthetical(s: &str) -> &str {
    if s.starts_with('(')
        && let Some(i) = s.find(')')
    {
        return s[i + 1..].trim_start();
    }
    s
}

fn strip_suffix_ci<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    let n = s.len().checked_sub(suffix.len())?;
    (s.is_char_boundary(n) && s[n..].eq_ignore_ascii_case(suffix)).then(|| &s[..n])
}

/// Deterministic first pass; fields stay `None` when unsure. `text` is always the line.
pub fn parse_ingredient(line: &str) -> RecipeIngredient {
    let mut out = RecipeIngredient {
        text: line.to_string(),
        quantity: None,
        unit: None,
        item: None,
    };
    let Some((q, mut rest)) = quantity(line.trim_start()) else {
        return out;
    };
    out.quantity = Some(q);
    rest = skip_parenthetical(rest.trim_start());
    let word_end = rest
        .find(|c: char| !c.is_alphabetic())
        .unwrap_or(rest.len());
    if let Some(n) = fluid_ounce(rest) {
        out.unit = Some("fl oz".to_string());
        rest = rest[n..].trim_start();
    } else if let Some(u) = normalise_unit(&rest[..word_end]) {
        out.unit = Some(u.to_string());
        rest = rest[word_end..].trim_start_matches('.').trim_start();
    }
    let lower = rest.to_ascii_lowercase();
    if lower.starts_with("of ") {
        rest = rest[3..].trim_start();
    }
    let mut rest = rest.trim();
    for tail in ["to taste", "as needed"] {
        if let Some(r) = strip_suffix_ci(rest, tail) {
            rest = r.trim_end().trim_end_matches(',').trim_end();
        }
    }
    // Preparation notes after the first comma are not part of the item.
    let rest = rest.split(',').next().unwrap_or(rest).trim();
    if !rest.is_empty() {
        out.item = Some(rest.to_string());
    }
    out
}
