//! fa-domain: IDs, enums, entities, the authorization model and validation.
//!
//! TypeScript types: every DTO derives `ts_rs::TS` with
//! `export_to = "../../../apps/web/src/gen/types/"` (relative to ts-rs's default
//! `<crate>/bindings` dir). Files are written by `cargo test -p fa-domain`.

pub mod authz;
pub mod entities;
pub mod ids;
pub mod validation;

pub use authz::*;
pub use entities::*;
pub use ids::*;
pub use validation::*;
