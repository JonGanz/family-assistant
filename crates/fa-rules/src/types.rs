use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use fa_domain::entities::Role;
use fa_domain::{
    CategoryBehaviour, Gender, GenderTerms, MailRule, MailSourceId, MemberId, OrgId, OrgKind,
};

/// A mail attachment as seen by rules and extractors.
#[derive(Debug, Clone, Default)]
pub struct AttachmentFacts {
    pub part: String,
    pub filename: Option<String>,
    /// Lower-case `type/subtype`.
    pub mime: String,
    pub size: usize,
    /// Whole text of a `text/calendar` part or `.ics` file.
    pub ics: Option<String>,
}

/// Everything the rules look at. `headers` keys are lower-case header names.
#[derive(Debug, Clone)]
pub struct MessageFacts<'a> {
    pub from_addr: &'a str,
    pub from_name: Option<&'a str>,
    pub to: &'a [String],
    pub cc: &'a [String],
    pub subject: &'a str,
    pub body_text: &'a str,
    pub body_html: Option<&'a str>,
    pub headers: &'a HashMap<String, String>,
    pub gmail_labels: &'a [String],
    pub attachments: &'a [AttachmentFacts],
    pub received_at: DateTime<Utc>,
    pub source: MailSourceId,
}

#[derive(Debug, Clone)]
pub struct ContactCtx {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrgCtx {
    pub id: OrgId,
    pub kind: OrgKind,
    pub name: String,
    pub domains: Vec<String>,
    pub senders: Vec<String>,
    pub contacts: Vec<ContactCtx>,
    pub bill_amount_regex: Option<String>,
    pub bill_due_regex: Option<String>,
    pub bill_account_regex: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnrollCtx {
    pub org: OrgId,
    pub grade: Option<String>,
    pub class_name: Option<String>,
    /// Contact e-mail addresses linked to this enrollment.
    pub contacts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MemberCtx {
    pub id: MemberId,
    pub name: String,
    pub aliases: Vec<String>,
    pub grade: Option<String>,
    pub gender: Option<Gender>,
    pub tags: Vec<String>,
    pub role: Role,
    pub enrollments: Vec<EnrollCtx>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub orgs: Vec<OrgCtx>,
    pub members: Vec<MemberCtx>,
    pub gender_terms: GenderTerms,
    pub rules: Vec<MailRule>,
    pub disabled_signals: HashSet<String>,
    /// Category keys that exist, with their behaviour.
    pub categories: Vec<(String, CategoryBehaviour)>,
    pub owners: Vec<MemberId>,
    pub tz: Tz,
}

impl Context {
    pub fn new(tz: Tz) -> Self {
        Self {
            orgs: Vec::new(),
            members: Vec::new(),
            gender_terms: GenderTerms::default(),
            rules: Vec::new(),
            disabled_signals: HashSet::new(),
            categories: Vec::new(),
            owners: Vec::new(),
            tz,
        }
    }
}

impl MessageFacts<'_> {
    /// Subject and body, for phrase scanning.
    pub(crate) fn text(&self) -> String {
        format!("{}\n\n{}", self.subject, self.body_text)
    }
}
