//! fa-domain: IDs, enums, entities, the authorization model and validation.
//!
//! TypeScript types: every DTO derives `ts_rs::TS` with
//! `export_to = "../../../apps/web/src/gen/types/"` (relative to ts-rs's default
//! `<crate>/bindings` dir). Files are written by `cargo test -p fa-domain`.

pub mod api;
pub mod authz;
pub mod calendar;
pub mod dashboard;
pub mod entities;
pub mod feeds;
pub mod ids;
pub mod mail;
pub mod mail_api;
pub mod notify;
pub mod oauth;
pub mod pipeline;
pub mod routines;
pub mod system_api;
pub mod tasks;
pub mod tasks_api;
pub mod theme;
pub mod timefmt;
pub mod validation;

pub use api::*;
pub use authz::*;
pub use dashboard::*;
pub use entities::*;
pub use feeds::*;
pub use ids::*;
pub use mail::*;
pub use mail_api::*;
pub use oauth::*;
pub use pipeline::*;
pub use system_api::*;
pub use tasks_api::*;
pub use theme::*;
pub use validation::*;
