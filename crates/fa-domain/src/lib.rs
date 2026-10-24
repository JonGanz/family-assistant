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
pub mod oauth;
pub mod validation;

pub use api::*;
pub use authz::*;
pub use dashboard::*;
pub use entities::*;
pub use feeds::*;
pub use ids::*;
pub use oauth::*;
pub use validation::*;
