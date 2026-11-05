//! fa-llm: System 2 client over an OpenAI-compatible endpoint (M4, D-093).
//!
//! No database access here; the `Provider` (T-06) lives in `provider.rs`.

mod client;
mod schema;

pub use client::{
    Dialect, EndpointConfig, ExtractRequest, Extracted, LlmClient, LlmError, Ping, default_http,
};
pub use schema::json_schema_for;

// T-06 adds here: `mod provider; pub use provider::Provider;`
