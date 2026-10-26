//! The labelling view (D-062): the queue, human decisions, the eval sample.

use fa_domain::{
    DecisionPointKey, EvalProgress, HouseholdId, LabelInput, LabelQueueItem, MailSource, MemberId,
    MessageDetail, MessageId, Subject, SubjectType,
};
use fa_pipeline::Pipeline;
use fa_store::{
    CategoryRepo, ChangeBus, MailSourceRepo, MemberRepo, MessageRepo, Store, mail_defaults,
};
use sqlx::PgConnection;

use crate::context::Viewer;
use crate::process::views::{self, readable_sources};
use crate::process::{EXTRACTION_STEPS, enqueue};
use crate::{MailError, Result};

/// Messages to put in the eval sample (D-028).
pub const EVAL_TARGET: i64 = 200;

#[derive(Clone)]
pub struct LabelService {
    store: Store,
}

fn subject(id: MessageId) -> Subject {
    Subject {
        kind: SubjectType::Message,
        id: id.as_uuid(),
    }
}

impl LabelService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// The source of a message the viewer may read; `NotFound` otherwise.
    async fn readable_source(
        conn: &mut PgConnection,
        viewer: Viewer,
        message: MessageId,
    ) -> Result<MailSource> {
        let readable = readable_sources(conn, viewer).await?;
        let msg = MessageRepo::get(conn, message)
            .await?
            .ok_or(MailError::NotFound)?;
        if !readable.contains(&msg.source_id) {
            return Err(MailError::NotFound);
        }
        Ok(MailSourceRepo::get(conn, msg.source_id)
            .await?
            .ok_or(MailError::NotFound)?
            .value)
    }

    pub async fn progress(&self, household: HouseholdId) -> Result<EvalProgress> {
        let mut tx = self.store.tx_for(household).await?;
        progress(&mut tx, household).await
    }

    /// The next message to label from the stratified queue, skipping `after`.
    pub async fn next(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        after: Option<MessageId>,
    ) -> Result<Option<LabelQueueItem>> {
        let mut tx = self.store.tx_for(household).await?;
        let sources = readable_sources(&mut tx, viewer).await?;
        let exclude: Vec<MessageId> = after.into_iter().collect();
        let ids = MessageRepo::label_queue(&mut tx, household, &sources, &exclude, 1).await?;
        let Some(id) = ids.first().copied() else {
            return Ok(None);
        };
        let source = Self::readable_source(&mut tx, viewer, id).await?;
        let message = views::detail(&mut tx, household, id, None, !source.read_only).await?;
        let progress = progress(&mut tx, household).await?;
        Ok(Some(LabelQueueItem { message, progress }))
    }

    /// Records the person's answer for one decision point, updates the message and, when a newly
    /// `extract` category (or a new "is a bill") calls for extraction, re-queues `mail.process`.
    pub async fn label(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        message: MessageId,
        input: LabelInput,
    ) -> Result<MessageDetail> {
        let member = viewer.adult_member().ok_or(MailError::Forbidden)?;
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        let source = Self::readable_source(&mut tx, viewer, message).await?;
        let msg = MessageRepo::get(&mut tx, message)
            .await?
            .ok_or(MailError::NotFound)?;
        let mut only: Option<Vec<&str>> = None;

        let value = match input.decision_point {
            DecisionPointKey::EmailCategory => {
                let key = input
                    .value
                    .as_str()
                    .ok_or_else(|| MailError::Invalid("the category must be a string".into()))?
                    .to_owned();
                let cat = CategoryRepo::get_by_key(&mut tx, &key)
                    .await?
                    .filter(|c| c.value.archived_at.is_none())
                    .ok_or_else(|| MailError::Invalid(format!("unknown category `{key}`")))?;
                let was_extract = match msg.category.as_deref() {
                    Some(old) => CategoryRepo::get_by_key(&mut tx, old)
                        .await?
                        .is_some_and(|c| {
                            c.value.behaviour == fa_domain::CategoryBehaviour::Extract
                        }),
                    None => false,
                };
                if cat.value.behaviour == fa_domain::CategoryBehaviour::Extract && !was_extract {
                    only = Some(EXTRACTION_STEPS.to_vec());
                }
                MessageRepo::set_classification(&mut tx, message, Some(&key), None, None).await?;
                serde_json::json!(key)
            }
            DecisionPointKey::EmailTimeSensitive => {
                let v = bool_value(&input.value)?;
                MessageRepo::set_classification(&mut tx, message, None, Some(v), None).await?;
                serde_json::json!(v)
            }
            DecisionPointKey::EmailIsBill => {
                let v = bool_value(&input.value)?;
                if v && msg.is_bill != Some(true) {
                    only = Some(vec!["bills"]);
                }
                MessageRepo::set_classification(&mut tx, message, None, None, Some(v)).await?;
                serde_json::json!(v)
            }
            DecisionPointKey::EmailMemberLink => {
                let known: Vec<MemberId> = MemberRepo::list(&mut tx, false)
                    .await?
                    .into_iter()
                    .map(|m| m.value.id)
                    .collect();
                let names: Vec<&str> = match &input.value {
                    serde_json::Value::Array(a) => a.iter().filter_map(|v| v.as_str()).collect(),
                    serde_json::Value::String(s) => vec![s.as_str()],
                    _ => return Err(MailError::Invalid("expected a list of member ids".into())),
                };
                let mut ids: Vec<MemberId> = Vec::new();
                for n in names {
                    let id: MemberId = n
                        .parse()
                        .map_err(|_| MailError::Invalid("bad member id".into()))?;
                    if !known.contains(&id) {
                        return Err(MailError::Invalid("unknown member".into()));
                    }
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                let links: Vec<_> = ids
                    .iter()
                    .map(|m| (*m, fa_domain::DecidedBy::User, 1.0_f32))
                    .collect();
                MessageRepo::set_members(&mut tx, message, &links).await?;
                serde_json::json!(ids)
            }
            DecisionPointKey::EventCalendarRoute => {
                return Err(MailError::Invalid(
                    "events are routed from the inbox, not labelled".into(),
                ));
            }
        };
        let decision = Pipeline::record_human(
            &mut tx,
            household,
            input.decision_point,
            subject(message),
            value,
            Some(member),
        )
        .await?;
        MessageRepo::set_learning_example(&mut tx, message, true).await?;
        if let Some(steps) = only {
            enqueue(
                &mut tx,
                household,
                "mail.process",
                serde_json::json!({"household": household, "message": message, "only": steps}),
                format!("mail.process:{message}:label:{decision}"),
            )
            .await?;
        }
        ChangeBus::publish(
            &mut tx,
            household,
            "mail.updated",
            &serde_json::json!({"source": msg.source_id}),
        )
        .await?;
        let detail = views::detail(&mut tx, household, message, None, !source.read_only).await?;
        tx.commit().await?;
        Ok(detail)
    }

    pub async fn set_eval_sample(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        message: MessageId,
        in_sample: bool,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        Self::readable_source(&mut tx, viewer, message).await?;
        MessageRepo::set_eval_sample(&mut tx, message, in_sample).await?;
        tx.commit().await?;
        Ok(())
    }
}

fn bool_value(v: &serde_json::Value) -> Result<bool> {
    v.as_bool()
        .ok_or_else(|| MailError::Invalid("expected true or false".into()))
}

async fn progress(conn: &mut PgConnection, household: HouseholdId) -> Result<EvalProgress> {
    let (in_sample, labelled) = MessageRepo::eval_progress(conn, household).await?;
    Ok(EvalProgress {
        in_sample,
        target: EVAL_TARGET,
        labelled,
    })
}
