//! The hand-written unit table and the kitchen rounding (arch/Meals.md §4.2 step 4).
//!
//! Quantities are stored in base units: `ml` for volume, `g` for weight, `count` for a bare
//! number, and the unit's own name for counted things (`clove`, `can`, ...). Only quantities of
//! one dimension are ever added.

use fa_domain::Units;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Volume,
    Weight,
    Count,
}

pub const ML_PER_TSP: f64 = 4.92892;
pub const ML_PER_TBSP: f64 = 14.7868;
pub const ML_PER_CUP: f64 = 240.0;
pub const G_PER_OZ: f64 = 28.3495;
pub const G_PER_LB: f64 = 453.592;

/// The canonical spelling of a unit word, or `None` when it is not in the table.
pub fn canonical_unit(word: &str) -> Option<&'static str> {
    Some(
        match word.trim().trim_end_matches('.').to_lowercase().as_str() {
            "tsp" | "tsps" | "teaspoon" | "teaspoons" => "tsp",
            "tbsp" | "tbsps" | "tbs" | "tablespoon" | "tablespoons" => "tbsp",
            "cup" | "cups" | "c" => "cup",
            "fl oz" | "fl. oz" | "floz" | "fluid ounce" | "fluid ounces" => "fl oz",
            "pint" | "pints" | "pt" => "pint",
            "quart" | "quarts" | "qt" => "quart",
            "gallon" | "gallons" | "gal" => "gallon",
            "l" | "liter" | "liters" | "litre" | "litres" => "l",
            "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => "ml",
            "g" | "gram" | "grams" => "g",
            "kg" | "kilogram" | "kilograms" => "kg",
            "oz" | "ounce" | "ounces" => "oz",
            "lb" | "lbs" | "pound" | "pounds" => "lb",
            "clove" | "cloves" => "clove",
            "can" | "cans" => "can",
            "piece" | "pieces" => "piece",
            "slice" | "slices" => "slice",
            "stick" | "sticks" => "stick",
            "bunch" | "bunches" => "bunch",
            "head" | "heads" => "head",
            "sprig" | "sprigs" => "sprig",
            "pinch" | "pinches" => "pinch",
            "count" | "each" => "count",
            _ => return None,
        },
    )
}

/// The dimension and the factor to the base unit (`ml`, `g`, or 1 for counts) of a canonical unit.
fn factor(unit: &str) -> Option<(Dimension, f64)> {
    Some(match unit {
        "tsp" => (Dimension::Volume, ML_PER_TSP),
        "tbsp" => (Dimension::Volume, ML_PER_TBSP),
        "cup" => (Dimension::Volume, ML_PER_CUP),
        "fl oz" => (Dimension::Volume, 29.5735),
        "pint" => (Dimension::Volume, 473.176),
        "quart" => (Dimension::Volume, 946.353),
        "gallon" => (Dimension::Volume, 3785.41),
        "l" => (Dimension::Volume, 1000.0),
        "ml" => (Dimension::Volume, 1.0),
        "oz" => (Dimension::Weight, G_PER_OZ),
        "lb" => (Dimension::Weight, G_PER_LB),
        "kg" => (Dimension::Weight, 1000.0),
        "g" => (Dimension::Weight, 1.0),
        "clove" | "can" | "piece" | "slice" | "stick" | "bunch" | "head" | "sprig" | "pinch"
        | "count" => (Dimension::Count, 1.0),
        _ => return None,
    })
}

/// Converts to the base unit: `(qty in ml, "ml")`, `(qty in g, "g")`, `(qty, "clove")`, or
/// `(qty, "count")` for no unit. `None` when the unit is not in the table.
pub fn to_base(qty: f64, unit: Option<&str>) -> Option<(f64, String)> {
    let Some(u) = unit.map(str::trim).filter(|u| !u.is_empty()) else {
        return Some((qty, "count".into()));
    };
    let canon = canonical_unit(u)?;
    let (dim, f) = factor(canon)?;
    Some(match dim {
        Dimension::Volume => (qty * f, "ml".into()),
        Dimension::Weight => (qty * f, "g".into()),
        Dimension::Count => (qty, canon.into()),
    })
}

/// The dimension of a stored (base) unit string. Anything that is not `ml` or `g` is a count.
pub fn dimension_of(base_unit: &str) -> Dimension {
    match base_unit {
        "ml" => Dimension::Volume,
        "g" => Dimension::Weight,
        _ => Dimension::Count,
    }
}

// ---- display ----

const QUARTERS: &[(f64, &str)] = &[(0.0, ""), (0.25, "¼"), (0.5, "½"), (0.75, "¾"), (1.0, "")];
const HALVES: &[(f64, &str)] = &[(0.0, ""), (0.5, "½"), (1.0, "")];
const KITCHEN: &[(f64, &str)] = &[
    (0.0, ""),
    (0.25, "¼"),
    (1.0 / 3.0, "⅓"),
    (0.5, "½"),
    (2.0 / 3.0, "⅔"),
    (0.75, "¾"),
    (1.0, ""),
];

