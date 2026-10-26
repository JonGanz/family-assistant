//! Mail actions on the provider (D-058): archive, mark read and labels, each recorded as a
//! `mail_action` row and undoable for 30 days; plus the `mail.auto_actions{message}` job.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use fa_domain::mail_api::{MailActionInput, MailActionView};
use fa_domain::{
    AutoAction, HouseholdId, MailAction, MailActionId, MailActionKind, MailSourceId,
    MailSourceState, MemberId, Message, MessageId,
};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{
    CategoryRepo, ChangeBus, MailActionRepo, MailSourceRepo, MemberRepo, MessageRepo, NewMailAction,
};

use crate::provider::{self, LabelCache, MailProvider, Op, Target};
use crate::sync::{job_err, job_payload_id};
use crate::{MailDeps, MailError, Result};

pub const UNDO_DAYS: i64 = 30;

/// `FA/<name>`, with slashes inside the name flattened.
pub fn fa_label(name: &str) -> String {
    let n = name.trim();
    match n.strip_prefix("FA/") {
        Some(rest) => format!("FA/{}", rest.replace('/', "-")),
        None => format!("FA/{}", n.replace('/', "-")),
    }
}

pub fn view(a: &MailAction) -> MailActionView {
    MailActionView {
        id: a.id,
        action: a.action,
        label: a.label.clone(),
        automatic: a.automatic,
        done_by: a.done_by,
        done_at: a.done_at,
        undone_at: a.undone_at,
        undo_until: a.done_at + Duration::days(UNDO_DAYS),
        error: a.error.clone(),
    }
}

fn op_of(kind: MailActionKind, label: Option<&str>) -> Result<Op> {
    Ok(match kind {
        MailActionKind::Archive => Op::Archive,
        MailActionKind::MarkRead => Op::MarkRead,
        MailActionKind::Label => Op::Label(
            label
                .ok_or_else(|| MailError::Invalid("a label action needs a label".into()))?
                .to_owned(),
        ),
    })
}

fn inverse(op: &Op) -> Op {
    match op {
        Op::Archive => Op::Unarchive,
        Op::Unarchive => Op::Archive,
        Op::MarkRead => Op::MarkUnread,
        Op::MarkUnread => Op::MarkRead,
        Op::Label(l) => Op::Unlabel(l.clone()),
        Op::Unlabel(l) => Op::Label(l.clone()),
    }
}

pub struct ActionService {
    deps: Arc<MailDeps>,
    labels: Mutex<HashMap<MailSourceId, LabelCache>>,
}

struct Ready {
    msg: Message,
    provider: Box<dyn MailProvider>,
}

impl ActionService {
    pub fn new(deps: Arc<MailDeps>) -> Self {
        Self {
            deps,
            labels: Mutex::new(HashMap::new()),
        }
    }

    fn cache(&self, id: MailSourceId) -> Option<LabelCache> {
        self.labels
            .lock()
            .ok()
            .map(|mut m| m.entry(id).or_default().clone())
    }

    /// Loads the message, checks that its source may be written to and opens the provider.
    async fn ready(&self, household: HouseholdId, message: MessageId) -> Result<Ready> {
        let (msg, src) = {
            let mut tx = self.deps.store.tx_for(household).await?;
            let msg = MessageRepo::get(&mut tx, message)
                .await?
                .ok_or(MailError::NotFound)?;
            let src = MailSourceRepo::get(&mut tx, msg.source_id)
                .await?
                .ok_or(MailError::NotFound)?
                .value;
            (msg, src)
        };
        if src.read_only {
            return Err(MailError::ReadOnly);
        }
        if src.state == MailSourceState::NeedsReauth {
            return Err(MailError::NeedsReauth);
        }
        let (_, provider) =
            provider::open(&self.deps, household, src.id, self.cache(src.id)).await?;
        Ok(Ready { msg, provider })
    }

