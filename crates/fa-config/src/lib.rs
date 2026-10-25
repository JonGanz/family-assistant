//! fa-config: process configuration (figment) and the household TOML (import/export).

mod feeds;
mod household;
mod process;
mod tasks;

pub use household::{
    CalendarConfig, CalendarSourceConfig, HouseholdConfig, HouseholdSection, HouseholdTomlError,
    ImportError, ImportMode, ImportReport, LocationConfig, MemberConfig, SCHEMA_VERSION,
    SUPPORTED_SECTIONS, WeatherRuleConfig, export, import,
};
pub use process::{ConfigError, LogFormat, ProcessConfig, SecretString};
pub use tasks::{
    NotifyConfig, RoutineConfig, RoutineStepConfig, TaskDefinitionConfig, TaskListConfig,
};
