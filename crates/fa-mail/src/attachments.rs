//! PDF bytes for bill extraction: re-read the raw message through its source (PDFs are never
//! stored).

use std::sync::Arc;

use async_trait::async_trait;
use fa_domain::Message;

use crate::MailDeps;
use crate::process::AttachmentSource;
use crate::provider::{self, Target};

/// Re-reads the raw message through its source and returns the PDF parts (never stored).
pub struct ProviderAttachments {
    deps: Arc<MailDeps>,
}

impl ProviderAttachments {
    pub fn new(deps: Arc<MailDeps>) -> Self {
        Self { deps }
    }
}

#[async_trait]
impl AttachmentSource for ProviderAttachments {
    async fn pdf_attachments(&self, message: &Message) -> Vec<(String, Vec<u8>)> {
        let opened =
            provider::open(&self.deps, message.household_id, message.source_id, None).await;
        let (_, mut p) = match opened {
            Ok(o) => o,
            Err(e) => {
                tracing::debug!(error = %e, "cannot open the source for a PDF fetch");
                return Vec::new();
            }
        };
        let raw = p
            .fetch_raw(&Target {
                remote_id: message.remote_id.clone(),
                folder: message.folder.clone(),
                rfc822_id: message.rfc822_id.clone(),
            })
            .await;
        p.close().await;
        let raw = match raw {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, "cannot re-fetch a message for its PDF");
                return Vec::new();
            }
        };
        match fa_rules::parse(&raw) {
            Ok(parsed) => parsed
                .attachments
                .into_iter()
                .filter_map(|a| a.pdf.map(|bytes| (a.part, bytes)))
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}
