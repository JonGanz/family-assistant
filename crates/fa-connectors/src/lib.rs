//! Thin, stateless HTTP clients returning neutral DTOs. No DB, no `fa-*` dependencies.

pub mod caldav;
pub mod dto;
pub mod error;
pub mod google;
pub mod http;
pub mod ics;
pub mod openmeteo;

pub use dto::*;
pub use error::ConnectorError;
pub use http::Http;
