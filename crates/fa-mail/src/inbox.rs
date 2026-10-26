//! The approval inbox: events, bills, suggested email tasks and contact suggestions.
//! Events and tasks are approved by fa-calendar and fa-tasks; bills and contacts here.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use fa_domain::{
    BillId, BillPatch, BillPatchStatus, BillStatus, ContactApproval, ContactRole,
    ContactSuggestionId, ContactSuggestionView, DecisionPointKey, EventProposalView, HouseholdId,
    InboxCounts, InboxItem, InboxKind, InboxView, MailSource, MemberId, MessageId, OrgContactView,
    Subject, SubjectType, SuggestionStatus, TaskOrigin, TaskStatus, TaskStepView, TaskView,
};
use fa_pipeline::Pipeline;
use fa_store::{
    BillRepo, ChangeBus, ContactSuggestionRepo, EventRepo, InboxRepo, MailSourceRepo, MessageRepo,
    OrgContactRepo, OrgRepo, Store, TaskFilter, TaskRepo, TaskRow,
};
use sqlx::PgConnection;

use crate::context::Viewer;
use crate::process::views::{bill_view, external_url};
use crate::process::{create_contact, enqueue_pay_task, link_to_enrollments, local_part};
use crate::{MailError, Result};

#[derive(Clone)]
pub struct InboxService {
    store: Store,
}

/// What the viewer may do with the inbox.
enum Access {
    Full {
        readable: Option<Vec<fa_domain::MailSourceId>>,
    },
    CountsOnly,
}

fn task_view(r: &TaskRow) -> TaskView {
    let t = &r.task.value;
    TaskView {
        id: t.id,
        list: t.list_id,
        list_name: r.list_name.clone(),
        definition: t.definition_id,
        title: t.title.clone(),
        notes: t.notes.clone(),
        due_at: t.due_at,
        visible_from: t.visible_from,
        status: t.status,
        origin: t.origin,
        priority_flag: t.priority_flag,
        calendar_display: t.calendar_display,
        assignees: r.assignees.clone(),
        assignee_away: false,
        steps: r
            .steps
            .iter()
            .map(|s| TaskStepView {
                id: s.id,
                ord: s.ord,
                text: s.text.clone(),
                done_at: s.done_at,
                done_by: s.done_by,
            })
            .collect(),
        claimed_by: t.claimed_by,
        claimed_until: t.claimed_until,
        completed_at: t.completed_at,
        completed_by: t.completed_by,
        undo_until: None,
        score: 0.0,
        version: r.task.version,
    }
}

/// Message links of inbox items: shown only for messages the viewer can read.
struct Links {
    sources: HashMap<fa_domain::MailSourceId, MailSource>,
    readable: Option<Vec<fa_domain::MailSourceId>>,
    cache: HashMap<MessageId, (bool, Option<String>)>,
}

impl Links {
    async fn get(
        &mut self,
        conn: &mut PgConnection,
        id: Option<MessageId>,
    ) -> Result<(Option<MessageId>, Option<String>)> {
        let Some(id) = id else {
            return Ok((None, None));
        };
        if !self.cache.contains_key(&id) {
            let entry = match MessageRepo::get(conn, id).await? {
                Some(m) => {
                    let src = self.sources.get(&m.source_id);
                    let ok = match (&self.readable, src) {
                        (Some(r), Some(_)) => r.contains(&m.source_id),
                        _ => false,
                    };
                    (ok, src.and_then(|s| external_url(s, &m.remote_id)))
                }
                None => (false, None),
            };
            self.cache.insert(id, entry);
        }
        let (ok, url) = &self.cache[&id];
        Ok((ok.then_some(id), if *ok { url.clone() } else { None }))
    }
}

