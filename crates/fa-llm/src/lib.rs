//! fa-llm: System 2 client over an OpenAI-compatible endpoint (M4, D-093).
//!
//! Only `provider.rs` touches the database (endpoint lookup and `llm_call` metrics).

mod client;
mod provider;
mod schema;

pub use client::{
    Dialect, EndpointConfig, ExtractRequest, Extracted, LlmClient, LlmError, Ping,
    THINKING_EXTRA_TOKENS, default_http,
};
pub use provider::Provider;
pub use schema::json_schema_for;
