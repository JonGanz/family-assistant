//! Second LLM pass for mail-extracted events: is the proposed event already on the calendar?
//! See arch/AI.md "Event duplicate check". A deterministic pre-filter loads the household's events
//! around the proposed date; with none, the model is not asked. A confident "duplicate" drops the
//! proposal and links the message to the existing event; every error keeps the proposal.
//! Logs carry ids, counts and kinds, never content.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use fa_domain::{EventId, HouseholdId, MessageId};
use fa_llm::ExtractRequest;
use fa_store::{AiSettingsRepo, DedupeEvent, EventRepo, MessageRepo};
use schemars::JsonSchema;
use serde::Deserialize;

use super::ProcessService;
use super::facts::{OwnedFacts, truncate};
use super::llm::{LlmEvent, clamp, parse_when};
use crate::Result;
use crate::context::ProcessCtx;

pub(crate) const DEDUPE_PURPOSE: &str = "event.dedupe";
const MAX_TOKENS: u32 = 512;
/// Existing events listed to the model (earliest first).
const MAX_EXISTING: i64 = 25;
const EXCERPT_CHARS: usize = 800;

/// The model's answer.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct DedupeAnswer {
    /// The number of the existing event that is the same event, or null when none is.
    #[serde(default)]
    pub duplicate_of: Option<u32>,
    /// 0 to 1.
    #[serde(default)]
    pub confidence: f32,
    /// One short sentence.
    #[serde(default)]
    pub reason: String,
}

fn when(start: DateTime<Utc>, end: DateTime<Utc>, all_day: bool, tz: Tz) -> String {
    let (s, e) = (start.with_timezone(&tz), end.with_timezone(&tz));
    if all_day {
        // `end` is exclusive for all-day events.
        let last = (e - Duration::days(1)).max(s);
        if last.date_naive() == s.date_naive() {
            format!("{} (all day)", s.format("%Y-%m-%d"))
        } else {
            format!(
                "{} to {} (all day)",
                s.format("%Y-%m-%d"),
                last.format("%Y-%m-%d")
            )
        }
    } else {
        format!(
            "{} to {}",
            s.format("%Y-%m-%d %H:%M"),
            e.format("%Y-%m-%d %H:%M")
        )
    }
}

struct Cand<'a> {
    event: &'a LlmEvent,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    all_day: bool,
}

fn request(
    tz: Tz,
    of: &OwnedFacts,
    members: &[String],
    c: &Cand,
    existing: &[DedupeEvent],
) -> ExtractRequest {
    let system = format!(
        "You compare one proposed calendar event, found in an email, with events that are already \
         on a family calendar, and answer with JSON only. Decide whether the proposed event is the \
         same real-world event as one of the existing ones (the same occasion, even if the title \
         is worded differently or the time differs a little). Different occasions on the same day \
         are not duplicates. duplicate_of is the number of the existing event, or null when none \
         is the same. confidence is a number from 0 to 1. reason is one short sentence. The \
         household time zone is {tz}."
    );
    let mut user = format!(
        "Proposed event:\ntitle: {}\nwhen: {}\nlocation: {}\nfor: {}\n\nFrom the email:\nSubject: {}\nFrom: {}\n{}\n\nExisting events:\n",
        c.event.title.trim(),
        when(c.start, c.end, c.all_day, tz),
        c.event.location.as_deref().unwrap_or("-"),
        if members.is_empty() {
            "-".to_owned()
        } else {
            members.join(", ")
        },
        of.subject,
        of.from_addr,
        truncate(&of.body_text, EXCERPT_CHARS),
    );
    for (i, e) in existing.iter().enumerate() {
        user.push_str(&format!(
            "{}. {} | {} | location: {} | calendar: {}{}\n",
            i + 1,
            e.title,
            when(e.start_at, e.end_at, e.all_day, tz),
            e.location.as_deref().unwrap_or("-"),
            e.calendar_name.as_deref().unwrap_or("-"),
            if e.proposed {
                " | pending proposal"
            } else {
                ""
            },
        ));
    }
    ExtractRequest {
        system,
        user,
        schema_name: "event_dedupe",
        max_tokens: MAX_TOKENS,
    }
}

