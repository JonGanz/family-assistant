//! Job `mail.llm` (one System 2 call per message) and job `llm.prune`. See plan/M4.md "`llm` stage
//! and the `mail.llm` job" and arch/AI.md. Nothing the model returns is applied without approval:
//! decisions go through the pipeline's `llm` stage threshold, events and tasks go to the approval
//! inbox, bills are only ever proposed. Logs carry ids, counts and kinds, never content.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use fa_domain::{
    Bill, BillId, BillStatus, Candidate, CategoryBehaviour, DecidedBy, DecisionPointKey,
    EmailTasksMode, ExtractOrigin, HouseholdId, LlmState, MailExtractMode, MailMode, MessageId,
    NotifyTrigger, PipelineVersionId, StageKind, Subject, SubjectType, bill_dedupe_key,
};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_llm::{ExtractRequest, LlmError};
use fa_pipeline::Pipeline;
use fa_store::{
    AiSettingsRepo, BillRepo, ChangeBus, DecisionPointRepo, DecisionRepo, EventRepo, ExtractTarget,
    ExtractedByRepo, HouseholdRepo, LlmCallRepo, LlmStateRepo, MailSourceRepo, MessageRepo,
    NewExtractedEvent, NotificationRepo, PipelineRepo, ProposalRefineRepo,
};
use schemars::JsonSchema;
use serde::Deserialize;

use super::extract::{Emitted, Extractor, enqueue};
use super::facts::{OwnedFacts, short_hash, truncate};
use super::{ProcessService, job_err};
use crate::context::{self, ProcessCtx};
use crate::{MailError, Result};

const PURPOSE: &str = "mail.llm";
const DEFAULT_THRESHOLD: f32 = 0.7;
const MAX_TOKENS: u32 = 2048;
const PRUNE_DAYS: i64 = 90;
const DPS: [DecisionPointKey; 4] = [
    DecisionPointKey::EmailCategory,
    DecisionPointKey::EmailTimeSensitive,
    DecisionPointKey::EmailMemberLink,
    DecisionPointKey::EmailIsBill,
];