/// A number as whole plus fraction glyph on the nearest grid point: `2 ½`, `¾`, `3`.
/// Returns the text and the rounded value.
fn fraction_text(v: f64, grid: &[(f64, &str)], min: f64) -> (String, f64) {
    let v = v.max(min);
    let whole = v.floor();
    let frac = v - whole;
    let (point, glyph) = grid
        .iter()
        .copied()
        .min_by(|a, b| (frac - a.0).abs().total_cmp(&(frac - b.0).abs()))
        .unwrap_or((0.0, ""));
    let (whole, glyph, point) = if point >= 1.0 {
        (whole + 1.0, "", 0.0)
    } else {
        (whole, glyph, point)
    };
    let value = whole + point;
    let text = match (whole >= 1.0, glyph.is_empty()) {
        (true, true) => format!("{whole:.0}"),
        (true, false) => format!("{whole:.0} {glyph}"),
        (false, false) => glyph.to_string(),
        (false, true) => "0".to_string(),
    };
    (text, value)
}

fn round_step(v: f64, step: f64, min: f64) -> f64 {
    ((v / step).round() * step).max(min)
}

/// A decimal without a trailing `.0`: `1.5`, `2`.
fn plain(v: f64) -> String {
    let s = format!("{:.2}", (v * 100.0).round() / 100.0);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn plural(unit: &str, value: f64) -> String {
    if value <= 1.0 {
        return unit.to_string();
    }
    if unit.ends_with("ch") || unit.ends_with("sh") || unit.ends_with('s') || unit.ends_with('x') {
        format!("{unit}es")
    } else {
        format!("{unit}s")
    }
}

fn metric_ml(ml: f64) -> String {
    let r = if ml < 10.0 {
        round_step(ml, 1.0, 1.0)
    } else if ml < 100.0 {
        round_step(ml, 5.0, 5.0)
    } else {
        round_step(ml, 25.0, 25.0)
    };
    if r >= 1000.0 {
        format!("{} l", plain(round_step(ml, 100.0, 100.0) / 1000.0))
    } else {
        format!("{} ml", plain(r))
    }
}

fn metric_g(g: f64) -> String {
    let r = if g < 10.0 {
        round_step(g, 1.0, 1.0)
    } else if g < 100.0 {
        round_step(g, 5.0, 5.0)
    } else {
        round_step(g, 25.0, 25.0)
    };
    if r >= 1000.0 {
        format!("{} kg", plain(round_step(g, 100.0, 100.0) / 1000.0))
    } else {
        format!("{} g", plain(r))
    }
}

fn imperial_ml(ml: f64) -> String {
    let tsp = ml / ML_PER_TSP;
    if tsp < 2.875 {
        let (t, _) = fraction_text(tsp, QUARTERS, 0.25);
        return format!("{t} tsp");
    }
    if ml < 58.0 {
        let (t, _) = fraction_text(ml / ML_PER_TBSP, HALVES, 0.5);
        return format!("{t} tbsp");
    }
    let (t, v) = fraction_text(ml / ML_PER_CUP, KITCHEN, 0.25);
    format!("{t} {}", plural("cup", v))
}

fn imperial_g(g: f64) -> String {
    let oz = g / G_PER_OZ;
    if g < 4.0 * G_PER_OZ {
        let (t, _) = fraction_text(oz, HALVES, 0.5);
        return format!("{t} oz");
    }
    if oz < 15.5 {
        return format!("{} oz", plain(round_step(oz, 1.0, 1.0)));
    }
    let (t, _) = fraction_text(g / G_PER_LB, QUARTERS, 0.25);
    format!("{t} lb")
}

/// The quantity text in the household's units, rounded to kitchen sizes. `unit` is a stored
/// base unit (`ml`, `g`, `count`, a count word) or a raw unit that was not convertible.
///
/// Rounding as shipped:
/// - metric volume: under 10 ml to 1 ml, under 100 ml to 5 ml, up to 1 l to 25 ml, then litres to 0.1 l;
/// - metric weight: under 10 g to 1 g, under 100 g to 5 g, up to 1 kg to 25 g, then kilograms to 0.1 kg;
/// - imperial volume: teaspoons to ¼ below 1 tbsp, tablespoons to ½ below ¼ cup, then cups to the nearest of ¼ ⅓ ½ ⅔ ¾;
/// - imperial weight: ounces to ½ below 4 oz, to 1 oz up to 1 lb, then pounds to ¼;
/// - counts: to the nearest of ¼ ⅓ ½ ⅔ ¾ and whole numbers.
pub fn display(qty: f64, unit: &str, units: Units) -> String {
    if !qty.is_finite() || qty <= 0.0 {
        return String::new();
    }
    match (dimension_of(unit), unit, units) {
        (Dimension::Volume, _, Units::Metric) => metric_ml(qty),
        (Dimension::Volume, _, Units::Imperial) => imperial_ml(qty),
        (Dimension::Weight, _, Units::Metric) => metric_g(qty),
        (Dimension::Weight, _, Units::Imperial) => imperial_g(qty),
        (Dimension::Count, "count" | "", _) => fraction_text(qty, KITCHEN, 0.25).0,
        (Dimension::Count, u, _) if factor(u).is_some() => {
            let (t, v) = fraction_text(qty, KITCHEN, 0.25);
            format!("{t} {}", plural(u, v))
        }
        // Not convertible: shown as it was typed.
        (_, u, _) => format!("{} {u}", plain(qty)),
    }
}

/// Up to two amounts of one item, e.g. `2 cups + 200 g`.
pub fn display_pair(
    a: (Option<f64>, Option<&str>),
    b: (Option<f64>, Option<&str>),
    units: Units,
) -> String {
    let one = |(q, u): (Option<f64>, Option<&str>)| {
        q.map(|q| display(q, u.unwrap_or("count"), units))
            .filter(|s| !s.is_empty())
    };
    [one(a), one(b)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" + ")
}
