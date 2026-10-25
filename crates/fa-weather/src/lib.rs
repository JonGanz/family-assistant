//! fa-weather (M1): Open-Meteo fetch, the rule DSL, evaluation, advisories and jobs.
//! See arch/Feeds.md section 4.

pub mod dsl;
pub mod eval;
pub mod units;

mod defaults;
mod jobs;
mod service;

pub use defaults::{DEFAULT_RULES, DefaultRule, ensure_default_rules};
pub use dsl::{ParseError, Rule, parse};
pub use eval::{Evaluation, evaluate, render_message};
pub use jobs::{default_schedules, handlers};
pub use service::{Preview, Validation, WeatherConfig, WeatherService};

#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
    #[error(transparent)]
    Connector(#[from] fa_connectors::ConnectorError),
    #[error("job error: {0}")]
    Job(String),
}

/// Checks that `s` is a valid weather-rule condition.
pub fn validate_condition(s: &str) -> Result<(), ParseError> {
    parse(s).map(|_| ())
}
