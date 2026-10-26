//! Job `mail.process`: runs the email decision points through the pipeline, writes the message
//! columns and member links, and hands extracted events, bills, tasks and contacts to their
//! owners. Also job `inbox.expire`. See plan/M3.md "Mail service" (T-10 paragraph).

mod expire;
mod extract;
mod facts;
pub(crate) mod views;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use fa_domain::{
    Candidate, CategoryBehaviour, DecidedBy, DecisionPointKey, HouseholdId, MemberId, MessageId,
    NotifyTrigger, ProcessingState, Subject, SubjectType,
};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_pipeline::Pipeline;
use fa_store::{
    ChangeBus, DecisionPointRepo, DecisionRepo, MailSourceRepo, MessageRepo, NewNotification,
    NotificationRepo, Store, mail_defaults,
};
use serde::Deserialize;
use sqlx::PgConnection;

pub use expire::expire_all;
pub use extract::{AttachmentSource, BILL_OPEN_THRESHOLD, Emitted, pdf_to_text};
pub(crate) use extract::{
    create_contact, enqueue, enqueue_pay_task, link_to_enrollments, local_part,
};
pub(crate) use facts::{OwnedFacts, RulesEvaluator, short_hash, truncate};

use crate::context::{self, ProcessCtx};
use crate::{MailDeps, MailError, Result};

/// Extraction steps that a re-run after a label fix may be limited to.
pub const EXTRACTION_STEPS: [&str; 4] = ["events", "bills", "tasks", "contacts"];

#[derive(Debug, Clone, Copy, Default)]
struct Steps {
    category: bool,
    time_sensitive: bool,
    member_link: bool,
    is_bill: bool,
    events: bool,
    bills: bool,
    tasks: bool,
    contacts: bool,
}

impl Steps {
    fn all() -> Self {
        Self {
            category: true,
            time_sensitive: true,
            member_link: true,
            is_bill: true,
            events: true,
            bills: true,
            tasks: true,
            contacts: true,
        }
    }

    /// Accepts decision point keys (`email.category`), short names (`category`), `classify` and
    /// `extract`; unknown names are ignored.
    fn from_only(only: &[String]) -> Self {
        let mut s = Self::default();
        for o in only {
            match o.as_str() {
                "email.category" | "category" => s.category = true,
                "email.time_sensitive" | "time_sensitive" => s.time_sensitive = true,
                "email.member_link" | "member_link" => s.member_link = true,
                "email.is_bill" | "is_bill" => s.is_bill = true,
                "classify" => {
                    s.category = true;
                    s.time_sensitive = true;
                    s.member_link = true;
                    s.is_bill = true;
                }
                "events" => s.events = true,
                "bills" => s.bills = true,
                "tasks" => s.tasks = true,
                "contacts" => s.contacts = true,
                "extract" => {
                    s.events = true;
                    s.bills = true;
                    s.tasks = true;
                    s.contacts = true;
                }
                _ => {}
            }
        }
        s
    }
}

pub(crate) struct Decided {
    pub values: Vec<Candidate>,
    pub by: DecidedBy,
}

/// A person's final decision wins over the pipeline; otherwise the pipeline runs.
async fn decide(
    conn: &mut PgConnection,
    household: HouseholdId,
    dp: DecisionPointKey,
    subject: Subject,
    eval: &RulesEvaluator<'_>,
) -> Result<Decided> {
    let point = DecisionPointRepo::get_by_key(conn, dp)
        .await?
        .ok_or_else(|| MailError::Invalid(format!("decision point {} missing", dp.as_str())))?;
    if let Some(last) = DecisionRepo::latest(conn, point.id, subject).await?
        && last.stage_kind == fa_domain::StageKind::Human
        && last.is_final
    {
        let values = DecisionRepo::latest_batch(conn, point.id, subject)
            .await?
            .into_iter()
            .map(|d| Candidate {
                value: d.value,
                confidence: d.confidence,
                source: "human".into(),
            })
            .collect();
        return Ok(Decided {
            values,
            by: DecidedBy::User,
        });
    }
    let o = Pipeline::run(conn, household, dp, subject, eval).await?;
    Ok(Decided {
        values: o.values,
        by: if o.is_final {
            DecidedBy::Rule
        } else {
            DecidedBy::Default
        },
    })
}

