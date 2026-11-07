//! fa-pipeline (M3): the decision pipeline engine. See arch/Ingest.md section 5 and plan/M3.md
//! "Pipeline engine".

use fa_domain::{
    Decision, DecisionId, DecisionPointKey, HouseholdId, MemberId, OnUnavailable, PipelineStage,
    PipelineStageId, PipelineVersionId, StageKind, Subject,
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
    /// An enabled `llm` stage follows the below-threshold `rules` stage: the default applies for
    /// now and the caller queues the deferred LLM job.
    pub pending_llm: bool,
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
        let mut pending_llm = false;
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
                StageKind::Llm => pending_llm = true,
                StageKind::Knn | StageKind::SystemOne => {
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
        pending_llm &= stored.is_none();
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
            pending_llm,
        })
    }

    /// Records the LLM stage's candidates of `version`. Candidates at or above the `llm` stage's
    /// threshold become `Decision`s with `stage_kind = llm`; otherwise nothing is written and the
    /// outcome is `human` with the default kept.
    pub async fn record_stage(
        conn: &mut PgConnection,
        subject: Subject,
        dp: DecisionPointKey,
        kind: StageKind,
        candidates: Vec<Candidate>,
        version: PipelineVersionId,
    ) -> Result<Outcome> {
        Self::record_stage_or(conn, subject, dp, kind, candidates, version, None).await
    }

    /// Like [`Pipeline::record_stage`], but a version without an `llm` stage is accepted when
    /// `fallback_threshold` is given (`llm_first` asks the model at every point whose stage is
    /// not switched off, D-107). An explicitly disabled stage is an error either way.
    pub async fn record_stage_or(
        conn: &mut PgConnection,
        subject: Subject,
        dp: DecisionPointKey,
        kind: StageKind,
        candidates: Vec<Candidate>,
        version: PipelineVersionId,
        fallback_threshold: Option<f32>,
    ) -> Result<Outcome> {
        if kind != StageKind::Llm {
            return Err(PipelineError::Invalid(
                "record_stage handles the llm stage only".into(),
            ));
        }
        let point = dp_id(conn, dp).await?;
        let stages = PipelineRepo::stages(conn, version).await?;
        let threshold = match stages.iter().find(|s| s.kind == StageKind::Llm) {
            Some(s) if s.enabled => s.accept_threshold,
            Some(_) => {
                return Err(PipelineError::Invalid(
                    "the version has no enabled llm stage".into(),
                ));
            }
            None => fallback_threshold.ok_or_else(|| {
                PipelineError::Invalid("the version has no enabled llm stage".into())
            })?,
        };
        let accepted = accept(dp, candidates, threshold);
        if accepted.is_empty() {
            return Ok(Outcome {
                values: vec![Candidate {
                    value: point.default_value.clone(),
                    confidence: 0.0,
                    source: "default".into(),
                }],
                stage: StageKind::Human,
                is_final: false,
                version,
                pending_llm: false,
            });
        }
        for c in &accepted {
            DecisionRepo::insert(
                conn,
                &NewDecision {
                    household_id: point.household_id,
                    decision_point_id: point.id,
                    subject,
                    pipeline_version_id: Some(version),
                    stage_kind: StageKind::Llm,
                    value: c.value.clone(),
                    confidence: c.confidence,
                    is_final: true,
                    source: Some(c.source.clone()),
                    decided_by: None,
                },
            )
            .await?;
        }
        Ok(Outcome {
            values: accepted,
            stage: StageKind::Llm,
            is_final: true,
            version,
            pending_llm: false,
        })
    }

    /// Saves a new active version with the `llm` stage inserted right before `human` (or updated
    /// in place). The previous version stays for rollback. Unchanged settings return the active
    /// version.
    pub async fn set_llm_stage(
        conn: &mut PgConnection,
        household: HouseholdId,
        dp: DecisionPointKey,
        enabled: bool,
        threshold: f32,
    ) -> Result<PipelineVersionId> {
        Self::set_stages(conn, household, dp, enabled, threshold, None).await
    }

    /// [`Pipeline::set_llm_stage`] plus, with `rules_threshold`, the `rules` stage's threshold
    /// (the `rules_first` confidence bar), in one new version. Unchanged settings return the
    /// active version.
    pub async fn set_stages(
        conn: &mut PgConnection,
        household: HouseholdId,
        dp: DecisionPointKey,
        enabled: bool,
        threshold: f32,
        rules_threshold: Option<f32>,
    ) -> Result<PipelineVersionId> {
        let point = dp_id(conn, dp).await?;
        let (active, stages) = PipelineRepo::active_version(conn, point.id)
            .await?
            .ok_or_else(|| {
                PipelineError::Invalid(format!("no active pipeline for {}", dp.as_str()))
            })?;
        let existing = stages.iter().find(|s| s.kind == StageKind::Llm);
        let rules_changed = rules_threshold.is_some_and(|t| {
            stages
                .iter()
                .any(|s| s.kind == StageKind::Rules && s.accept_threshold != t)
        });
        let llm_same = match existing {
            Some(s) => s.enabled == enabled && s.accept_threshold == threshold,
            None => !enabled,
        };
        if llm_same && !rules_changed {
            return Ok(active.id);
        }
        let new_id = PipelineVersionId::new();
        let llm = PipelineStage {
            id: PipelineStageId::new(),
            household_id: household,
            pipeline_version_id: new_id,
            ord: 0,
            kind: StageKind::Llm,
            enabled,
            accept_threshold: threshold,
            on_unavailable: existing.map_or(OnUnavailable::Queue, |s| s.on_unavailable),
            config: existing.map_or_else(|| serde_json::json!({}), |s| s.config.clone()),
        };
        let mut next: Vec<PipelineStage> = stages
            .iter()
            .filter(|s| s.kind != StageKind::Llm)
            .cloned()
            .collect();
        if let Some(t) = rules_threshold {
            for s in next.iter_mut().filter(|s| s.kind == StageKind::Rules) {
                s.accept_threshold = t;
            }
        }
        let at = next
            .iter()
            .position(|s| s.kind == StageKind::Human)
            .unwrap_or(next.len());
        if existing.is_some() || enabled {
            next.insert(at, llm);
        }
        for (i, s) in next.iter_mut().enumerate() {
            s.id = PipelineStageId::new();
            s.pipeline_version_id = new_id;
            s.ord = i32::try_from(i).unwrap_or(i32::MAX);
        }
        validate_version(&next)?;
        let latest = PipelineRepo::versions(conn, point.id)
            .await?
            .first()
            .map_or(active.version, |v| v.version.max(active.version));
        PipelineRepo::insert_version(
            conn,
            &fa_domain::PipelineVersion {
                id: new_id,
                household_id: household,
                decision_point_id: point.id,
                version: latest + 1,
                active: true,
                name: if next.iter().any(|s| s.kind == StageKind::Llm && s.enabled) {
                    "Rules + LLM"
                } else {
                    "Rules only"
                }
                .into(),
                created_by: None,
            },
            &next,
        )
        .await?;
        Ok(new_id)
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