impl InboxService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    async fn access(conn: &mut PgConnection, viewer: Viewer) -> Result<Access> {
        if let Some(m) = viewer.adult_member() {
            let readable = MailSourceRepo::readable_by(conn, m).await?;
            return Ok(Access::Full {
                readable: Some(readable),
            });
        }
        if viewer.device && viewer.elevated {
            return Ok(Access::Full { readable: None });
        }
        if viewer.device {
            return Ok(Access::CountsOnly);
        }
        Err(MailError::Forbidden)
    }

    pub async fn list(&self, household: HouseholdId, viewer: Viewer) -> Result<InboxView> {
        self.list_kind(household, viewer, None).await
    }

    /// Items newest first, optionally of one kind. A dashboard device without elevation gets the
    /// counts only. Message links are included only for messages the viewer can read.
    pub async fn list_kind(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        kind: Option<InboxKind>,
    ) -> Result<InboxView> {
        let mut tx = self.store.tx_for(household).await?;
        let access = Self::access(&mut tx, viewer).await?;
        let counts = InboxRepo::counts(&mut tx).await?;
        let Access::Full { readable } = access else {
            return Ok(InboxView {
                items: Vec::new(),
                counts,
            });
        };
        let want = |k: InboxKind| kind.is_none_or(|w| w == k);
        let sources: HashMap<_, _> = MailSourceRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .map(|(s, _)| (s.value.id, s.value))
            .collect();
        let mut stamped: Vec<(DateTime<Utc>, InboxItem)> = Vec::new();
        let mut links = Links {
            sources,
            readable,
            cache: HashMap::new(),
        };

        if want(InboxKind::Event) {
            for r in EventRepo::proposals(&mut tx).await? {
                let e = r.event;
                let (message, url) = links.get(&mut tx, e.message_id).await?;
                stamped.push((
                    e.event.created_at,
                    InboxItem::Event {
                        event: EventProposalView {
                            id: e.event.id,
                            title: e.event.title.clone(),
                            start_at: e.event.start_at,
                            end_at: e.event.end_at,
                            all_day: e.event.all_day,
                            location: e.event.location.clone(),
                            confidence: e.event.confidence.unwrap_or(0.0),
                            suggested_calendar: e.target_calendar_id,
                            route_confidence: e.route_confidence,
                            pending_cancel: e.pending_cancel,
                            members: e.members.clone(),
                            message,
                            from_label: r.message_from.or(r.org_name),
                            external_url: url,
                            write_error: e.write_error.clone(),
                            version: e.version,
                        },
                    },
                ));
            }
        }
        if want(InboxKind::Bill) {
            for b in BillRepo::list(&mut tx, Some(BillStatus::Proposed)).await? {
                let (message, url) = links.get(&mut tx, b.value.message_id).await?;
                let mut view = bill_view(&mut tx, &b, url).await?;
                view.message = message;
                stamped.push((b.value.created_at, InboxItem::Bill { bill: view }));
            }
        }
        if want(InboxKind::Task) {
            let rows = TaskRepo::get_view_rows(
                &mut tx,
                &TaskFilter {
                    statuses: Some(vec![TaskStatus::Suggested]),
                    ..Default::default()
                },
            )
            .await?;
            for r in rows
                .iter()
                .filter(|r| r.task.value.origin == TaskOrigin::Email)
            {
                stamped.push((
                    r.task.value.created_at,
                    InboxItem::Task { task: task_view(r) },
                ));
            }
        }
        if want(InboxKind::Contact) {
            let orgs: HashMap<_, _> = OrgRepo::list(&mut tx, true)
                .await?
                .into_iter()
                .map(|o| (o.value.id, o.value.name))
                .collect();
            for s in ContactSuggestionRepo::list(&mut tx, Some(SuggestionStatus::Proposed)).await? {
                let (message, _) = links.get(&mut tx, s.message_id).await?;
                stamped.push((
                    s.created_at,
                    InboxItem::Contact {
                        contact: ContactSuggestionView {
                            id: s.id,
                            org: s.org_id,
                            org_name: orgs.get(&s.org_id).cloned().unwrap_or_default(),
                            email: s.email,
                            name: s.name,
                            member: s.member_id,
                            message,
                            created_at: s.created_at,
                        },
                    },
                ));
            }
        }
        stamped.sort_by_key(|a| std::cmp::Reverse(a.0));
        Ok(InboxView {
            items: stamped.into_iter().map(|(_, i)| i).collect(),
            counts,
        })
    }

    pub async fn counts(&self, household: HouseholdId) -> Result<InboxCounts> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(InboxRepo::counts(&mut tx).await?)
    }

    /// Opens a proposed bill (applying the edits in `patch`) and queues its pay task. A patch
    /// with `status: dismissed` rejects it instead. Approving an open bill again is a no-op.
    pub async fn approve_bill(
        &self,
        household: HouseholdId,
        id: BillId,
        patch: BillPatch,
        by: Option<MemberId>,
    ) -> Result<fa_domain::BillView> {
        if patch.status == Some(BillPatchStatus::Dismissed) {
            return self.reject_bill(household, id, by).await;
        }
        let mut tx = self.store.tx_for(household).await?;
        let mut b = BillRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        match b.value.status {
            BillStatus::Proposed => {}
            BillStatus::Open | BillStatus::Paid => {
                let v = self.view(&mut tx, &b).await?;
                return Ok(v);
            }
            BillStatus::Dismissed => return Err(MailError::Conflict),
        }
        if let Some(p) = patch.payee.filter(|p| !p.trim().is_empty()) {
            b.value.payee = p.trim().to_owned();
        }
        if let Some(a) = patch.amount_minor {
            if a <= 0 {
                return Err(MailError::Invalid("the amount must be positive".into()));
            }
            b.value.amount_minor = a;
        }
        if let Some(c) = patch.currency.filter(|c| !c.trim().is_empty()) {
            b.value.currency = c.trim().to_uppercase();
        }
        if let Some(d) = patch.due_on {
            b.value.due_on = d;
        }
        if let Some(a) = patch.account_hint {
            b.value.account_hint = Some(a).filter(|a| !a.trim().is_empty());
        }
        b.value.status = BillStatus::Open;
        BillRepo::update(&mut tx, &b.value, b.version).await?;
        if let Some(m) = b.value.message_id {
            record_is_bill(&mut tx, household, m, true, by).await?;
        }
        enqueue_pay_task(&mut tx, household, id).await?;
        publish_bills(&mut tx, household).await?;
        let b = BillRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let v = self.view(&mut tx, &b).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Dismisses a proposed bill and records that the message is not a bill.
    pub async fn reject_bill(
        &self,
        household: HouseholdId,
        id: BillId,
        by: Option<MemberId>,
    ) -> Result<fa_domain::BillView> {
        let mut tx = self.store.tx_for(household).await?;
        let b = BillRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        match b.value.status {
            BillStatus::Dismissed => {
                let v = self.view(&mut tx, &b).await?;
                return Ok(v);
            }
            BillStatus::Proposed => {}
            _ => return Err(MailError::Conflict),
        }
        BillRepo::set_status(&mut tx, id, BillStatus::Dismissed).await?;
        if let Some(m) = b.value.message_id {
            record_is_bill(&mut tx, household, m, false, by).await?;
        }
        publish_bills(&mut tx, household).await?;
        let b = BillRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let v = self.view(&mut tx, &b).await?;
        tx.commit().await?;
        Ok(v)
    }

    async fn view(
        &self,
        conn: &mut PgConnection,
        b: &fa_store::Versioned<fa_domain::Bill>,
    ) -> Result<fa_domain::BillView> {
        let url = match b.value.message_id {
            Some(m) => match MessageRepo::get(conn, m).await? {
                Some(msg) => MailSourceRepo::get(conn, msg.source_id)
                    .await?
                    .and_then(|s| external_url(&s.value, &msg.remote_id)),
                None => None,
            },
            None => None,
        };
        bill_view(conn, b, url).await
    }

    /// Creates the contact (or finds it) and links it to the member's enrollment at the org.
    pub async fn approve_contact(
        &self,
        household: HouseholdId,
        id: ContactSuggestionId,
        a: ContactApproval,
        by: Option<MemberId>,
    ) -> Result<OrgContactView> {
        let mut tx = self.store.tx_for(household).await?;
        let s = ContactSuggestionRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if s.status != SuggestionStatus::Proposed {
            return Err(MailError::Conflict);
        }
        let name = a
            .name
            .filter(|n| !n.trim().is_empty())
            .or(s.name.clone())
            .unwrap_or_else(|| local_part(&s.email));
        let role = a.role.unwrap_or(ContactRole::Other);
        let (cid, _) = create_contact(&mut tx, household, s.org_id, &name, &s.email, role)
            .await?
            .ok_or_else(|| {
                MailError::Invalid("this address belongs to an archived contact".into())
            })?;
        if let Some(member) = a.member.or(s.member_id) {
            link_to_enrollments(&mut tx, member, s.org_id, cid).await?;
        }
        ContactSuggestionRepo::set_status(&mut tx, id, SuggestionStatus::Accepted, by).await?;
        let c = OrgContactRepo::get(&mut tx, cid)
            .await?
            .ok_or(MailError::NotFound)?;
        publish_inbox(&mut tx, household).await?;
        tx.commit().await?;
        Ok(OrgContactView {
            id: c.value.id,
            org: c.value.org_id,
            name: c.value.name,
            email: c.value.email,
            role: c.value.role,
            version: c.version,
        })
    }

    /// Rejects the suggestion; the address is remembered, so it is not suggested again.
    pub async fn reject_contact(
        &self,
        household: HouseholdId,
        id: ContactSuggestionId,
        by: Option<MemberId>,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let s = ContactSuggestionRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        match s.status {
            SuggestionStatus::Rejected => return Ok(()),
            SuggestionStatus::Proposed => {}
            _ => return Err(MailError::Conflict),
        }
        ContactSuggestionRepo::set_status(&mut tx, id, SuggestionStatus::Rejected, by).await?;
        publish_inbox(&mut tx, household).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Stores the person's answer to "is this a bill?" and updates the message.
async fn record_is_bill(
    conn: &mut PgConnection,
    household: HouseholdId,
    message: MessageId,
    is_bill: bool,
    by: Option<MemberId>,
) -> Result<()> {
    Pipeline::record_human(
        conn,
        household,
        DecisionPointKey::EmailIsBill,
        Subject {
            kind: SubjectType::Message,
            id: message.as_uuid(),
        },
        serde_json::json!(is_bill),
        by,
    )
    .await?;
    MessageRepo::set_classification(conn, message, None, None, Some(is_bill)).await?;
    MessageRepo::set_learning_example(conn, message, true).await?;
    Ok(())
}

async fn publish_inbox(conn: &mut PgConnection, household: HouseholdId) -> Result<()> {
    ChangeBus::publish(conn, household, "inbox.updated", &serde_json::json!({})).await?;
    Ok(())
}

async fn publish_bills(conn: &mut PgConnection, household: HouseholdId) -> Result<()> {
    ChangeBus::publish(conn, household, "bills.updated", &serde_json::json!({})).await?;
    publish_inbox(conn, household).await
}
