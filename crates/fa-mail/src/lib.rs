//! fa-mail (M3): mail sources and sync (Gmail API and IMAP), processing through the decision
//! pipelines, extraction hand-offs, the approval inbox, mail actions, retention and labelling.
//! See arch/Ingest.md. Module ownership: plan/M3.md "Mail service".

// T-09 (sync side)
pub mod actions;
pub mod attachments;
pub mod provider;
pub mod retention;
pub mod sources;
pub mod sync;
// T-10 (processing side)
pub mod context;
pub mod inbox;
pub mod labelling;
pub mod process;
pub mod setup;

mod error;

use std::sync::Arc;

use fa_connectors::Http;
use fa_domain::AccessTokenProvider;
use fa_jobs::JobHandler;
use fa_store::{MasterKey, Store};

pub use error::{MailError, Result};

/// Process configuration injected by fa-server (plan/M3.md "Process config").
#[derive(Debug, Clone)]
pub struct MailConfig {
    /// `FA_GOOGLE_API_BASE`, default `https://www.googleapis.com`.
    pub gmail_api_base: String,
    /// `FA_ALLOW_PLAIN_IMAP`: tests and e2e only.
    pub allow_plain_imap: bool,
    /// `FA_IMAP_TLS_INSECURE`: accept self-signed certificates (tests only).
    pub imap_tls_insecure: bool,
    /// `FA_ALLOW_PRIVATE_HOSTS`: the server builds `MailDeps::http` with
    /// `Http::with_policy(_, allow_private_hosts)` (C-05).
    pub allow_private_hosts: bool,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            gmail_api_base: fa_connectors::google::DEFAULT_API_BASE.to_owned(),
            allow_plain_imap: false,
            imap_tls_insecure: false,
            allow_private_hosts: false,
        }
    }
}

/// Everything the mail services and jobs need.
#[derive(Clone)]
pub struct MailDeps {
    pub store: Store,
    pub http: Http,
    pub tokens: Arc<dyn AccessTokenProvider>,
    pub key: Arc<MasterKey>,
    pub cfg: MailConfig,
}

/// All fa-mail job handlers.
pub fn handlers(deps: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    let mut h = sync::handlers(deps);
    h.extend(process::handlers(deps));
    h
}

/// Cron schedules (kind, cron expression).
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    let mut s = sync::default_schedules();
    s.extend(process::default_schedules());
    s
}