impl ProcessService {
    /// `items` without the events the model says are already on the calendar. Never fails and
    /// never drops an event on doubt: any error keeps it.
    pub(super) async fn dedupe_events(
        &self,
        household: HouseholdId,
        message: MessageId,
        pc: &ProcessCtx,
        of: &OwnedFacts,
        items: Vec<LlmEvent>,
        threshold: f32,
    ) -> Vec<LlmEvent> {
        match self
            .dedupe_inner(household, message, pc, of, &items, threshold)
            .await
        {
            Ok(drop) if !drop.is_empty() => items
                .into_iter()
                .enumerate()
                .filter(|(i, _)| !drop.contains(i))
                .map(|(_, e)| e)
                .collect(),
            Ok(_) => items,
            Err(e) => {
                tracing::warn!(message = %message, error = %e, "event.dedupe skipped");
                items
            }
        }
    }

    async fn dedupe_inner(
        &self,
        household: HouseholdId,
        message: MessageId,
        pc: &ProcessCtx,
        of: &OwnedFacts,
        items: &[LlmEvent],
        threshold: f32,
    ) -> Result<HashSet<usize>> {
        let mut drop = HashSet::new();
        let Some(llm) = &self.llm else {
            return Ok(drop);
        };
        let tz = pc.tz();
        let now = (self.now)();
        // Read first; no transaction is held during a model call.
        let (dedupe_threshold, keys, members): (f32, HashSet<String>, Vec<String>) = {
            let mut tx = self.store.tx_for(household).await?;
            let s = AiSettingsRepo::get_or_default(&mut tx, household).await?;
            if !s.event_dedupe_llm {
                return Ok(drop);
            }
            let keys = EventRepo::derived_for_message(&mut tx, message)
                .await?
                .into_iter()
                .map(|e| e.event.dedupe_key)
                .collect();
            let names = MessageRepo::members(&mut tx, message)
                .await?
                .into_iter()
                .filter_map(|(id, ..)| {
                    pc.rules
                        .members
                        .iter()
                        .find(|m| m.id == id)
                        .map(|m| m.name.clone())
                })
                .collect();
            tx.rollback().await?;
            (s.event_dedupe_threshold, keys, names)
        };
        let mut cands: Vec<(usize, Cand)> = Vec::new();
        for (i, e) in items.iter().enumerate() {
            // The same filters `llm_events` applies, and its deterministic dedupe first.
            let Some((start, all_day)) = parse_when(&e.start, tz) else {
                continue;
            };
            if clamp(e.confidence) < threshold || e.title.trim().is_empty() {
                continue;
            }
            let end = e
                .end
                .as_deref()
                .and_then(|s| parse_when(s, tz))
                .map(|x| x.0)
                .unwrap_or(start + Duration::hours(1))
                .max(start);
            if end < now
                || keys.contains(&fa_rules::event_dedupe_key(
                    e.title.trim(),
                    start,
                    all_day,
                    tz,
                ))
            {
                continue;
            }
            cands.push((
                i,
                Cand {
                    event: e,
                    start,
                    end,
                    all_day,
                },
            ));
        }
        for (i, c) in cands {
            let existing = {
                let mut tx = self.store.tx_for(household).await?;
                let v = EventRepo::llm_dedupe_candidates(
                    &mut tx,
                    c.start - Duration::days(1),
                    c.end.max(c.start + Duration::minutes(1)) + Duration::days(1),
                    message,
                    MAX_EXISTING,
                )
                .await?;
                tx.rollback().await?;
                v
            };
            if existing.is_empty() {
                continue;
            }
            let req = request(tz, of, &members, &c, &existing);
            let got = match llm
                .call::<DedupeAnswer>(household, DEDUPE_PURPOSE, &req)
                .await
            {
                Ok(g) => g.value,
                Err(e) => {
                    tracing::warn!(message = %message, kind = e.kind(), "event.dedupe call failed");
                    continue;
                }
            };
            let Some(hit) = got
                .duplicate_of
                .and_then(|n| usize::try_from(n).ok())
                .and_then(|n| n.checked_sub(1))
                .and_then(|n| existing.get(n))
            else {
                continue;
            };
            let conf = clamp(got.confidence);
            if conf < dedupe_threshold {
                continue;
            }
            let existing_id = EventId::from(hit.id);
            let mut tx = self.store.tx_for(household).await?;
            EventRepo::link_message(&mut tx, existing_id, message).await?;
            tx.commit().await?;
            tracing::info!(
                message = %message,
                duplicate_of = %existing_id,
                confidence = conf,
                reason = %truncate(got.reason.trim(), 200),
                "event.dedupe: proposal dropped, event already on the calendar"
            );
            drop.insert(i);
        }
        Ok(drop)
    }
}