/// Member ids named by decision values: ids, arrays of ids, `owners` (the source's owners) or
/// `household` (nobody).
pub(crate) fn resolve_members(pc: &ProcessCtx, d: &Decided) -> Vec<(MemberId, DecidedBy, f32)> {
    let mut out: Vec<(MemberId, DecidedBy, f32)> = Vec::new();
    let mut add = |id: MemberId, by: DecidedBy, conf: f32| {
        if pc.member(id).is_some() && !out.iter().any(|o| o.0 == id) {
            out.push((id, by, conf));
        }
    };
    for c in &d.values {
        let names: Vec<&str> = match &c.value {
            serde_json::Value::String(s) => vec![s.as_str()],
            serde_json::Value::Array(a) => a.iter().filter_map(|v| v.as_str()).collect(),
            _ => vec![],
        };
        for n in names {
            match n {
                "owners" => {
                    for o in &pc.owners {
                        add(*o, DecidedBy::Default, c.confidence);
                    }
                }
                "household" => {}
                s => {
                    if let Ok(id) = s.parse::<MemberId>() {
                        add(id, d.by, c.confidence);
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct Payload {
    message: MessageId,
    #[serde(default)]
    household: Option<HouseholdId>,
    #[serde(default)]
    only: Option<Vec<String>>,
}

/// Processes stored messages (job `mail.process`).
#[derive(Clone)]
pub struct ProcessService {
    store: Store,
    attachments: Option<Arc<dyn AttachmentSource>>,
    now: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
}

impl ProcessService {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            attachments: None,
            now: Arc::new(Utc::now),
        }
    }

    /// Replaces the clock (tests): events that already ended by then are not proposed.
    pub fn with_clock(mut self, now: impl Fn() -> DateTime<Utc> + Send + Sync + 'static) -> Self {
        self.now = Arc::new(now);
        self
    }

    /// Lets bill extraction fetch PDF bytes that are not stored.
    pub fn with_attachments(mut self, src: Arc<dyn AttachmentSource>) -> Self {
        self.attachments = Some(src);
        self
    }

    /// Runs the whole processing of `message`, or only the steps in `only`. Everything happens in
    /// one transaction, so a failure leaves nothing behind and a retry cannot duplicate items.
    pub async fn process(
        &self,
        household: HouseholdId,
        message: MessageId,
        only: Option<&[String]>,
    ) -> Result<Emitted> {
        let steps = only.map_or_else(Steps::all, Steps::from_only);
        let full = only.is_none();
        let mut out = Emitted::default();
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        let Some(msg) = MessageRepo::get_with_content(&mut tx, message).await? else {
            return Ok(out);
        };
        let source = MailSourceRepo::get(&mut tx, msg.source_id)
            .await?
            .ok_or(MailError::NotFound)?
            .value;
        let pc = context::load(&mut tx, household, Some(&source)).await?;
        let of = OwnedFacts::new(&msg);
        let eval = RulesEvaluator {
            facts: of.facts(),
            ctx: &pc.rules,
        };
        let subject = Subject {
            kind: SubjectType::Message,
            id: msg.id.as_uuid(),
        };

        let mut category = msg.category.clone();
        let mut time_sensitive = msg.time_sensitive;
        let mut is_bill = msg.is_bill;
        let mut members: Vec<MemberId> = MessageRepo::members(&mut tx, msg.id)
            .await?
            .into_iter()
            .map(|m| m.0)
            .collect();

        if steps.category {
            let d = decide(
                &mut tx,
                household,
                DecisionPointKey::EmailCategory,
                subject,
                &eval,
            )
            .await?;
            let key = d
                .values
                .first()
                .and_then(|c| c.value.as_str())
                .map(str::to_owned)
                .filter(|k| pc.categories.contains_key(k))
                .or_else(|| {
                    pc.categories
                        .contains_key("other")
                        .then(|| "other".to_owned())
                });
            category = key.or(category);
        }
        if steps.time_sensitive {
            let d = decide(
                &mut tx,
                household,
                DecisionPointKey::EmailTimeSensitive,
                subject,
                &eval,
            )
            .await?;
            time_sensitive = d
                .values
                .first()
                .and_then(|c| c.value.as_bool())
                .or(time_sensitive);
        }
        if steps.is_bill {
            let d = decide(
                &mut tx,
                household,
                DecisionPointKey::EmailIsBill,
                subject,
                &eval,
            )
            .await?;
            is_bill = d.values.first().and_then(|c| c.value.as_bool()).or(is_bill);
        }
        if steps.member_link {
            let d = decide(
                &mut tx,
                household,
                DecisionPointKey::EmailMemberLink,
                subject,
                &eval,
            )
            .await?;
            let links = resolve_members(&pc, &d);
            members = links.iter().map(|l| l.0).collect();
            MessageRepo::set_members(&mut tx, msg.id, &links).await?;
        }
        if steps.category || steps.time_sensitive || steps.is_bill {
            MessageRepo::set_classification(
                &mut tx,
                msg.id,
                steps.category.then_some(category.as_deref()).flatten(),
                steps.time_sensitive.then_some(time_sensitive).flatten(),
                steps.is_bill.then_some(is_bill).flatten(),
            )
            .await?;
        }

        let cat = category.as_deref().and_then(|k| pc.categories.get(k));
        if steps.time_sensitive
            && time_sensitive == Some(true)
            && let Some(cat) = cat
            && cat.notify_time_sensitive
        {
            for reader in pc.readers(&source) {
                NotificationRepo::insert_dedupe(
                    &mut tx,
                    &NewNotification {
                        household_id: household,
                        member_id: reader,
                        trigger: NotifyTrigger::MailTimeSensitive,
                        subject_type: "message".into(),
                        subject_id: Some(msg.id.as_uuid()),
                        title: format!("Time-sensitive: {}", truncate(&msg.subject, 100)),
                        body: format!(
                            "{} ({})",
                            msg.from_name.as_deref().unwrap_or(&msg.from_addr),
                            cat.label
                        ),
                        click_path: Some(format!("/mail/{}", msg.id)),
                        priority: 4,
                        dedupe_key: format!("mts:{}:{reader}", msg.id),
                        not_before: None,
                    },
                )
                .await?;
            }
        }

        let extractable =
            cat.filter(|c| c.behaviour == CategoryBehaviour::Extract && c.key != "junk");
        if let Some(cat) = extractable
            && (steps.events || steps.bills || steps.tasks || steps.contacts)
        {
            let mut ex = extract::Extractor {
                conn: &mut tx,
                household,
                msg: &msg,
                source: &source,
                pc: &pc,
                of: &of,
                category: cat,
                is_bill,
                members: &members,
                attachments: self.attachments.as_ref(),
            };
            if steps.events {
                ex.events((self.now)(), &mut out).await?;
            }
            if steps.bills {
                ex.bills(&mut out).await?;
            }
            if steps.tasks {
                ex.tasks(&mut out).await?;
            }
            if steps.contacts {
                ex.contacts(&mut out).await?;
            }
        }

        if steps.category {
            enqueue(
                &mut tx,
                household,
                "mail.auto_actions",
                serde_json::json!({"household": household, "message": msg.id}),
                format!("mail.auto_actions:{}", msg.id),
            )
            .await?;
        }
        if full && msg.processing_state == ProcessingState::Pending {
            MessageRepo::set_state(&mut tx, msg.id, ProcessingState::Processed).await?;
        }
        ChangeBus::publish(
            &mut tx,
            household,
            "mail.updated",
            &serde_json::json!({"source": msg.source_id}),
        )
        .await?;
        if out.bills_proposed > 0 || out.contacts_suggested > 0 {
            ChangeBus::publish(&mut tx, household, "inbox.updated", &serde_json::json!({})).await?;
        }
        tx.commit().await?;
        tracing::info!(
            message = %msg.id,
            events = out.events,
            bills_open = out.bills_open,
            bills_proposed = out.bills_proposed,
            tasks = out.tasks,
            contacts = out.contacts_suggested + out.contacts_created,
            "mail.process"
        );
        Ok(out)
    }

    /// Resolves the household of a message when the payload carries none.
    async fn household_of(&self, message: MessageId) -> Result<Option<HouseholdId>> {
        let mut tx = self.store.tx_system().await?;
        Ok(MessageRepo::household_of(&mut tx, message).await?)
    }
}

pub(crate) fn job_err(e: MailError) -> JobError {
    match e {
        MailError::Store(s) => JobError::Store(s),
        other => JobError::Handler(other.to_string()),
    }
}

struct ProcessJob(ProcessService);
struct ExpireJob;

impl JobHandler for ProcessJob {
    fn kind(&self) -> &'static str {
        "mail.process"
    }
    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let p: Payload = serde_json::from_value(payload)
                .map_err(|e| JobError::Handler(format!("bad payload: {e}")))?;
            let household = match p.household {
                Some(h) => Some(h),
                None => self.0.household_of(p.message).await.map_err(job_err)?,
            };
            let Some(household) = household else {
                return Ok(());
            };
            self.0
                .process(household, p.message, p.only.as_deref())
                .await
                .map_err(job_err)?;
            Ok(())
        })
    }
}

impl JobHandler for ExpireJob {
    fn kind(&self) -> &'static str {
        "inbox.expire"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let n = expire_all(&ctx.store, Utc::now()).await.map_err(job_err)?;
            if n > 0 {
                tracing::info!(expired = n, "inbox.expire");
            }
            Ok(())
        })
    }
}

/// A `ProcessService` that can re-fetch PDF parts through the message's source.
pub fn process_service(deps: &Arc<MailDeps>) -> ProcessService {
    ProcessService::new(deps.store.clone()).with_attachments(Arc::new(
        crate::attachments::ProviderAttachments::new(deps.clone()),
    ))
}

pub fn handlers(deps: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    vec![
        Arc::new(ProcessJob(process_service(deps))),
        Arc::new(ExpireJob),
    ]
}

/// `(job kind, cron)`; `mail.process` is only ever enqueued.
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![("inbox.expire", "7 * * * *")]
}
