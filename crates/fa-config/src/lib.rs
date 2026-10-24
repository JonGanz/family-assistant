//! fa-config: process configuration (figment) and the household TOML (import/export).

mod household;
mod process;

pub use household::{
    HouseholdConfig, HouseholdSection, HouseholdTomlError, ImportError, ImportMode, ImportReport,
    LocationConfig, MemberConfig, SCHEMA_VERSION, SUPPORTED_SECTIONS, export, import,
};
pub use process::{ConfigError, LogFormat, ProcessConfig};
