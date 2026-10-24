//! fa-auth: passwords, login, sessions, device tokens, PIN elevation and actor resolution.
//!
//! Framework-agnostic: no axum types. `fa-api` adapts [`AuthService::resolve_actor`] to requests
//! and mounts [`PgSessionStore`] under `tower-sessions`.

mod config;
mod devices;
mod error;
mod limiter;
mod login;
mod password;
mod pin;
mod resolve;
mod service;
mod session_store;

pub use config::AuthConfig;
pub use devices::PairedDevice;
pub use error::{AuthError, Result};
pub use limiter::IpRateLimiter;
pub use login::{LoginOutcome, SessionInfo};
pub use password::{PasswordCheck, hash_password, verify_password};
pub use service::{AuthEvents, AuthService};
pub use session_store::PgSessionStore;
