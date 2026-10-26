//! fa-pipeline (M3): the decision pipeline engine. See arch/Ingest.md section 5 and plan/M3.md
//! "Pipeline engine".

use fa_domain::{
    Decision, DecisionId, DecisionPointKey, HouseholdId, MemberId, PipelineStage,
    PipelineVersionId, StageKind, Subject,
};
use fa_store::{DecisionPointRepo, DecisionRepo, NewDecision, PipelineRepo, mail_defaults};
use sqlx::PgConnection;

pub use fa_domain::Candidate;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Store(#[from] fa_store::StoreError),
}

pub type Result<T> = std::result::Result<T, PipelineError>;

/// Supplies the rules-stage candidates for a decision point.
pub trait Evaluator: Send + Sync {
    fn candidates(&self, dp: DecisionPointKey) -> Vec<Candidate>;
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub values: Vec<Candidate>,
    pub stage: StageKind,
    pub is_final: bool,
    pub version: PipelineVersionId,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Pipeline;

/// A version needs an enabled stage, must end in `rules` or `human`, and have thresholds in 0..=1.
pub fn validate_version(stages: &[PipelineStage]) -> Result<()> {
    if let Some(s) = stages
        .iter()
        .find(|s| !(0.0..=1.0).contains(&s.accept_threshold))
    {
        return Err(PipelineError::Invalid(format!(
            "stage {} threshold {} is outside 0..=1",
            s.ord, s.accept_threshold
        )));
    }
    let last = stages
        .iter()
        .filter(|s| s.enabled)
        .max_by_key(|s| s.ord)
        .ok_or_else(|| {
            PipelineError::Invalid("pipeline needs at least one enabled stage".into())
        })?;
    if !matches!(last.kind, StageKind::Rules | StageKind::Human) {
        return Err(PipelineError::Invalid(
            "the last enabled stage must be rules or human".into(),
        ));
    }
    Ok(())
}

/// Candidates at or above the threshold: every member for `email.member_link` (deduped by value,
/// highest confidence kept), otherwise the single best (ties: first).
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

async fn dp_id(conn: &mut PgConnection, dp: DecisionPointKey) -> Result<fa_domain::DecisionPoint> {
    DecisionPointRepo::get_by_key(conn, dp)
        .await?
        .ok_or_else(|| PipelineError::Invalid(format!("decision point {} missing", dp.as_str())))
}

impl Pipeline {
    pub fn new() -> Self {
        Self
    }

    pub async fn ensure_defaults(conn: &mut PgConnection, household: HouseholdId) -> Result<()> {
        mail_defaults::ensure(conn, household).await?;
        Ok(())
    }

    pub async fn run(
        conn: &mut PgConnection,
        household: HouseholdId,
        dp: DecisionPointKey,
        subject: Subject,
        eval: &dyn Evaluator,
    ) -> Result<Outcome> {
        let point = dp_id(conn, dp).await?;
        let (version, stages) = PipelineRepo::active_version(conn, point.id)
            .await?
            .ok_or_else(|| {
                PipelineError::Invalid(format!("no active pipeline for {}", dp.as_str()))
            })?;
        let mut warned = false;
        let mut stored: Option<(StageKind, bool, Vec<Candidate>)> = None;
        for stage in stages.iter().filter(|s| s.enabled) {
            match stage.kind {
                StageKind::Rules => {
                    let accepted = accept(dp, eval.candidates(dp), stage.accept_threshold);
                    if !accepted.is_empty() {
                        stored = Some((StageKind::Rules, true, accepted));
                        break;
                    }
                }
                StageKind::Human => break,
                StageKind::Knn | StageKind::SystemOne | StageKind::Llm => {
                    if !warned {
                        warned = true;
                        tracing::debug!(
                            dp = dp.as_str(),
                            stage = stage.kind.as_str(),
                            on_unavailable = stage.on_unavailable.as_str(),
                            "pipeline stage unavailable, skipped"
                        );
                    }
                }
            }
        }
        let (kind, is_final, values) = stored.unwrap_or_else(|| {
            (
                StageKind::Human,
                false,
                vec![Candidate {
                    value: point.default_value.clone(),
                    confidence: 0.0,
                    source: "default".into(),
                }],
            )
        });
        for c in &values {
            DecisionRepo::insert(
                conn,
                &NewDecision {
                    household_id: household,
                    decision_point_id: point.id,
                    subject,
                    pipeline_version_id: Some(version.id),
                    stage_kind: kind,
                    value: c.value.clone(),
                    confidence: c.confidence,
                    is_final,
                    source: Some(c.source.clone()),
                    decided_by: None,
                },
            )
            .await?;
        }
        Ok(Outcome {
            values,
            stage: kind,
            is_final,
            version: version.id,
        })
    }

    pub async fn record_human(
        conn: &mut PgConnection,
        household: HouseholdId,
        dp: DecisionPointKey,
        subject: Subject,
        value: serde_json::Value,
        member: Option<MemberId>,
    ) -> Result<DecisionId> {
        let point = dp_id(conn, dp).await?;
        Ok(DecisionRepo::insert(
            conn,
            &NewDecision {
                household_id: household,
                decision_point_id: point.id,
                subject,
                pipeline_version_id: None,
                stage_kind: StageKind::Human,
                value,
                confidence: 1.0,
                is_final: true,
                source: Some("human".into()),
                decided_by: member,
            },
        )
        .await?)
    }

    /// The latest final decision, else the latest decision.
    pub async fn current(
        conn: &mut PgConnection,
        dp: DecisionPointKey,
        subject: Subject,
    ) -> Result<Option<Decision>> {
        let point = dp_id(conn, dp).await?;
        Self::current_for(conn, point.id, subject).await
    }

    async fn current_for(
        conn: &mut PgConnection,
        dp: fa_domain::DecisionPointId,
        subject: Subject,
    ) -> Result<Option<Decision>> {
        if let Some(d) = DecisionRepo::latest_final(conn, dp, subject).await? {
            return Ok(Some(d));
        }
        Ok(DecisionRepo::latest(conn, dp, subject).await?)
    }

    pub async fn all_current(conn: &mut PgConnection, subject: Subject) -> Result<Vec<Decision>> {
        let mut out = Vec::new();
        for p in DecisionPointRepo::list(conn).await? {
            if let Some(d) = Self::current_for(conn, p.id, subject).await? {
                out.push(d);
            }
        }
        Ok(out)
    }
}
