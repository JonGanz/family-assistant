//! Display formatting in household units; mirrors `apps/web/src/lib/units.ts`.

use fa_domain::Units;

use crate::dsl::Field;

fn round(v: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    let r = (v * f).round() / f;
    if r == 0.0 { 0.0 } else { r }
}

fn num(v: f64, digits: i32) -> String {
    format!("{}", round(v, digits))
}

pub fn c_to_f(c: f64) -> f64 {
    c * 9.0 / 5.0 + 32.0
}

pub fn mm_to_in(mm: f64) -> f64 {
    mm / 25.4
}

pub fn cm_to_in(cm: f64) -> f64 {
    cm / 2.54
}

pub fn kmh_to_mph(k: f64) -> f64 {
    k / 1.609_344
}

/// Formats a metric value of `field` for display (the `{value}` placeholder).
pub fn format_value(field: Field, v: f64, units: Units) -> String {
    let imperial = units == Units::Imperial;
    match field {
        Field::TempC | Field::FeelsC => {
            if imperial {
                format!("{}°F", num(c_to_f(v), 0))
            } else {
                format!("{}°C", num(v, 0))
            }
        }
        Field::PrecipMm => {
            if imperial {
                format!("{} in", num(mm_to_in(v), 2))
            } else {
                format!("{} mm", num(v, 1))
            }
        }
        Field::SnowCm => {
            if imperial {
                format!("{} in", num(cm_to_in(v), 1))
            } else {
                format!("{} cm", num(v, 1))
            }
        }
        Field::GustKmh | Field::WindKmh => {
            if imperial {
                format!("{} mph", num(kmh_to_mph(v), 0))
            } else {
                format!("{} km/h", num(v, 0))
            }
        }
        Field::PrecipProb | Field::Code => num(v, 0),
        Field::Uv => num(v, 1),
    }
}
