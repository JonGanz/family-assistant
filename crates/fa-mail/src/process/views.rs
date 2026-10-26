//! DTO builders shared by the inbox, labelling and setup services.

use std::collections::HashMap;

use chrono::Duration;
use fa_domain::{
    Bill, BillStatus, BillView, DecisionPointKey, DecisionView, DerivedEvent, DerivedTask,
    HouseholdId, MailActionView, MailSource, MailSourceId, MemberId, Message, MessageDetail,
    MessageId, MessageSummary, SectionView, TaskStatus, message_external_url,
};
use fa_store::{
    BillRepo, DecisionPointRepo, DecisionRepo, EventRepo, MailActionRepo, MailSourceRepo,
    MessageRepo, TaskRepo, Versioned,
};
use sqlx::PgConnection;

use super::facts::OwnedFacts;
use crate::context::{self, Viewer};
use crate::{MailError, Result};

/// Mail actions can be undone for this long (D-058).
pub const UNDO_DAYS: i64 = 30;

/// The decision points a message is labelled on.
pub const EMAIL_POINTS: [DecisionPointKey; 4] = [
    DecisionPointKey::EmailCategory,
    DecisionPointKey::EmailTimeSensitive,
    DecisionPointKey::EmailMemberLink,
    DecisionPointKey::EmailIsBill,
];

pub fn external_url(source: &MailSource, remote_id: &str) -> Option<String> {
    message_external_url(
        source.provider,
        source.gmail_ext,
        &source.address,
        remote_id,
    )
}

pub fn summary(m: &Message, members: Vec<MemberId>, source: Option<&MailSource>) -> MessageSummary {
    MessageSummary {
        id: m.id,
        source: m.source_id,
        from_addr: m.from_addr.clone(),
        from_name: m.from_name.clone(),
        subject: m.subject.clone(),
        snippet: m.snippet.clone(),
        received_at: m.received_at,
        category: m.category.clone(),
        time_sensitive: m.time_sensitive,
        is_bill: m.is_bill,
        members,
        processing_state: m.processing_state,
        has_attachments: m.has_attachments,
        in_eval_sample: m.in_eval_sample,
        external_url: source.and_then(|s| external_url(s, &m.remote_id)),
    }
}

/// Non-archived sources of the household by id.
pub async fn sources_by_id(conn: &mut PgConnection) -> Result<HashMap<MailSourceId, MailSource>> {
    Ok(MailSourceRepo::list(conn, false)
        .await?
        .into_iter()
        .map(|(s, _)| (s.value.id, s.value))
        .collect())
}

/// Sources the viewer may read. Only adult members may; children and devices never list mail.
pub async fn readable_sources(
    conn: &mut PgConnection,
    viewer: Viewer,
) -> Result<Vec<MailSourceId>> {
    let member = viewer.adult_member().ok_or(MailError::Forbidden)?;
    Ok(MailSourceRepo::readable_by(conn, member).await?)
}

pub async fn bill_view(
    conn: &mut PgConnection,
    b: &Versioned<Bill>,
    url: Option<String>,
) -> Result<BillView> {
    let v = &b.value;
    let task_done = match v.task_id {
        Some(t) => TaskRepo::get(conn, t)
            .await?
            .is_some_and(|t| t.value.status == TaskStatus::Done),
        None => false,
    };
    Ok(BillView {
        id: v.id,
        payee: v.payee.clone(),
        amount_minor: v.amount_minor,
        currency: v.currency.clone(),
        due_on: v.due_on,
        account_hint: v.account_hint.clone(),
        status: v.status,
        confidence: v.confidence,
        task: v.task_id,
        task_done,
        message: v.message_id,
        external_url: url,
        version: b.version,
    })
}

/// Full message detail. The caller has checked that the viewer may read the source.
pub async fn detail(
    conn: &mut PgConnection,
    household: HouseholdId,
    id: MessageId,
    member: Option<MemberId>,
    can_act: bool,
) -> Result<MessageDetail> {
    let m = MessageRepo::get_with_content(conn, id)
        .await?
        .ok_or(MailError::NotFound)?;
    let source = MailSourceRepo::get(conn, m.source_id)
        .await?
        .map(|s| s.value);
    let members: Vec<MemberId> = MessageRepo::members(conn, id)
        .await?
        .into_iter()
        .map(|m| m.0)
        .collect();

    let mut sections = Vec::new();
    if m.content_purged_at.is_none() {
        let of = OwnedFacts::new(&m);
        let facts = of.facts();
        let secs = fa_rules::sections(&facts);
        let pc = match member {
            Some(_) => Some(context::load(conn, household, source.as_ref()).await?),
            None => None,
        };
        let target = member
            .zip(pc.as_ref())
            .and_then(|(id, pc)| pc.member(id).map(|mc| (mc, &pc.rules)));
        for s in secs {
            let relevance = target.map(|(mc, ctx)| fa_rules::relevance(&s, mc, ctx));
            sections.push(SectionView {
                title: s.title,
                text: s.text,
                relevance,
            });
        }
    }

    let subject = fa_domain::Subject {
        kind: fa_domain::SubjectType::Message,
        id: id.as_uuid(),
    };
    let mut decisions = Vec::new();
    for dp in EMAIL_POINTS {
        let Some(point) = DecisionPointRepo::get_by_key(conn, dp).await? else {
            continue;
        };
        let rows = if dp == DecisionPointKey::EmailMemberLink {
            DecisionRepo::latest_batch(conn, point.id, subject).await?
        } else {
            match DecisionRepo::latest_final(conn, point.id, subject).await? {
                Some(d) => vec![d],
                None => DecisionRepo::latest(conn, point.id, subject)
                    .await?
                    .into_iter()
                    .collect(),
            }
        };
        for d in rows {
            decisions.push(DecisionView {
                decision_point: dp,
                value: d.value,
                confidence: d.confidence,
                stage: d.stage_kind,
                is_final: d.is_final,
                source: d.source,
                decided_by: d.decided_by,
                at: d.created_at,
            });
        }
    }

    let events = EventRepo::derived_for_message(conn, id)
        .await?
        .into_iter()
        .map(|e| DerivedEvent {
            id: e.event.id,
            title: e.event.title,
            start_at: e.event.start_at,
            status: e.event.status.unwrap_or(fa_domain::EventStatus::Extracted),
        })
        .collect();
    let tasks = TaskRepo::derived_for_message(conn, id)
        .await?
        .into_iter()
        .map(|(id, title, status)| DerivedTask { id, title, status })
        .collect();
    let url = source.as_ref().and_then(|s| external_url(s, &m.remote_id));
    let mut bills = Vec::new();
    for b in BillRepo::for_message(conn, id).await? {
        if b.value.status != BillStatus::Dismissed {
            bills.push(bill_view(conn, &b, url.clone()).await?);
        }
    }
    let actions = MailActionRepo::list_for_message(conn, id)
        .await?
        .into_iter()
        .map(|a| MailActionView {
            id: a.id,
            action: a.action,
            label: a.label,
            automatic: a.automatic,
            done_by: a.done_by,
            done_at: a.done_at,
            undone_at: a.undone_at,
            undo_until: a.done_at + Duration::days(UNDO_DAYS),
            error: a.error,
        })
        .collect();

    Ok(MessageDetail {
        summary: summary(&m, members, source.as_ref()),
        body_text: m.body_text.clone(),
        content_purged: m.content_purged_at.is_some(),
        sections,
        decisions,
        events,
        tasks,
        bills,
        actions,
        can_act,
    })
}
