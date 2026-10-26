//! fa-rules (M3): deterministic signals, user rules and extractors over parsed mail.
//! See arch/Ingest.md sections 6 and 7. Pure: no DB, no network.
//!
//! Candidate values by decision point: `email.category` a category key string,
//! `email.time_sensitive` and `email.is_bill` a bool, `email.member_link` one candidate per
//! member whose value is the member id as a string.

mod actions;
mod bills;
mod dates;
mod dedupe;
mod events;
mod parse;
mod rules;
mod sections;
mod signals;
mod text;
mod types;

pub use actions::{ActionKind, ContactHint, ExtractedAction, contact_hint, extract_actions};
pub use bills::{ExtractedBill, extract_bills, pdf_text};
pub use dedupe::{event_dedupe_key, title_similarity};
pub use events::{EventSource, ExtractedEvent, extract_events};
pub use parse::{ParsedAttachment, ParsedMessage, parse};
pub use rules::{RuleError, validate_rule};
pub use sections::{Section, relevance, sections};
pub use signals::{SignalDef, evaluate, signals};
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("message could not be parsed")]
    Parse,
    #[error("pdf: {0}")]
    Pdf(String),
}

pub type Result<T> = std::result::Result<T, Error>;