    fn target(m: &Message) -> Target {
        Target {
            remote_id: m.remote_id.clone(),
            folder: m.folder.clone(),
            rfc822_id: m.rfc822_id.clone(),
        }
    }

    /// Runs one operation and follows an IMAP move in the message row.
    async fn run_op(&self, household: HouseholdId, r: &mut Ready, op: &Op) -> Result<()> {
        let moved = r.provider.modify(&Self::target(&r.msg), op).await?;
        if let Some(m) = moved {
            let mut tx = self.deps.store.tx_for(household).await?;
            MessageRepo::relocate(&mut tx, r.msg.id, &m.remote_id, &m.folder).await?;
            tx.commit().await?;
            r.msg.remote_id = m.remote_id;
            r.msg.folder = Some(m.folder);
        }
        Ok(())
    }

    async fn publish(&self, household: HouseholdId, m: &Message) -> Result<()> {
        let mut tx = self.deps.store.tx_for(household).await?;
        ChangeBus::publish(
            &mut tx,
            household,
            "mail.updated",
            &serde_json::json!({"source": m.source_id, "message": m.id}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Applies several actions over one connection. A failing action is recorded with its error
    /// and the others still run; the first error is returned at the end.
    async fn apply_many(
        &self,
        household: HouseholdId,
        message: MessageId,
        items: Vec<(MailActionKind, Option<String>)>,
        by: Option<MemberId>,
        automatic: bool,
    ) -> Result<Vec<MailAction>> {
        let mut ready = self.ready(household, message).await?;
        let mut done = Vec::new();
        let mut first_err = None;
        for (kind, label) in items {
            let op = op_of(kind, label.as_deref())?;
            let prior = if automatic {
                let mut tx = self.deps.store.tx_for(household).await?;
                MailActionRepo::list_for_message(&mut tx, message)
                    .await?
                    .into_iter()
                    .find(|a| a.automatic && a.action == kind && a.label == label)
            } else {
                None
            };
            // An automatic action runs once; only a failed one is retried (an undone one stays).
            if prior.as_ref().is_some_and(|p| p.error.is_none()) {
                continue;
            }
            let res = self.run_op(household, &mut ready, &op).await;
            let error = res.as_ref().err().map(ToString::to_string);
            let mut tx = self.deps.store.tx_for(household).await?;
            let row = match prior {
                Some(p) => {
                    MailActionRepo::set_error(&mut tx, p.id, error.as_deref()).await?;
                    MailAction { error, ..p }
                }
                None => {
                    MailActionRepo::insert(
                        &mut tx,
                        &NewMailAction {
                            household_id: household,
                            message_id: message,
                            action: kind,
                            label,
                            automatic,
                            done_by: by,
                            error,
                        },
                    )
                    .await?
                }
            };
            tx.commit().await?;
            match res {
                Ok(()) => done.push(row),
                Err(e) => {
                    tracing::warn!(message = %message, "mail action failed");
                    first_err.get_or_insert(e);
                }
            }
        }
        ready.provider.close().await;
        self.publish(household, &ready.msg).await?;
        match first_err {
            Some(e) => Err(e),
            None => Ok(done),
        }
    }

    pub async fn apply(
        &self,
        household: HouseholdId,
        message: MessageId,
        input: MailActionInput,
        by: Option<MemberId>,
        automatic: bool,
    ) -> Result<MailActionView> {
        let label = match input.action {
            MailActionKind::Label => Some(fa_label(
                input
                    .label
                    .as_deref()
                    .filter(|l| !l.trim().is_empty())
                    .ok_or_else(|| MailError::Invalid("a label action needs a label".into()))?,
            )),
            _ => None,
        };
        let done = self
            .apply_many(
                household,
                message,
                vec![(input.action, label)],
                by,
                automatic,
            )
            .await?;
        done.first().map(view).ok_or(MailError::Conflict)
    }

    /// Reverses an action done within the last 30 days.
    pub async fn undo(
        &self,
        household: HouseholdId,
        action: MailActionId,
        by: Option<MemberId>,
    ) -> Result<MailActionView> {
        let a = {
            let mut tx = self.deps.store.tx_for(household).await?;
            MailActionRepo::get(&mut tx, action)
                .await?
                .ok_or(MailError::NotFound)?
        };
        if a.undone_at.is_some() {
            return Err(MailError::Conflict);
        }
        if a.error.is_some() {
            return Err(MailError::Invalid("the action did not complete".into()));
        }
        if Utc::now() > a.done_at + Duration::days(UNDO_DAYS) {
            return Err(MailError::Invalid(format!(
                "an action can be undone for {UNDO_DAYS} days"
            )));
        }
        let op = inverse(&op_of(a.action, a.label.as_deref())?);
        let mut ready = self.ready(household, a.message_id).await?;
        let res = self.run_op(household, &mut ready, &op).await;
        ready.provider.close().await;
        res?;
        let mut tx = self.deps.store.tx_for(household).await?;
        MailActionRepo::mark_undone(&mut tx, action, by).await?;
        let a = MailActionRepo::get(&mut tx, action)
            .await?
            .ok_or(MailError::NotFound)?;
        tx.commit().await?;
        self.publish(household, &ready.msg).await?;
        Ok(view(&a))
    }

    /// Applies the message's category `auto_actions` (labels `FA/<category>` and `FA/<member>`
    /// by default) when its source is writable.
    pub async fn auto_actions(&self, household: HouseholdId, message: MessageId) -> Result<()> {
        let items = {
            let mut tx = self.deps.store.tx_for(household).await?;
            let Some(msg) = MessageRepo::get(&mut tx, message).await? else {
                return Ok(());
            };
            let Some(src) = MailSourceRepo::get(&mut tx, msg.source_id).await? else {
                return Ok(());
            };
            let s = src.value;
            if s.read_only || s.archived_at.is_some() || s.state != MailSourceState::Ok {
                return Ok(());
            }
            let Some(key) = msg.category.as_deref() else {
                return Ok(());
            };
            let Some(cat) = CategoryRepo::get_by_key(&mut tx, key).await? else {
                return Ok(());
            };
            let cat = cat.value;
            let mut items: Vec<(MailActionKind, Option<String>)> = Vec::new();
            for a in &cat.auto_actions {
                match a {
                    AutoAction::Label => {
                        items.push((MailActionKind::Label, Some(fa_label(&cat.label))));
                        for (m, _, _) in MessageRepo::members(&mut tx, message).await? {
                            if let Some(member) = MemberRepo::get(&mut tx, m).await? {
                                items.push((
                                    MailActionKind::Label,
                                    Some(fa_label(&member.value.display_name)),
                                ));
                            }
                        }
                    }
                    AutoAction::Archive => items.push((MailActionKind::Archive, None)),
                    AutoAction::MarkRead => items.push((MailActionKind::MarkRead, None)),
                }
            }
            items
        };
        if items.is_empty() {
            return Ok(());
        }
        self.apply_many(household, message, items, None, true)
            .await
            .map(|_| ())
    }
}

// -------------------------------------------------------------------- jobs

struct AutoActions(Arc<ActionService>);

impl JobHandler for AutoActions {
    fn kind(&self) -> &'static str {
        "mail.auto_actions"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let message: MessageId = job_payload_id(&payload, "message")?;
            let household = match job_payload_id::<HouseholdId>(&payload, "household") {
                Ok(h) => h,
                Err(_) => {
                    let mut tx = ctx.store.tx_system().await?;
                    MessageRepo::household_of(&mut tx, message)
                        .await?
                        .ok_or_else(|| JobError::Handler("unknown message".into()))?
                }
            };
            self.0
                .auto_actions(household, message)
                .await
                .map_err(job_err)
        })
    }
}

pub fn handlers(deps: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(AutoActions(Arc::new(ActionService::new(
        deps.clone(),
    ))))]
}
