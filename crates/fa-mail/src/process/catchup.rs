//! Job `mail.llm.catchup` (D-107): queues `mail.llm` for the messages that waited for an LLM
//! (`message.llm_state = awaiting`) once the endpoint answers again. Enqueued with a household by
//! `PUT /ai/llm-endpoint` and, without one, by the 15-minute schedule (which then looks at every
//! household that has waiting messages). Logs carry counts only.

use std::time::Instant;

use chrono::{Days, TimeZone, Utc};
use chrono_tz::Tz;
use fa_domain::{HouseholdId, NewLlmCall};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{AiSettingsRepo, HouseholdRepo, LlmCallRepo, LlmEndpointRepo, LlmStateRepo};
use serde::Deserialize;

use super::{ProcessService, enqueue_llm, job_err};
use crate::Result;

const BATCH: i64 = 200;
/// The call purpose of the reachability probe (the same as the endpoint test in Settings).
const PROBE_PURPOSE: &str = "endpoint.test";

#[derive(Debug, Default, Deserialize)]
struct Payload {
    #[serde(default)]
    household: Option<HouseholdId>,
}

impl ProcessService {
    /// Queues the waiting messages of `household` received within `catchup_days`, newest first.
    /// Does nothing when nothing waits, there is no usable endpoint, or it does not answer a ping.
    /// Returns how many messages were queued.
    pub async fn catch_up(&self, household: HouseholdId) -> Result<usize> {
        let Some(llm) = &self.llm else {
            return Ok(0);
        };
        let (ai, backlog, tz, model) = {
            let mut tx = self.store.tx_for(household).await?;
            let ai = AiSettingsRepo::get_or_default(&mut tx, household).await?;
            let backlog = LlmStateRepo::backlog(&mut tx, household).await?;
            let tz: Tz = HouseholdRepo::get(&mut tx, household)
                .await?
                .and_then(|h| h.value.tz.parse().ok())
                .unwrap_or(Tz::UTC);
            let model = LlmEndpointRepo::get(&mut tx, household)
                .await?
                .map(|e| e.model)
                .unwrap_or_default();
            tx.rollback().await?;
            (ai, backlog, tz, model)
        };
        if backlog.awaiting == 0 || ai.catchup_days == 0 {
            return Ok(0);
        }
        let Some(client) = llm.client_for(household).await.ok().flatten() else {
            return Ok(0);
        };
        let started_at = Utc::now();
        let t0 = Instant::now();
        let probe = client.ping().await;
        let rec = NewLlmCall {
            household_id: household,
            purpose: PROBE_PURPOSE.to_owned(),
            model,
            started_at,
            latency_ms: i32::try_from(t0.elapsed().as_millis()).unwrap_or(i32::MAX),
            ok: probe.is_ok(),
            error_kind: probe.as_ref().err().map(|e| e.kind().to_owned()),
            prompt_tokens: None,
            completion_tokens: None,
        };
        {
            let mut tx = self.store.tx_for(household).await?;
            LlmCallRepo::insert(&mut tx, &rec).await?;
            tx.commit().await?;
        }
        if let Err(e) = probe {
            tracing::info!(kind = e.kind(), "mail.llm.catchup: endpoint not reachable");
            return Ok(0);
        }

        // The household's calendar day, `catchup_days` back, at local midnight.
        let now = (self.now)();
        let since = now
            .with_timezone(&tz)
            .date_naive()
            .checked_sub_days(Days::new(u64::from(ai.catchup_days)))
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|n| tz.from_local_datetime(&n).earliest())
            .map_or(now, |t| t.with_timezone(&Utc));
        let stamp = now.timestamp();
        let mut queued = 0usize;
        loop {
            let mut tx = self.store.tx_for(household).await?;
            let ids = LlmStateRepo::claim_awaiting(&mut tx, household, since, BATCH).await?;
            if ids.is_empty() {
                tx.rollback().await?;
                break;
            }
            for id in &ids {
                let rank = i32::try_from(queued).unwrap_or(i32::MAX);
                if enqueue_llm(&mut tx, household, *id, false, Some((stamp, rank))).await? {
                    queued += 1;
                } else {
                    LlmStateRepo::set(&mut tx, *id, fa_domain::LlmState::Awaiting).await?;
                }
            }
            tx.commit().await?;
            if ids.len() < usize::try_from(BATCH).unwrap_or(usize::MAX) {
                break;
            }
        }
        if queued > 0 {
            tracing::info!(queued, "mail.llm.catchup");
        }
        Ok(queued)
    }
}

pub(super) struct CatchupJob(pub ProcessService);

impl JobHandler for CatchupJob {
    fn kind(&self) -> &'static str {
        "mail.llm.catchup"
    }
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let p: Payload = serde_json::from_value(payload).unwrap_or_default();
            let households = match p.household {
                Some(h) => vec![h],
                None => {
                    let mut tx = ctx.store.tx_system().await?;
                    let ids = LlmStateRepo::households_awaiting(&mut tx).await?;
                    tx.rollback().await?;
                    ids
                }
            };
            for h in households {
                self.0.catch_up(h).await.map_err(job_err)?;
            }
            Ok(())
        })
    }
}
