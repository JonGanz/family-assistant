//! fa-config: process configuration (figment) and the household TOML (import/export).

mod feeds;
mod household;
mod mail;
mod process;
pub mod runtime;
mod tasks;

pub use household::{
    CalendarConfig, CalendarSourceConfig, HouseholdConfig, HouseholdSection, HouseholdTomlError,
    ImportError, ImportMode, ImportReport, LocationConfig, MemberConfig, SCHEMA_VERSION,
    SUPPORTED_SECTIONS, WeatherRuleConfig, create_household, export, import,
};
pub use mail::{
    BillTemplateConfig, CategoryConfig, EnrollmentConfig, EventsConfig, GenderTermsConfig,
    ImapSection, InboxConfig, MailRetentionConfig, MailRuleConfig, MailSourceConfig, OrgConfig,
    OrgContactConfig, RetentionConfig,
};
pub use process::{ConfigError, CookieSecure, LogFormat, ProcessConfig, SecretString};
pub use runtime::{RUNTIME_KEYS, RuntimeKey, RuntimeSnapshot, validate_value};
pub use tasks::{
    NotifyConfig, RoutineConfig, RoutineStepConfig, TaskDefinitionConfig, TaskListConfig,
};
