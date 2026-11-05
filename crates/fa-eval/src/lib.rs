//! fa-eval: replays the labelled sample (`message.in_eval_sample`) through the decision pipeline,
//! rules only and then rules plus the `llm` stage, and reports accuracy, coverage and latency.
//! It writes no decisions and enqueues nothing. See plan/M4.md "Eval (T-09)".

use std::collections::BTreeSet;
use std::time::Instant;

use fa_domain::{Candidate, DecisionPointKey, HouseholdId, MessageId, StageKind};
use fa_llm::Provider;
use fa_mail::context;
use fa_mail::process::{MailLlmResult, eval_request, llm_candidates, rules_candidates};
use fa_store::{AiSettingsRepo, DecisionPointRepo, MailSourceRepo, MessageRepo, PipelineRepo};
use serde::Serialize;
use sqlx::PgConnection;

const DEFAULT_THRESHOLD: f32 = 0.7;
const PURPOSE: &str = "eval";

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
    #[error(transparent)]
    Mail(#[from] fa_mail::MailError),
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    /// `--with-llm` without a provider or an enabled endpoint.
    #[error("{0}")]
    NotConfigured(String),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, EvalError>;

#[derive(Debug, Clone, Copy, Default)]
pub struct EvalOpts {
    pub with_llm: bool,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StageReport {
    pub stage: StageKind,
    pub decided: u32,
    pub correct: u32,
    pub p50_ms: f64,
    pub p95_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalReport {
    pub dp: DecisionPointKey,
    /// Sample messages that have a human truth for `dp`.
    pub items: u32,
    pub per_stage: Vec<StageReport>,
    /// Items nothing decided before `human`.
    pub undecided: u32,
    /// LLM calls that failed (the item counts as undecided).
    pub llm_errors: u32,
    /// Correct over decided, across stages.
    pub accuracy: f64,
    /// Share of items decided before `human`.
    pub coverage: f64,
}

/// Nearest-rank percentile of `xs` (sorted in place); 0 when empty.
fn percentile(xs: &mut [f64], p: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.sort_by(f64::total_cmp);
    let rank = (p * xs.len() as f64).ceil().max(1.0) as usize;
    xs[rank.min(xs.len()) - 1]
}

fn member_set(v: &serde_json::Value) -> BTreeSet<String> {
    match v {
        serde_json::Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_str().map(str::to_owned))
            .collect(),
        serde_json::Value::String(s) => BTreeSet::from([s.clone()]),
        _ => BTreeSet::new(),
    }
}

fn is_correct(dp: DecisionPointKey, accepted: &[Candidate], truth: &serde_json::Value) -> bool {
    if dp == DecisionPointKey::EmailMemberLink {
        let got: BTreeSet<String> = accepted
            .iter()
            .filter_map(|c| c.value.as_str().map(str::to_owned))
            .collect();
        got == member_set(truth)
    } else {
        accepted.first().is_some_and(|c| &c.value == truth)
    }
}

/// Candidates at or above the threshold: all members for `member_link` (deduped by value), else
/// the single best (ties: first). Mirrors `fa_pipeline`'s stage acceptance.
fn accept(dp: DecisionPointKey, cands: Vec<Candidate>, threshold: f32) -> Vec<Candidate> {
    let ok = cands.into_iter().filter(|c| c.confidence >= threshold);
    if dp == DecisionPointKey::EmailMemberLink {
        let mut out: Vec<Candidate> = Vec::new();
        for c in ok {
            match out.iter_mut().find(|o| o.value == c.value) {
                Some(o) if c.confidence > o.confidence => *o = c,
                Some(_) => {}
                None => out.push(c),
            }
        }
        out
    } else {
        let mut best: Option<Candidate> = None;
        for c in ok {
            if best.as_ref().is_none_or(|b| c.confidence > b.confidence) {
                best = Some(c);
            }
        }
        best.into_iter().collect()
    }
}

#[derive(Default)]
struct Acc {
    decided: u32,
    correct: u32,
    ms: Vec<f64>,
}

impl Acc {
    fn report(mut self, stage: StageKind) -> StageReport {
        StageReport {
            stage,
            decided: self.decided,
            correct: self.correct,
            p50_ms: percentile(&mut self.ms, 0.5),
            p95_ms: percentile(&mut self.ms, 0.95),
        }
    }
}

/// Replays the sample for `dp`. `conn` must be a transaction scoped to `household`.
pub async fn run(
    conn: &mut PgConnection,
    household: HouseholdId,
    dp: DecisionPointKey,
    opts: EvalOpts,
    llm: Option<&Provider>,
) -> Result<EvalReport> {
    if dp == DecisionPointKey::EventCalendarRoute {
        return Err(EvalError::Invalid(
            "the eval covers the four email decision points".into(),
        ));
    }
    let provider = if opts.with_llm {
        let p = llm.ok_or_else(|| EvalError::NotConfigured("no LLM provider".into()))?;
        let enabled = p
            .client_for(household)
            .await
            .map_err(|e| EvalError::NotConfigured(format!("LLM endpoint unusable: {}", e.kind())))?
            .is_some();
        if !enabled {
            return Err(EvalError::NotConfigured(
                "no enabled LLM endpoint for this household".into(),
            ));
        }
        Some(p)
    } else {
        None
    };

    let point = DecisionPointRepo::get_by_key(conn, dp)
        .await?
        .ok_or_else(|| EvalError::Invalid(format!("decision point {} missing", dp.as_str())))?;
    let stages = PipelineRepo::active_version(conn, point.id)
        .await?
        .map(|(_, s)| s)
        .unwrap_or_default();
    let rules_threshold = stages
        .iter()
        .find(|s| s.enabled && s.kind == StageKind::Rules)
        .map_or(DEFAULT_THRESHOLD, |s| s.accept_threshold);
    // The llm stage threshold applies whether or not the stage is enabled: the eval asks what the
    // stage would do.
    let llm_threshold = stages
        .iter()
        .find(|s| s.kind == StageKind::Llm)
        .map_or(DEFAULT_THRESHOLD, |s| s.accept_threshold);
    let max_chars = AiSettingsRepo::get_or_default(conn, household)
        .await?
        .max_input_chars as usize;

    // Sample messages with a human truth: the latest human decision per message.
    let rows: Vec<(uuid::Uuid, serde_json::Value)> = sqlx::query_as(
        "SELECT m.id, h.value FROM message m
         JOIN LATERAL (SELECT d.value FROM decision d
                       WHERE d.decision_point_id = $1 AND d.subject_type = 'message'
                         AND d.subject_id = m.id AND d.stage_kind = 'human'
                       ORDER BY d.created_at DESC, d.id DESC LIMIT 1) h ON true
         WHERE m.household_id = $2 AND m.in_eval_sample
         ORDER BY m.received_at, m.id
         LIMIT $3",
    )
    .bind(point.id.as_uuid())
    .bind(household.as_uuid())
    .bind(opts.limit.map_or(i64::MAX, i64::from))
    .fetch_all(&mut *conn)
    .await?;

    let mut rules = Acc::default();
    let mut llm_acc = Acc::default();
    let mut llm_errors = 0u32;
    let mut items = 0u32;
    for (id, truth) in rows {
        let id = MessageId::from(id);
        let Some(msg) = MessageRepo::get_with_content(conn, id).await? else {
            continue;
        };
        let source = MailSourceRepo::get(conn, msg.source_id)
            .await?
            .map(|s| s.value);
        let pc = context::load(conn, household, source.as_ref()).await?;
        items += 1;

        let t0 = Instant::now();
        let accepted = accept(dp, rules_candidates(&pc, &msg, dp), rules_threshold);
        let took = t0.elapsed().as_secs_f64() * 1000.0;
        if !accepted.is_empty() {
            rules.decided += 1;
            rules.ms.push(took);
            rules.correct += u32::from(is_correct(dp, &accepted, &truth));
            continue;
        }
        let Some(p) = provider else { continue };
        let req = eval_request(&pc, &msg, dp, max_chars);
        match p.call::<MailLlmResult>(household, PURPOSE, &req).await {
            Ok(got) => {
                let accepted = accept(dp, llm_candidates(&pc, &got.value, dp), llm_threshold);
                if !accepted.is_empty() {
                    llm_acc.decided += 1;
                    llm_acc.ms.push(got.latency.as_secs_f64() * 1000.0);
                    llm_acc.correct += u32::from(is_correct(dp, &accepted, &truth));
                }
            }
            Err(e) => {
                llm_errors += 1;
                tracing::warn!(kind = e.kind(), "eval llm call failed");
            }
        }
    }

    let decided = rules.decided + llm_acc.decided;
    let correct = rules.correct + llm_acc.correct;
    let mut per_stage = vec![rules.report(StageKind::Rules)];
    if provider.is_some() {
        per_stage.push(llm_acc.report(StageKind::Llm));
    }
    let ratio = |n: u32, d: u32| {
        if d == 0 {
            0.0
        } else {
            f64::from(n) / f64::from(d)
        }
    };
    Ok(EvalReport {
        dp,
        items,
        per_stage,
        undecided: items - decided,
        llm_errors,
        accuracy: ratio(correct, decided),
        coverage: ratio(decided, items),
    })
}

impl EvalReport {
    /// A plain-text table.
    pub fn to_table(&self) -> String {
        let mut s = format!(
            "decision point: {}\nitems with a human label: {}\n\n{:<8} {:>8} {:>8} {:>9} {:>9}\n",
            self.dp.as_str(),
            self.items,
            "stage",
            "decided",
            "correct",
            "p50 ms",
            "p95 ms"
        );
        for st in &self.per_stage {
            s.push_str(&format!(
                "{:<8} {:>8} {:>8} {:>9.1} {:>9.1}\n",
                format!("{:?}", st.stage).to_lowercase(),
                st.decided,
                st.correct,
                st.p50_ms,
                st.p95_ms
            ));
        }
        s.push_str(&format!(
            "\nundecided (fall to human): {}\nllm errors: {}\ncoverage: {:.1}%\naccuracy: {:.1}%\n",
            self.undecided,
            self.llm_errors,
            self.coverage * 100.0,
            self.accuracy * 100.0
        ));
        s
    }
}
