//! Extraction hand-offs of `mail.process`: events, bills, tasks and contact hints.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fa_domain::{
    Bill, BillId, BillStatus, Category, ContactMode, ContactRole, ContactSuggestion,
    ContactSuggestionId, EmailTasksMode, HouseholdId, MailSource, MemberId, Message, NotifyTrigger,
    OrgContact, OrgContactId, bill_dedupe_key, message_external_url,
};
use fa_rules::{ActionKind, OrgCtx};
use fa_store::{
    BillRepo, ContactSuggestionRepo, EnrollmentRepo, EventRepo, JobRepo, MessageRepo,
    NewExtractedEvent, NewJob, NotificationRepo, OrgContactRepo,
};
use sqlx::PgConnection;

use super::facts::{OwnedFacts, attachment_meta, match_org, short_hash, truncate};
use crate::Result;
use crate::context::ProcessCtx;

/// Bills at or above this confidence are opened without asking (Ingest.md section 9).
pub const BILL_OPEN_THRESHOLD: f32 = 0.8;
const PDF_TIMEOUT: Duration = Duration::from_secs(10);
const PDF_MAX_TEXT: usize = 20_000;

/// Supplies the raw PDF attachments of a message when their text is not stored yet (the sync side
/// keeps no attachment bytes). Wired in by the server; without one only stored `pdf_text` is used.
#[async_trait]
pub trait AttachmentSource: Send + Sync {
    /// `(attachment part id, bytes)` of each PDF of the message, up to 5 MB each.
    async fn pdf_attachments(&self, message: &Message) -> Vec<(String, Vec<u8>)>;
}

/// PDF text in a blocking task with a 10 s timeout; `None` on any failure.
pub async fn pdf_to_text(bytes: Vec<u8>) -> Option<String> {
    let task = tokio::task::spawn_blocking(move || fa_rules::pdf_text(&bytes, PDF_MAX_TEXT));
    match tokio::time::timeout(PDF_TIMEOUT, task).await {
        Ok(Ok(Ok(text))) if !text.trim().is_empty() => Some(text),
        Ok(Ok(Err(e))) => {
            tracing::debug!(error = %e, "pdf text extraction failed");
            None
        }
        Ok(_) => None,
        Err(_) => {
            tracing::warn!("pdf text extraction timed out");
            None
        }
    }
}

pub(crate) async fn enqueue(
    conn: &mut PgConnection,
    household: HouseholdId,
    kind: &str,
    payload: serde_json::Value,
    key: String,
) -> Result<bool> {
    let mut job = NewJob::new(kind);
    job.household_id = Some(household);
    job.payload = payload;
    job.idempotency_key = Some(key);
    Ok(JobRepo::enqueue(conn, &job).await?.is_some())
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Emitted {
    pub events: usize,
    pub bills_open: usize,
    pub bills_proposed: usize,
    pub tasks: usize,
    pub contacts_suggested: usize,
    pub contacts_created: usize,
}

/// One message's extraction run.
pub(crate) struct Extractor<'a> {
    pub conn: &'a mut PgConnection,
    pub household: HouseholdId,
    pub msg: &'a Message,
    pub source: &'a MailSource,
    pub pc: &'a ProcessCtx,
    pub of: &'a OwnedFacts,
    pub category: &'a Category,
    pub is_bill: Option<bool>,
    pub members: &'a [MemberId],
    pub attachments: Option<&'a Arc<dyn AttachmentSource>>,
}

