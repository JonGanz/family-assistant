//! Thin, stateless HTTP clients returning neutral DTOs. No DB, no `fa-*` dependencies.

pub mod caldav;
pub mod dto;
pub mod error;
pub mod google;
pub mod http;
pub mod ics;
pub mod imap;
pub mod ntfy;
pub mod openmeteo;

pub use dto::*;
pub use error::ConnectorError;
pub use google::gmail::{HistoryPage, Label, MsgRef, Profile, RawMessage};
pub use http::Http;
pub use imap::{
    Caps, Folder, FolderStatus, ImapClient, ImapConfig, ImapMessage, ImapSecurity, SpecialUse,
};
pub use ntfy::NtfyMessage;