/// The model's answer. One struct for every combination of asked parts: the prompt names the
/// fields to fill and every field is optional, so a missing key is tolerated (Ollama does not
/// enforce strict mode, and `json_schema_for` does not list `Option` fields as required).
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct MailLlmResult {
    #[serde(default)]
    pub category: Option<CategoryPick>,
    #[serde(default)]
    pub time_sensitive: Option<FlagPick>,
    #[serde(default)]
    pub is_bill: Option<FlagPick>,
    #[serde(default)]
    pub member_link: Option<Vec<MemberPick>>,
    #[serde(default)]
    pub events: Option<Vec<LlmEvent>>,
    #[serde(default)]
    pub bills: Option<Vec<LlmBill>>,
    #[serde(default)]
    pub tasks: Option<Vec<LlmTask>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CategoryPick {
    /// One of the allowed category keys.
    pub value: String,
    pub confidence: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlagPick {
    pub value: bool,
    pub confidence: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemberPick {
    /// One of the listed member ids.
    pub member: String,
    pub confidence: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LlmEvent {
    pub title: String,
    /// RFC 3339 date-time, or `YYYY-MM-DD` for an all-day event.
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LlmBill {
    pub payee: String,
    pub amount_cents: i64,
    /// ISO 4217 code.
    pub currency: String,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub due: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LlmTask {
    pub title: String,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub due: Option<String>,
    /// `reply`, `sign`, `rsvp`, `pay` or `other`.
    pub kind: String,
    pub confidence: f32,
}

enum Fail {
    Mail(MailError),
    Llm(LlmError),
}

impl<E: Into<MailError>> From<E> for Fail {
    fn from(e: E) -> Self {
        Fail::Mail(e.into())
    }
}

struct Ask {
    dp: DecisionPointKey,
    version: PipelineVersionId,
}

fn clamp(c: f32) -> f32 {
    if c.is_finite() {
        c.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn cand(value: serde_json::Value, confidence: f32) -> Candidate {
    Candidate {
        value,
        confidence: clamp(confidence),
        source: "llm".into(),
    }
}

/// The candidates the model's answer holds for one decision point (the `llm` stage input).
pub fn llm_candidates(
    pc: &ProcessCtx,
    res: &MailLlmResult,
    dp: DecisionPointKey,
) -> Vec<Candidate> {
    match dp {
        DecisionPointKey::EmailCategory => res
            .category
            .iter()
            .filter(|c| pc.categories.contains_key(&c.value))
            .map(|c| cand(c.value.clone().into(), c.confidence))
            .collect(),
        DecisionPointKey::EmailTimeSensitive => res
            .time_sensitive
            .iter()
            .map(|c| cand(c.value.into(), c.confidence))
            .collect(),
        DecisionPointKey::EmailIsBill => res
            .is_bill
            .iter()
            .map(|c| cand(c.value.into(), c.confidence))
            .collect(),
        DecisionPointKey::EmailMemberLink => res
            .member_link
            .iter()
            .flatten()
            .filter(|m| m.member.parse().is_ok_and(|id| pc.member(id).is_some()))
            .map(|m| cand(m.member.clone().into(), m.confidence))
            .collect(),
        _ => Vec::new(),
    }
}

/// The request the `mail.llm` job would send for `msg`, limited to `dp` and without extraction
/// (used by the eval, so it shares the prompt with the job).
pub fn eval_request(
    pc: &ProcessCtx,
    msg: &fa_domain::Message,
    dp: DecisionPointKey,
    max_chars: usize,
) -> ExtractRequest {
    let asks = [Ask {
        dp,
        version: PipelineVersionId::new(),
    }];
    ProcessService::llm_request(pc, &OwnedFacts::new(msg), &asks, false, max_chars)
}

/// Rules-stage candidates for `dp` (before the stage threshold).
pub fn rules_candidates(
    pc: &ProcessCtx,
    msg: &fa_domain::Message,
    dp: DecisionPointKey,
) -> Vec<Candidate> {
    let of = OwnedFacts::new(msg);
    fa_rules::evaluate(dp, &of.facts(), &pc.rules)
}

/// An RFC 3339 instant, `YYYY-MM-DD` (all-day, local midnight) or a zone-less local date-time.
fn parse_when(s: &str, tz: Tz) -> Option<(DateTime<Utc>, bool)> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some((d.with_timezone(&Utc), false));
    }
    let local = |n: NaiveDateTime| {
        tz.from_local_datetime(&n)
            .earliest()
            .map(|d| d.with_timezone(&Utc))
    };
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return local(d.and_time(NaiveTime::MIN)).map(|t| (t, true));
    }
    ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
        .and_then(local)
        .map(|t| (t, false))
}

impl ProcessService {
    /// The decision points to ask the model about, and the threshold for extracted items.
    ///
    /// `rules_first`: points whose active pipeline has an enabled `llm` stage and no final
    /// decision yet. `llm_first`: every point whose `llm` stage is not switched off, except where
    /// a person decided (a label); the rules' decisions are replaced.
    async fn llm_asks(
        conn: &mut sqlx::PgConnection,
        subject: Subject,
        mode: MailMode,
    ) -> Result<(Vec<Ask>, f32)> {
        let mut asks = Vec::new();
        let mut threshold = DEFAULT_THRESHOLD;
        for dp in DPS {
            let Some(point) = DecisionPointRepo::get_by_key(conn, dp).await? else {
                continue;
            };
            let Some((version, stages)) = PipelineRepo::active_version(conn, point.id).await?
            else {
                continue;
            };
            let stage = stages.iter().find(|s| s.kind == StageKind::Llm);
            let latest = DecisionRepo::latest(conn, point.id, subject).await?;
            let (usable, wanted) = match mode {
                MailMode::RulesFirst => (
                    stage.is_some_and(|s| s.enabled),
                    latest.is_none_or(|d| !d.is_final),
                ),
                MailMode::LlmFirst => (
                    stage.is_none_or(|s| s.enabled),
                    !latest.is_some_and(|d| d.stage_kind == StageKind::Human && d.is_final),
                ),
            };
            if !usable {
                continue;
            }
            if dp == DecisionPointKey::EmailCategory
                && let Some(s) = stage
            {
                threshold = s.accept_threshold;
            }
            if wanted {
                asks.push(Ask {
                    dp,
                    version: version.id,
                });
            }
        }
        Ok((asks, threshold))
    }

    fn llm_request(
        pc: &ProcessCtx,
        of: &OwnedFacts,
        asks: &[Ask],
        extract: bool,
        max_chars: usize,
    ) -> ExtractRequest {
        let has = |dp| asks.iter().any(|a| a.dp == dp);
        let mut parts: Vec<&str> = Vec::new();
        let mut sys = format!(
            "You read one email for a family organiser and answer with JSON only. Fill only the \
             fields listed below and leave every other field out. Confidence is a number from 0 \
             to 1. The household time zone is {}. Dates without an offset are in that zone.\n",
            pc.household.tz
        );
        if has(DecisionPointKey::EmailCategory) {
            parts.push("category");
            let mut keys: Vec<&str> = pc.categories.keys().map(String::as_str).collect();
            keys.sort_unstable();
            sys.push_str(&format!(
                "category.value must be one of: {}.\n",
                keys.join(", ")
            ));
        }
        if has(DecisionPointKey::EmailTimeSensitive) {
            parts.push("time_sensitive");
        }
        if has(DecisionPointKey::EmailIsBill) {
            parts.push("is_bill");
        }
        if has(DecisionPointKey::EmailMemberLink) {
            parts.push("member_link");
            let mut ms: Vec<_> = pc.rules.members.iter().collect();
            ms.sort_by_key(|m| m.id.to_string());
            sys.push_str("member_link lists the household members the email is about (member must be one of these ids):\n");
            for m in ms {
                sys.push_str(&format!(
                    "{} {} ({})\n",
                    m.id,
                    m.name,
                    format!("{:?}", m.role).to_lowercase()
                ));
            }
        }
        if extract {
            parts.extend(["events", "bills", "tasks"]);
            sys.push_str(
                "events: start and end are RFC 3339 date-times, or YYYY-MM-DD for an all-day event. \
                 bills: amount_cents is an integer in minor units, due is YYYY-MM-DD. \
                 tasks: kind is one of reply, sign, rsvp, pay, other; due is YYYY-MM-DD. \
                 Only list items the email clearly asks for.\n",
            );
        }
        sys.push_str(&format!("Fields to fill: {}.", parts.join(", ")));
        let tz = pc.tz();
        let user = format!(
            "Subject: {}\nFrom: {}\nReceived: {}\n\n{}",
            of.subject,
            of.from_addr,
            of.received_at
                .with_timezone(&tz)
                .format("%Y-%m-%d %H:%M %Z"),
            truncate(&of.body_text, max_chars)
        );
        ExtractRequest {
            system: sys,
            user,
            schema_name: "mail_llm_result",
            max_tokens: MAX_TOKENS,
        }
    }

    /// No usable endpoint: a message that waited in `queued` or never asked moves to `awaiting`,
    /// so the catch-up picks it up when an endpoint is saved or switched on.
    async fn park(
        &self,
        household: HouseholdId,
        message: MessageId,
    ) -> std::result::Result<(), Fail> {
        let mut tx = self.store.tx_for(household).await?;
        LlmStateRepo::transition(
            &mut tx,
            message,
            &[LlmState::None, LlmState::Queued],
            LlmState::Awaiting,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// One `mail.llm` run: asks the model for what is still open, records the decisions and hands
    /// the extracted items to the approval inbox.
    async fn run_llm(
        &self,
        household: HouseholdId,
        message: MessageId,
        rules_strong: bool,
    ) -> std::result::Result<(), Fail> {
        let Some(llm) = &self.llm else {
            return self.park(household, message).await;
        };
        if llm
            .client_for(household)
            .await
            .map_err(Fail::Llm)?
            .is_none()
        {
            // No endpoint, or it is switched off: wait for one (D-107).
            return self.park(household, message).await;
        }
        let subject = Subject {
            kind: SubjectType::Message,
            id: message.as_uuid(),
        };
        // Read everything first; no transaction is held during the model call.
        let (msg, pc, of, asks, threshold, extract, max_chars, mode) = {
            let mut tx = self.store.tx_for(household).await?;
            if LlmStateRepo::get(&mut tx, message).await? == Some(LlmState::Done) {
                return Ok(());
            }
            let Some(msg) = MessageRepo::get_with_content(&mut tx, message).await? else {
                return Ok(());
            };
            let source = MailSourceRepo::get(&mut tx, msg.source_id)
                .await?
                .ok_or(MailError::NotFound)?
                .value;
            let pc = context::load(&mut tx, household, Some(&source)).await?;
            let ai = AiSettingsRepo::get_or_default(&mut tx, household).await?;
            let (asks, threshold) = Self::llm_asks(&mut tx, subject, ai.mail_mode).await?;
            let cat_asked = asks.iter().any(|a| a.dp == DecisionPointKey::EmailCategory);
            let extractable = msg
                .category
                .as_deref()
                .and_then(|k| pc.categories.get(k))
                .is_some_and(|c| c.behaviour == CategoryBehaviour::Extract && c.key != "junk");
            let extract = ai.mail_extract_mode != MailExtractMode::Off
                && (cat_asked || extractable)
                && (ai.mail_mode == MailMode::LlmFirst
                    || ai.mail_extract_mode == MailExtractMode::Always
                    || !rules_strong);
            let of = OwnedFacts::new(&msg);
            (
                msg,
                pc,
                of,
                asks,
                threshold,
                extract,
                ai.max_input_chars as usize,
                ai.mail_mode,
            )
        };
        if asks.is_empty() && !extract {
            let mut tx = self.store.tx_for(household).await?;
            LlmStateRepo::set(&mut tx, message, LlmState::Done).await?;
            tx.commit().await?;
            return Ok(());
        }
        let req = Self::llm_request(&pc, &of, &asks, extract, max_chars);
        let got = llm
            .call::<MailLlmResult>(household, PURPOSE, &req)
            .await
            .map_err(Fail::Llm)?;
        let res = got.value;

        // Decisions and message columns.
        let mut category: Option<String> = None;
        let mut time_sensitive = None;
        let mut is_bill = None;
        let mut recorded = 0usize;
        let mut tx = self.store.tx_for(household).await?;
        for ask in &asks {
            let cands = llm_candidates(&pc, &res, ask.dp);
            if cands.is_empty() {
                continue;
            }
            let o = Pipeline::record_stage_or(
                &mut tx,
                subject,
                ask.dp,
                StageKind::Llm,
                cands,
                ask.version,
                Some(DEFAULT_THRESHOLD),
            )
            .await?;
            if o.stage != StageKind::Llm {
                continue;
            }
            recorded += o.values.len();
            let first = o.values.first();
            match ask.dp {
                DecisionPointKey::EmailCategory => {
                    category = first.and_then(|c| c.value.as_str()).map(str::to_owned);
                }
                DecisionPointKey::EmailTimeSensitive => {
                    time_sensitive = first.and_then(|c| c.value.as_bool());
                }
                DecisionPointKey::EmailIsBill => {
                    is_bill = first.and_then(|c| c.value.as_bool());
                }
                DecisionPointKey::EmailMemberLink => {
                    let d = super::Decided {
                        values: o.values,
                        by: DecidedBy::Rule,
                        pending_llm: false,
                    };
                    let links = super::resolve_members(&pc, &d);
                    MessageRepo::set_members(&mut tx, message, &links).await?;
                }
                _ => {}
            }
        }
        if category.is_some() || time_sensitive.is_some() || is_bill.is_some() {
            MessageRepo::set_classification(
                &mut tx,
                message,
                category.as_deref(),
                time_sensitive,
                is_bill,
            )
            .await?;
        }
        let changed = category
            .as_deref()
            .is_some_and(|c| Some(c) != msg.category.as_deref());
        if changed {
            // The category behaviour step again: auto actions (a new job key) and, below, the
            // rule extractors, which are idempotent.
            enqueue(
                &mut tx,
                household,
                "mail.auto_actions",
                serde_json::json!({"household": household, "message": message}),
                format!("mail.auto_actions:{message}:llm"),
            )
            .await?;
        }
        LlmStateRepo::set(&mut tx, message, LlmState::Done).await?;
        ChangeBus::publish(
            &mut tx,
            household,
            "mail.updated",
            &serde_json::json!({"source": msg.source_id}),
        )
        .await?;
        tx.commit().await?;
        if changed {
            self.process(household, message, Some(&["extract".to_owned()]))
                .await?;
        }

        // Extracted items, after the rule extractors so that duplicates of rule items drop.
        let mut out = Emitted::default();
        let cat_key = category.clone().or(msg.category.clone());
        let extractable = cat_key
            .as_deref()
            .and_then(|k| pc.categories.get(k))
            .filter(|c| c.behaviour == CategoryBehaviour::Extract && c.key != "junk");
        if extract
            && let Some(cat) = extractable
            && (res.events.is_some() || res.bills.is_some() || res.tasks.is_some())
        {
            let mut tx = self.store.tx_for(household).await?;
            let source = MailSourceRepo::get(&mut tx, msg.source_id)
                .await?
                .ok_or(MailError::NotFound)?
                .value;
            let members: Vec<_> = MessageRepo::members(&mut tx, message)
                .await?
                .into_iter()
                .map(|m| m.0)
                .collect();
            let bill_flag = is_bill.or(msg.is_bill);
            let mut ex = Extractor {
                conn: &mut tx,
                household,
                msg: &msg,
                source: &source,
                pc: &pc,
                of: &of,
                category: cat,
                is_bill: bill_flag,
                members: &members,
                attachments: None,
                refine: mode == MailMode::LlmFirst,
                strong_cutoff: 1.0,
                strong: false,
            };
            ex.llm_events(
                (self.now)(),
                res.events.as_deref().unwrap_or(&[]),
                threshold,
                &mut out,
            )
            .await?;
            if bill_flag != Some(false) {
                ex.llm_bills(res.bills.as_deref().unwrap_or(&[]), threshold, &mut out)
                    .await?;
            }
            ex.llm_tasks(res.tasks.as_deref().unwrap_or(&[]), threshold, &mut out)
                .await?;
            if out.events > 0 || out.bills_proposed > 0 || out.tasks > 0 || out.refined > 0 {
                ChangeBus::publish(&mut tx, household, "inbox.updated", &serde_json::json!({}))
                    .await?;
            }
            tx.commit().await?;
        }
        tracing::info!(
            message = %message,
            asked = asks.len(),
            extract,
            decisions = recorded,
            category_changed = changed,
            events = out.events,
            bills = out.bills_proposed,
            tasks = out.tasks,
            refined = out.refined,
            latency_ms = got.latency.as_millis() as u64,
            "mail.llm"
        );
        Ok(())
    }
}

/// The event, among `items` (title, start, all-day), that the LLM's event is the same as: the
/// same start slot (local day and quarter hour, as in the dedupe key) and, when several share
/// it, the most similar title.
fn match_slot(
    items: &[(&str, DateTime<Utc>, bool)],
    title: &str,
    start: DateTime<Utc>,
    all_day: bool,
    tz: Tz,
) -> Option<usize> {
    let slot = fa_rules::event_dedupe_key("", start, all_day, tz);
    let same: Vec<usize> = (0..items.len())
        .filter(|&i| {
            items[i].2 == all_day && fa_rules::event_dedupe_key("", items[i].1, all_day, tz) == slot
        })
        .collect();
    if same.len() == 1 {
        return same.first().copied();
    }
    same.into_iter()
        .map(|i| (i, fa_rules::title_similarity(items[i].0, title)))
        .filter(|(_, sim)| *sim >= 0.6)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

impl Extractor<'_> {
    pub(crate) async fn llm_events(
        &mut self,
        now: DateTime<Utc>,
        items: &[LlmEvent],
        threshold: f32,
        out: &mut Emitted,
    ) -> Result<()> {
        let tz = self.pc.tz();
        let org = self.org().map(|o| o.id);
        let derived = EventRepo::derived_for_message(self.conn, self.msg.id).await?;
        let mut seen: HashSet<String> =
            derived.iter().map(|e| e.event.dedupe_key.clone()).collect();
        // Events of this message that are no longer pending (approved, rejected, written, or
        // already the LLM's): an LLM event for the same slot is the same event, not a new one.
        let settled: Vec<(&str, DateTime<Utc>, bool)> = derived
            .iter()
            .filter(|e| e.message_id == Some(self.msg.id))
            .map(|e| (e.event.title.as_str(), e.event.start_at, e.event.all_day))
            .collect();
        let mut pending = if self.refine {
            ProposalRefineRepo::events(self.conn, self.msg.id).await?
        } else {
            Vec::new()
        };
        for e in items {
            let conf = clamp(e.confidence);
            let title = e.title.trim();
            let Some((start, all_day)) = parse_when(&e.start, tz) else {
                continue;
            };
            if conf < threshold || title.is_empty() {
                continue;
            }
            let end = e
                .end
                .as_deref()
                .and_then(|s| parse_when(s, tz))
                .map(|x| x.0);
            if end.unwrap_or(start) < now {
                continue;
            }
            let key = fa_rules::event_dedupe_key(title, start, all_day, tz);
            let pending_view: Vec<(&str, DateTime<Utc>, bool)> = pending
                .iter()
                .map(|p| (p.title.as_str(), p.start_at, p.all_day))
                .collect();
            let matched = if self.refine {
                match_slot(&pending_view, title, start, all_day, tz)
            } else {
                None
            };
            if let Some(i) = matched {
                let p = pending.remove(i);
                let location = e.location.as_deref().filter(|l| !l.trim().is_empty());
                if ProposalRefineRepo::refine_event(self.conn, p.id, title, location, conf, &key)
                    .await?
                {
                    seen.insert(key);
                    out.refined += 1;
                    continue;
                }
            }
            if self.refine && match_slot(&settled, title, start, all_day, tz).is_some() {
                continue;
            }
            if !seen.insert(key.clone()) {
                continue;
            }
            let (start_date, end_date, end_at) = if all_day {
                let s = start.with_timezone(&tz).date_naive();
                let mut en = end.unwrap_or(start).with_timezone(&tz).date_naive();
                if en <= s {
                    en = s + chrono::Days::new(1);
                }
                (Some(s), Some(en), end.unwrap_or(start).max(start))
            } else {
                (
                    None,
                    None,
                    end.unwrap_or(start + Duration::hours(1)).max(start),
                )
            };
            let id = EventRepo::insert_extracted(
                self.conn,
                &NewExtractedEvent {
                    household_id: self.household,
                    message_id: Some(self.msg.id),
                    org_id: org,
                    title: title.to_owned(),
                    description: None,
                    location: e.location.clone().filter(|l| !l.trim().is_empty()),
                    start_at: start,
                    end_at,
                    all_day,
                    start_date,
                    end_date,
                    source_tz: Some(tz.name().to_owned()),
                    cancelled: false,
                    confidence: conf,
                    source_ref: Some(self.source_ref()),
                    dedupe_key: key,
                    remote_uid: None,
                    members: self.members.to_vec(),
                },
            )
            .await?;
            ExtractedByRepo::set(
                self.conn,
                ExtractTarget::Event,
                id.as_uuid(),
                ExtractOrigin::Llm,
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

    /// Bills are only proposed, never opened: the approval inbox decides.
    pub(crate) async fn llm_bills(
        &mut self,
        items: &[LlmBill],
        threshold: f32,
        out: &mut Emitted,
    ) -> Result<()> {
        let org = self.org().map(|o| o.id);
        let (mut pending, settled) = if self.refine {
            (
                ProposalRefineRepo::bills(self.conn, self.msg.id).await?,
                ProposalRefineRepo::settled_bills(self.conn, self.msg.id).await?,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        let same_bill = |p: &fa_store::PendingBill, amount: i64, due: Option<NaiveDate>| {
            p.amount_minor == amount && (p.due_on == due || p.due_on.is_none() || due.is_none())
        };
        for b in items {
            let conf = clamp(b.confidence);
            let payee = b.payee.trim();
            if conf < threshold || payee.is_empty() || b.amount_cents <= 0 {
                continue;
            }
            let due_on = b
                .due
                .as_deref()
                .and_then(|d| NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok());
            // The same bill: the same amount, and the same due date unless one side has none.
            if let Some(i) = pending
                .iter()
                .position(|p| same_bill(p, b.amount_cents, due_on))
            {
                let p = pending.remove(i);
                let key = bill_dedupe_key(payee, b.amount_cents, due_on.or(p.due_on));
                if ProposalRefineRepo::refine_bill(self.conn, p.id, payee, due_on, conf, &key)
                    .await?
                {
                    out.refined += 1;
                    continue;
                }
            }
            if settled.iter().any(|p| same_bill(p, b.amount_cents, due_on)) {
                continue;
            }
            let currency = match b.currency.trim() {
                c if c.len() == 3 && c.is_ascii() => c.to_ascii_uppercase(),
                _ => self.pc.household.currency.clone(),
            };
            let bill = Bill {
                id: BillId::new(),
                household_id: self.household,
                message_id: Some(self.msg.id),
                org_id: org,
                payee: payee.to_owned(),
                amount_minor: b.amount_cents,
                currency,
                due_on,
                account_hint: None,
                statement_period: None,
                status: BillStatus::Proposed,
                confidence: conf,
                task_id: None,
                source_ref: self.source_ref(),
                dedupe_key: bill_dedupe_key(payee, b.amount_cents, due_on),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            if BillRepo::insert_if_new(self.conn, &bill).await?.is_none() {
                continue;
            }
            ExtractedByRepo::set(
                self.conn,
                ExtractTarget::Bill,
                bill.id.as_uuid(),
                ExtractOrigin::Llm,
            )
            .await?;
            NotificationRepo::insert_for_source_adults(
                self.conn,
                Some(self.msg.source_id),
                NotifyTrigger::ApprovalNeeded,
                "bill",
                Some(bill.id.as_uuid()),
                "A bill needs your confirmation",
                &format!("{}: please confirm the amount", truncate(payee, 80)),
                Some("/inbox"),
                &format!("approval:bill:{}", bill.id),
            )
            .await?;
            out.bills_proposed += 1;
        }
        Ok(())
    }

    /// Tasks are only suggested. The `tasks.from_mail` payload carries `extracted_by: "llm"`; the
    /// job that creates the task id marks it.
    pub(crate) async fn llm_tasks(
        &mut self,
        items: &[LlmTask],
        threshold: f32,
        out: &mut Emitted,
    ) -> Result<()> {
        if self.category.email_tasks == EmailTasksMode::Off {
            return Ok(());
        }
        let tz = self.pc.tz();
        let (reply_to, action_to) = self.task_targets();
        let mut pending = if self.refine {
            ProposalRefineRepo::tasks(self.conn, self.msg.id).await?
        } else {
            Vec::new()
        };
        for t in items {
            let title = t.title.trim();
            if clamp(t.confidence) < threshold || title.is_empty() {
                continue;
            }
            let kind = match t.kind.as_str() {
                k @ ("reply" | "sign" | "rsvp" | "pay") => k,
                _ => "other",
            };
            let due_at = t
                .due
                .as_deref()
                .and_then(|d| NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok())
                .and_then(|d| {
                    tz.from_local_datetime(&d.and_hms_opt(23, 59, 0)?)
                        .earliest()
                })
                .map(|d| d.with_timezone(&Utc));
            // A pending rule task of the same sort (reply with reply, anything else with the
            // rules' "action") takes the LLM's title, due date and kind.
            if let Some(i) = pending
                .iter()
                .position(|p| (p.kind == "reply") == (kind == "reply"))
            {
                let p = pending.remove(i);
                if ProposalRefineRepo::refine_task(self.conn, p.id, title, due_at, kind).await? {
                    out.refined += 1;
                    continue;
                }
            }
            let assignees = if kind == "reply" {
                &reply_to
            } else {
                &action_to
            };
            let payload = serde_json::json!({
                "household": self.household,
                "message": self.msg.id,
                "kind": kind,
                "title": title,
                "due_at": due_at,
                "assignees": assignees,
                "mode": "suggest",
                "external_url": self.external_url(),
                "extracted_by": "llm",
            });
            let key = format!("tasks.from_mail:{}:{}", self.msg.id, short_hash(title));
            if enqueue(self.conn, self.household, "tasks.from_mail", payload, key).await? {
                out.tasks += 1;
            }
        }
        Ok(())
    }
}

pub(super) struct LlmJob(pub ProcessService);

#[derive(Deserialize)]
struct LlmPayload {
    message_id: MessageId,
    #[serde(default)]
    household: Option<HouseholdId>,
    #[serde(default)]
    rules_strong: bool,
}

impl JobHandler for LlmJob {
    fn kind(&self) -> &'static str {
        "mail.llm"
    }
    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let p: LlmPayload = serde_json::from_value(payload)
                .map_err(|e| JobError::Handler(format!("bad payload: {e}")))?;
            let household = match p.household {
                Some(h) => Some(h),
                None => self.0.household_of(p.message_id).await.map_err(job_err)?,
            };
            let Some(household) = household else {
                return Ok(());
            };
            match self
                .0
                .run_llm(household, p.message_id, p.rules_strong)
                .await
            {
                Ok(()) => Ok(()),
                Err(Fail::Mail(e)) => Err(job_err(e)),
                // Retried with the job runner's backoff (on_unavailable = queue); only the kind
                // is kept, never the message of the error. On the last attempt the message
                // waits for the catch-up instead.
                Err(Fail::Llm(LlmError::Config(c))) if c == "disabled" => self
                    .0
                    .park(household, p.message_id)
                    .await
                    .map_err(|f| match f {
                        Fail::Mail(e) => job_err(e),
                        Fail::Llm(e) => JobError::Handler(format!("llm {}", e.kind())),
                    }),
                Err(Fail::Llm(e)) => {
                    self.0.after_failure(household, p.message_id).await;
                    Err(JobError::Handler(format!("llm {}", e.kind())))
                }
            }
        })
    }
}

impl ProcessService {
    /// A failed model call: when no retry is left, the message moves to `awaiting`.
    async fn after_failure(&self, household: HouseholdId, message: MessageId) {
        let res = async {
            let last = {
                let mut sys = self.store.tx_system().await?;
                let a = LlmStateRepo::running_attempt(&mut sys, message).await?;
                sys.rollback().await?;
                a.is_none_or(|(attempts, max)| attempts >= max)
            };
            if last {
                let mut tx = self.store.tx_for(household).await?;
                LlmStateRepo::transition(
                    &mut tx,
                    message,
                    &[LlmState::None, LlmState::Queued],
                    LlmState::Awaiting,
                )
                .await?;
                tx.commit().await?;
            }
            Ok::<(), fa_store::StoreError>(())
        }
        .await;
        if let Err(e) = res {
            tracing::warn!(error = %e, "llm_state not updated");
        }
    }
}

/// Daily: deletes `llm_call` rows older than 90 days, per household.
pub(super) struct PruneJob;

impl JobHandler for PruneJob {
    fn kind(&self) -> &'static str {
        "llm.prune"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let cutoff = Utc::now() - Duration::days(PRUNE_DAYS);
            let ids = {
                let mut tx = ctx.store.tx_system().await?;
                HouseholdRepo::list(&mut tx)
                    .await?
                    .into_iter()
                    .map(|h| h.value.id)
                    .collect::<Vec<_>>()
            };
            let mut total = 0;
            for h in ids {
                let mut tx = ctx.store.tx_for(h).await?;
                total += LlmCallRepo::prune_before(&mut tx, cutoff).await?;
                tx.commit().await?;
            }
            if total > 0 {
                tracing::info!(deleted = total, "llm.prune");
            }
            Ok(())
        })
    }
}