impl Extractor<'_> {
    fn external_url(&self) -> Option<String> {
        message_external_url(
            self.source.provider,
            self.source.gmail_ext,
            &self.source.address,
            &self.msg.remote_id,
        )
    }

    fn source_ref(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "email",
            "message_id": self.msg.id.to_string(),
            "external_url": self.external_url(),
        })
    }

    fn org(&self) -> Option<&OrgCtx> {
        match_org(&self.of.from_addr, &self.pc.rules.orgs)
    }

    pub async fn events(&mut self, now: DateTime<Utc>, out: &mut Emitted) -> Result<()> {
        let tz = self.pc.tz();
        let org = self.org().map(|o| o.id);
        let mut seen: HashSet<String> = EventRepo::derived_for_message(self.conn, self.msg.id)
            .await?
            .into_iter()
            .map(|e| e.event.dedupe_key)
            .collect();
        for e in fa_rules::extract_events(&self.of.facts(), &self.pc.rules) {
            let end = e.end.unwrap_or(e.start);
            if end < now {
                continue;
            }
            let key = fa_rules::event_dedupe_key(&e.title, e.start, e.all_day, tz);
            if !seen.insert(key.clone()) {
                continue;
            }
            let (start_date, end_date, end_at) = if e.all_day {
                let s = e.start.with_timezone(&tz).date_naive();
                let mut en = end.with_timezone(&tz).date_naive();
                if en <= s {
                    en = s + chrono::Days::new(1);
                }
                (Some(s), Some(en), end.max(e.start))
            } else {
                let end_at = e.end.unwrap_or(e.start + chrono::Duration::hours(1));
                (None, None, end_at.max(e.start))
            };
            let id = EventRepo::insert_extracted(
                self.conn,
                &NewExtractedEvent {
                    household_id: self.household,
                    message_id: Some(self.msg.id),
                    org_id: org,
                    title: e.title.clone(),
                    description: None,
                    location: e.location.clone(),
                    start_at: e.start,
                    end_at,
                    all_day: e.all_day,
                    start_date,
                    end_date,
                    source_tz: Some(tz.name().to_owned()),
                    cancelled: e.cancel,
                    confidence: e.confidence,
                    source_ref: Some(self.source_ref()),
                    dedupe_key: key,
                    remote_uid: e.uid.clone(),
                    members: self.members.to_vec(),
                },
            )
            .await?;
            enqueue(
                self.conn,
                self.household,
                "calendar.route",
                serde_json::json!({"household": self.household, "event": id}),
                format!("calendar.route:{id}"),
            )
            .await?;
            out.events += 1;
        }
        Ok(())
    }

    /// Stored `pdf_text`, else the PDFs from the attachment source (stored back into `attachments`).
    async fn pdf_text(&mut self) -> Result<Option<String>> {
        let mut metas = attachment_meta(self.msg);
        let mut texts: Vec<String> = metas.iter().filter_map(|a| a.pdf_text.clone()).collect();
        if texts.is_empty()
            && let Some(src) = self.attachments
        {
            let mut changed = false;
            for (part, bytes) in src.pdf_attachments(self.msg).await {
                if let Some(t) = pdf_to_text(bytes).await {
                    if let Some(m) = metas.iter_mut().find(|m| m.part == part) {
                        m.pdf_text = Some(t.clone());
                        changed = true;
                    }
                    texts.push(t);
                }
            }
            if changed && let Ok(v) = serde_json::to_value(&metas) {
                MessageRepo::attachments_update(self.conn, self.msg.id, &v).await?;
            }
        }
        Ok((!texts.is_empty()).then(|| texts.join("\n")))
    }

    pub async fn bills(&mut self, out: &mut Emitted) -> Result<()> {
        if self.is_bill != Some(true) {
            return Ok(());
        }
        let currency = self.pc.household.currency.clone();
        let org = self.org().cloned();
        let mut found = fa_rules::extract_bills(&self.of.facts(), org.as_ref(), None, &currency);
        if found.is_empty()
            && let Some(text) = self.pdf_text().await?
        {
            found = fa_rules::extract_bills(&self.of.facts(), org.as_ref(), Some(&text), &currency);
        }
        for b in found {
            let open = b.confidence >= BILL_OPEN_THRESHOLD;
            let bill = Bill {
                id: BillId::new(),
                household_id: self.household,
                message_id: Some(self.msg.id),
                org_id: org.as_ref().map(|o| o.id),
                payee: b.payee.clone(),
                amount_minor: b.amount_minor,
                currency: b.currency.clone(),
                due_on: b.due_on,
                account_hint: b.account_hint.clone(),
                statement_period: None,
                status: if open {
                    BillStatus::Open
                } else {
                    BillStatus::Proposed
                },
                confidence: b.confidence,
                task_id: None,
                source_ref: self.source_ref(),
                dedupe_key: bill_dedupe_key(&b.payee, b.amount_minor, b.due_on),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let Some(created) = BillRepo::insert_if_new(self.conn, &bill).await? else {
                continue;
            };
            if open {
                enqueue_pay_task(self.conn, self.household, bill.id).await?;
                out.bills_open += 1;
            } else {
                NotificationRepo::insert_for_adults(
                    self.conn,
                    NotifyTrigger::ApprovalNeeded,
                    "bill",
                    Some(bill.id.as_uuid()),
                    "A bill needs your confirmation",
                    &format!(
                        "{}: please confirm the amount",
                        truncate(&created.value.payee, 80)
                    ),
                    Some("/inbox"),
                    &format!("approval:bill:{}", bill.id),
                )
                .await?;
                out.bills_proposed += 1;
            }
        }
        Ok(())
    }

    pub async fn tasks(&mut self, out: &mut Emitted) -> Result<()> {
        let mode = match self.category.email_tasks {
            EmailTasksMode::Off => return Ok(()),
            EmailTasksMode::Suggest => "suggest",
            EmailTasksMode::Create => "create",
        };
        let owner_adults: Vec<MemberId> = self
            .pc
            .adults
            .iter()
            .copied()
            .filter(|a| self.pc.owners.contains(a))
            .collect();
        let reply_to = if owner_adults.is_empty() {
            self.pc.adults.clone()
        } else {
            owner_adults
        };
        let action_to = if self.members.is_empty() {
            self.pc.owners.clone()
        } else {
            self.members.to_vec()
        };
        for a in fa_rules::extract_actions(&self.of.facts(), &self.pc.rules) {
            let (kind, assignees) = match a.kind {
                ActionKind::Action => ("action", &action_to),
                ActionKind::Reply => ("reply", &reply_to),
            };
            let payload = serde_json::json!({
                "household": self.household,
                "message": self.msg.id,
                "kind": kind,
                "title": a.title,
                "due_at": a.due_at,
                "assignees": assignees,
                "mode": mode,
                "external_url": self.external_url(),
            });
            let key = format!("tasks.from_mail:{}:{}", self.msg.id, short_hash(&a.title));
            if enqueue(self.conn, self.household, "tasks.from_mail", payload, key).await? {
                out.tasks += 1;
            }
        }
        Ok(())
    }

    pub async fn contacts(&mut self, out: &mut Emitted) -> Result<()> {
        let Some(hint) = fa_rules::contact_hint(&self.of.facts(), &self.pc.rules) else {
            return Ok(());
        };
        let Some(org) = self.pc.org(hint.org) else {
            return Ok(());
        };
        match org.contact_mode {
            ContactMode::Manual => {}
            ContactMode::Suggest => {
                let s = ContactSuggestion {
                    id: ContactSuggestionId::new(),
                    household_id: self.household,
                    org_id: org.id,
                    email: hint.email.clone(),
                    name: hint.name.clone(),
                    member_id: Some(hint.member),
                    message_id: Some(self.msg.id),
                    status: fa_domain::SuggestionStatus::Proposed,
                    decided_by: None,
                    decided_at: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                if ContactSuggestionRepo::insert_if_new(self.conn, &s).await? {
                    NotificationRepo::insert_for_adults(
                        self.conn,
                        NotifyTrigger::ApprovalNeeded,
                        "contact",
                        Some(s.id.as_uuid()),
                        "A new contact needs your confirmation",
                        &format!("A new sender at {}", truncate(&org.name, 80)),
                        Some("/inbox"),
                        &format!("approval:contact:{}", s.id),
                    )
                    .await?;
                    out.contacts_suggested += 1;
                }
            }
            ContactMode::Auto => {
                let name = hint
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| local_part(&hint.email));
                let created = create_contact(
                    self.conn,
                    self.household,
                    org.id,
                    &name,
                    &hint.email,
                    ContactRole::Other,
                )
                .await?;
                if let Some((id, is_new)) = created {
                    link_to_enrollments(self.conn, hint.member, org.id, id).await?;
                    if is_new {
                        out.contacts_created += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn local_part(email: &str) -> String {
    email.split('@').next().unwrap_or(email).to_owned()
}

pub(crate) async fn enqueue_pay_task(
    conn: &mut PgConnection,
    household: HouseholdId,
    bill: BillId,
) -> Result<()> {
    enqueue(
        conn,
        household,
        "tasks.from_bill",
        serde_json::json!({"household": household, "bill": bill}),
        format!("tasks.from_bill:{bill}"),
    )
    .await?;
    Ok(())
}

/// Creates the contact, or finds the existing one. `(id, newly created)`; `None` when the address
/// exists only as an archived contact.
pub(crate) async fn create_contact(
    conn: &mut PgConnection,
    household: HouseholdId,
    org: fa_domain::OrgId,
    name: &str,
    email: &str,
    role: ContactRole,
) -> Result<Option<(OrgContactId, bool)>> {
    let c = OrgContact {
        id: OrgContactId::new(),
        household_id: household,
        org_id: org,
        name: name.to_owned(),
        email: email.to_lowercase(),
        role,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    if let Some(id) = OrgContactRepo::insert_if_new(conn, &c).await? {
        return Ok(Some((id, true)));
    }
    Ok(OrgContactRepo::find_in_org(conn, org, email)
        .await?
        .map(|id| (id, false)))
}

/// Links the contact to every enrollment the member has at the org.
pub(crate) async fn link_to_enrollments(
    conn: &mut PgConnection,
    member: MemberId,
    org: fa_domain::OrgId,
    contact: OrgContactId,
) -> Result<()> {
    for e in EnrollmentRepo::list(conn, Some(member)).await? {
        if e.value.org_id == org {
            EnrollmentRepo::add_contact(conn, e.value.id, contact).await?;
        }
    }
    Ok(())
}
