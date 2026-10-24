use fa_domain::{RuleAction, Severity};
use fa_store::WeatherRuleRepo;
use sqlx::PgConnection;

use crate::WeatherError;

pub struct DefaultRule {
    pub name: &'static str,
    pub condition: &'static str,
    pub severity: Severity,
    pub message: &'static str,
}

pub const DEFAULT_RULES: [DefaultRule; 8] = [
    DefaultRule {
        name: "Frost tonight",
        condition: "min(temp_c, tonight) <= 0",
        severity: Severity::Warning,
        message: "Frost tonight, low of {value}",
    },
    DefaultRule {
        name: "Heavy rain",
        condition: "sum(precip_mm, next 12h) >= 15",
        severity: Severity::Warning,
        message: "Heavy rain: {value} in the {when}",
    },
    DefaultRule {
        name: "Likely rain today",
        condition: "max(precip_prob, today) >= 70",
        severity: Severity::Info,
        message: "Rain likely today ({value}%)",
    },
    DefaultRule {
        name: "Snow",
        condition: "sum(snow_cm, next 24h) >= 2",
        severity: Severity::Warning,
        message: "Snow expected: {value} in the {when}",
    },
    DefaultRule {
        name: "Strong wind",
        condition: "max(gust_kmh, next 12h) >= 60",
        severity: Severity::Warning,
        message: "Gusts up to {value} in the {when}",
    },
    DefaultRule {
        name: "Heat",
        condition: "max(feels_c, today) >= 32",
        severity: Severity::Warning,
        message: "Hot today, feels like {value}",
    },
    DefaultRule {
        name: "High UV",
        condition: "max(uv, today) >= 8",
        severity: Severity::Info,
        message: "High UV today ({value})",
    },
    DefaultRule {
        name: "Thunderstorm",
        condition: "any(code, today) >= 95",
        severity: Severity::Severe,
        message: "Thunderstorms possible today",
    },
];

/// Seeds the default rules for the transaction's household (`tx_for`). Idempotent; returns how
/// many rows were inserted.
pub async fn ensure_default_rules(conn: &mut PgConnection) -> Result<usize, WeatherError> {
    let mut n = 0;
    for r in &DEFAULT_RULES {
        if WeatherRuleRepo::name_exists(conn, r.name).await? {
            continue;
        }
        if WeatherRuleRepo::ensure_default(
            conn,
            r.name,
            r.condition,
            RuleAction::Alert,
            r.severity,
            r.message,
        )
        .await?
        {
            n += 1;
        }
    }
    Ok(n)
}
