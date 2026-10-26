//! Message facts for the rules, an `Evaluator` over `fa_rules::evaluate`, and small helpers.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use fa_domain::{
    AttachmentMeta, Candidate, DecisionPointKey, MailSourceId, Message, domain_matches, domain_of,
};
use fa_pipeline::Evaluator;
use fa_rules::{AttachmentFacts, Context, MessageFacts, OrgCtx};

/// Owned pieces of a stored message, borrowed as `MessageFacts`.
pub(crate) struct OwnedFacts {
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub headers: HashMap<String, String>,
    pub labels: Vec<String>,
    pub attachments: Vec<AttachmentFacts>,
    pub received_at: DateTime<Utc>,
    pub source: MailSourceId,
}

pub(crate) fn attachment_meta(m: &Message) -> Vec<AttachmentMeta> {
    m.attachments
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

impl OwnedFacts {
    pub fn new(m: &Message) -> Self {
        let headers = m
            .headers
            .as_ref()
            .and_then(|h| h.as_object())
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|v| (k.to_lowercase(), v.to_owned())))
                    .collect()
            })
            .unwrap_or_default();
        let attachments = attachment_meta(m)
            .into_iter()
            .map(|a| AttachmentFacts {
                part: a.part,
                filename: a.filename,
                mime: a.mime.to_lowercase(),
                size: usize::try_from(a.size).unwrap_or(0),
                ics: a.ics,
            })
            .collect();
        Self {
            from_addr: m.from_addr.clone(),
            from_name: m.from_name.clone(),
            to: m.to_addrs.clone(),
            cc: m.cc_addrs.clone(),
            subject: m.subject.clone(),
            body_text: m.body_text.clone().unwrap_or_default(),
            body_html: m.body_html.clone(),
            headers,
            labels: m.gmail_labels.clone(),
            attachments,
            received_at: m.received_at,
            source: m.source_id,
        }
    }

    pub fn facts(&self) -> MessageFacts<'_> {
        MessageFacts {
            from_addr: &self.from_addr,
            from_name: self.from_name.as_deref(),
            to: &self.to,
            cc: &self.cc,
            subject: &self.subject,
            body_text: &self.body_text,
            body_html: self.body_html.as_deref(),
            headers: &self.headers,
            gmail_labels: &self.labels,
            attachments: &self.attachments,
            received_at: self.received_at,
            source: self.source,
        }
    }
}

/// Rules-stage candidates from `fa_rules::evaluate`.
pub(crate) struct RulesEvaluator<'a> {
    pub facts: MessageFacts<'a>,
    pub ctx: &'a Context,
}

impl Evaluator for RulesEvaluator<'_> {
    fn candidates(&self, dp: DecisionPointKey) -> Vec<Candidate> {
        fa_rules::evaluate(dp, &self.facts, self.ctx)
    }
}

/// The org a sender belongs to: a contact address, then a listed sender, then a domain.
pub(crate) fn match_org<'a>(from_addr: &str, orgs: &'a [OrgCtx]) -> Option<&'a OrgCtx> {
    let from = from_addr.trim().to_lowercase();
    orgs.iter()
        .find(|o| {
            o.contacts
                .iter()
                .any(|c| c.email.eq_ignore_ascii_case(&from))
        })
        .or_else(|| {
            orgs.iter()
                .find(|o| o.senders.iter().any(|s| s.eq_ignore_ascii_case(&from)))
        })
        .or_else(|| {
            let d = domain_of(&from)?;
            orgs.iter()
                .find(|o| o.domains.iter().any(|od| domain_matches(d, od)))
        })
}

/// A stable short hash (FNV-1a, 64 bit) for idempotency keys.
pub(crate) fn short_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.trim().to_lowercase().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}
