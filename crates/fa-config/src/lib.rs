//! fa-config: process configuration (figment) and runtime settings.

mod household;
mod process;
pub mod runtime;

pub use household::{HouseholdError, create_household};
pub use process::{ConfigError, CookieSecure, LogFormat, ProcessConfig, SecretString};
pub use runtime::{RUNTIME_KEYS, RuntimeKey, RuntimeSnapshot, validate_value};
