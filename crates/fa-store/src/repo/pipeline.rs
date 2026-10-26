//! Decision points, pipeline versions and stages, and the append-only decision log.

use chrono::{DateTime, Utc};
use fa_domain::{
    Decision, DecisionId, DecisionPoint, DecisionPointId, DecisionPointKey, HouseholdId, MemberId,
    PipelineStage, PipelineVersion, PipelineVersionId, StageKind, Subject, SubjectType,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{enum_str, parse_enum};
use crate::{Result, StoreError};

// ---- decision points ----

pub struct DecisionPointRepo;

type DpRow = (Uuid, Uuid, String, serde_json::Value);

fn dp(r: DpRow) -> Result<DecisionPoint> {
    Ok(DecisionPoint {
        id: r.0.into(),
        household_id: r.1.into(),
        key: parse_enum(&r.2)?,
        default_value: r.3,
    })
}

impl DecisionPointRepo {
    pub async fn get_by_key(
        conn: &mut PgConnection,
        key: DecisionPointKey,
    ) -> Result<Option<DecisionPoint>> {
        let row: Option<DpRow> = sqlx::query_as(
            "SELECT id, household_id, key, default_value FROM decision_point WHERE key = $1",
        )
        .bind(key.as_str())
        .fetch_optional(conn)
        .await?;
        row.map(dp).transpose()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: DecisionPointId,
    ) -> Result<Option<DecisionPoint>> {
        let row: Option<DpRow> = sqlx::query_as(
            "SELECT id, household_id, key, default_value FROM decision_point WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(dp).transpose()
    }

    pub async fn list(conn: &mut PgConnection) -> Result<Vec<DecisionPoint>> {
        let rows: Vec<DpRow> = sqlx::query_as(
            "SELECT id, household_id, key, default_value FROM decision_point ORDER BY key",
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(dp).collect()
    }
}

// ---- pipelines ----

pub struct PipelineRepo;

type VersionRow = (Uuid, Uuid, Uuid, i32, bool, String, Option<Uuid>);

fn version(r: VersionRow) -> PipelineVersion {
    PipelineVersion {
        id: r.0.into(),
        household_id: r.1.into(),
        decision_point_id: r.2.into(),
        version: r.3,
        active: r.4,
        name: r.5,
        created_by: r.6.map(Into::into),
    }
}

type StageRow = (
    Uuid,
    Uuid,
    Uuid,
    i32,
    String,
    bool,
    f32,
    String,
    serde_json::Value,
);

fn stage(r: StageRow) -> Result<PipelineStage> {
    Ok(PipelineStage {
        id: r.0.into(),
        household_id: r.1.into(),
        pipeline_version_id: r.2.into(),
        ord: r.3,
        kind: parse_enum(&r.4)?,
        enabled: r.5,
        accept_threshold: r.6,
        on_unavailable: parse_enum(&r.7)?,
        config: r.8,
    })
}

const VERSION_COLS: &str = "id, household_id, decision_point_id, version, active, name, created_by";
const STAGE_COLS: &str =
    "id, household_id, pipeline_version_id, ord, kind, enabled, accept_threshold,
    on_unavailable, config";

impl PipelineRepo {
    pub async fn stages(
        conn: &mut PgConnection,
        version: PipelineVersionId,
    ) -> Result<Vec<PipelineStage>> {
        let rows: Vec<StageRow> = sqlx::query_as(&format!(
            "SELECT {STAGE_COLS} FROM pipeline_stage WHERE pipeline_version_id = $1 ORDER BY ord"
        ))
        .bind(version.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(stage).collect()
    }

    /// The active version of a decision point with its stages in `ord` order.
    pub async fn active_version(
        conn: &mut PgConnection,
        dp: DecisionPointId,
    ) -> Result<Option<(PipelineVersion, Vec<PipelineStage>)>> {
        let row: Option<VersionRow> = sqlx::query_as(&format!(
            "SELECT {VERSION_COLS} FROM pipeline_version WHERE decision_point_id = $1 AND active"
        ))
        .bind(dp.as_uuid())
        .fetch_optional(&mut *conn)
        .await?;
        let Some(row) = row else { return Ok(None) };
        let v = version(row);
        let stages = Self::stages(conn, v.id).await?;
        Ok(Some((v, stages)))
    }

    /// All versions of a decision point, newest first.
    pub async fn versions(
        conn: &mut PgConnection,
        dp: DecisionPointId,
    ) -> Result<Vec<PipelineVersion>> {
        let rows: Vec<VersionRow> = sqlx::query_as(&format!(
            "SELECT {VERSION_COLS} FROM pipeline_version WHERE decision_point_id = $1
             ORDER BY version DESC"
        ))
        .bind(dp.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(version).collect())
    }

    /// Inserts a version with its stages. When `v.active`, the other versions are deactivated first.
    pub async fn insert_version(
        conn: &mut PgConnection,
        v: &PipelineVersion,
        stages: &[PipelineStage],
    ) -> Result<()> {
        if v.active {
            sqlx::query("UPDATE pipeline_version SET active = false WHERE decision_point_id = $1")
                .bind(v.decision_point_id.as_uuid())
                .execute(&mut *conn)
                .await?;
        }
        sqlx::query(
            "INSERT INTO pipeline_version (id, household_id, decision_point_id, version, active, name,
                                           created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(v.id.as_uuid())
        .bind(v.household_id.as_uuid())
        .bind(v.decision_point_id.as_uuid())
        .bind(v.version)
        .bind(v.active)
        .bind(&v.name)
        .bind(v.created_by.map(|m| m.as_uuid()))
        .execute(&mut *conn)
        .await?;
        for s in stages {
            sqlx::query(
                "INSERT INTO pipeline_stage (id, household_id, pipeline_version_id, ord, kind, enabled,
                                             accept_threshold, on_unavailable, config)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(s.id.as_uuid())
            .bind(s.household_id.as_uuid())
            .bind(v.id.as_uuid())
            .bind(s.ord)
            .bind(enum_str(&s.kind))
            .bind(s.enabled)
            .bind(s.accept_threshold)
            .bind(enum_str(&s.on_unavailable))
            .bind(&s.config)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    /// Makes one version the active one of its decision point.
    pub async fn activate(conn: &mut PgConnection, version: PipelineVersionId) -> Result<()> {
        let dp: Option<Uuid> =
            sqlx::query_scalar("SELECT decision_point_id FROM pipeline_version WHERE id = $1")
                .bind(version.as_uuid())
                .fetch_optional(&mut *conn)
                .await?;
        let dp = dp.ok_or(StoreError::NotFound)?;
        sqlx::query("UPDATE pipeline_version SET active = false WHERE decision_point_id = $1")
            .bind(dp)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE pipeline_version SET active = true WHERE id = $1")
            .bind(version.as_uuid())
            .execute(conn)
            .await?;
        Ok(())
    }
}

// ---- decisions ----

pub struct DecisionRepo;

/// A decision to append.
#[derive(Debug, Clone)]
pub struct NewDecision {
    pub household_id: HouseholdId,
    pub decision_point_id: DecisionPointId,
    pub subject: Subject,
    pub pipeline_version_id: Option<PipelineVersionId>,
    pub stage_kind: StageKind,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub is_final: bool,
    /// Signal key or rule id; `"default"` for the human-stage default.
    pub source: Option<String>,
    pub decided_by: Option<MemberId>,
}

#[derive(sqlx::FromRow)]
struct DecisionRow {
    id: Uuid,
    household_id: Uuid,
    decision_point_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
    pipeline_version_id: Option<Uuid>,
    stage_kind: String,
    value: serde_json::Value,
    confidence: f32,
    is_final: bool,
    source: Option<String>,
    decided_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

const DECISION_COLS: &str = "id, household_id, decision_point_id, subject_type, subject_id,
    pipeline_version_id, stage_kind, value, confidence, is_final, source, decided_by, created_at";

impl TryFrom<DecisionRow> for Decision {
    type Error = StoreError;
    fn try_from(r: DecisionRow) -> Result<Self> {
        Ok(Decision {
            id: r.id.into(),
            household_id: r.household_id.into(),
            decision_point_id: r.decision_point_id.into(),
            subject_type: parse_enum(&r.subject_type)?,
            subject_id: r.subject_id,
            pipeline_version_id: r.pipeline_version_id.map(Into::into),
            stage_kind: parse_enum(&r.stage_kind)?,
            value: r.value,
            confidence: r.confidence,
            is_final: r.is_final,
            source: r.source,
            decided_by: r.decided_by.map(Into::into),
            created_at: r.created_at,
        })
    }
}

impl DecisionRepo {
    pub async fn insert(conn: &mut PgConnection, d: &NewDecision) -> Result<DecisionId> {
        let id = DecisionId::new();
        sqlx::query(
            "INSERT INTO decision (id, household_id, decision_point_id, subject_type, subject_id,
                                   pipeline_version_id, stage_kind, value, confidence, is_final,
                                   source, decided_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id.as_uuid())
        .bind(d.household_id.as_uuid())
        .bind(d.decision_point_id.as_uuid())
        .bind(enum_str(&d.subject.kind))
        .bind(d.subject.id)
        .bind(d.pipeline_version_id.map(|v| v.as_uuid()))
        .bind(enum_str(&d.stage_kind))
        .bind(&d.value)
        .bind(d.confidence)
        .bind(d.is_final)
        .bind(&d.source)
        .bind(d.decided_by.map(|m| m.as_uuid()))
        .execute(conn)
        .await?;
        Ok(id)
    }

    /// The most recent decision for a subject at a decision point.
    pub async fn latest(
        conn: &mut PgConnection,
        dp: DecisionPointId,
        subject: Subject,
    ) -> Result<Option<Decision>> {
        let row: Option<DecisionRow> = sqlx::query_as(&format!(
            "SELECT {DECISION_COLS} FROM decision
             WHERE decision_point_id = $1 AND subject_type = $2 AND subject_id = $3
             ORDER BY created_at DESC, id DESC LIMIT 1"
        ))
        .bind(dp.as_uuid())
        .bind(enum_str(&subject.kind))
        .bind(subject.id)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// The most recent final decision (a rule or a person), if any.
    pub async fn latest_final(
        conn: &mut PgConnection,
        dp: DecisionPointId,
        subject: Subject,
    ) -> Result<Option<Decision>> {
        let row: Option<DecisionRow> = sqlx::query_as(&format!(
            "SELECT {DECISION_COLS} FROM decision
             WHERE decision_point_id = $1 AND subject_type = $2 AND subject_id = $3 AND is_final
             ORDER BY created_at DESC, id DESC LIMIT 1"
        ))
        .bind(dp.as_uuid())
        .bind(enum_str(&subject.kind))
        .bind(subject.id)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Every row written by the latest run (same transaction timestamp) for a subject at a decision
    /// point: a multi-valued decision (member links) has one row per value.
    pub async fn latest_batch(
        conn: &mut PgConnection,
        dp: DecisionPointId,
        subject: Subject,
    ) -> Result<Vec<Decision>> {
        let rows: Vec<DecisionRow> = sqlx::query_as(&format!(
            "SELECT {DECISION_COLS} FROM decision
             WHERE decision_point_id = $1 AND subject_type = $2 AND subject_id = $3
               AND created_at = (SELECT max(created_at) FROM decision
                                 WHERE decision_point_id = $1 AND subject_type = $2 AND subject_id = $3)
             ORDER BY id"
        ))
        .bind(dp.as_uuid())
        .bind(enum_str(&subject.kind))
        .bind(subject.id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// The whole history of a subject, oldest first.
    pub async fn for_subject(conn: &mut PgConnection, subject: Subject) -> Result<Vec<Decision>> {
        let rows: Vec<DecisionRow> = sqlx::query_as(&format!(
            "SELECT {DECISION_COLS} FROM decision WHERE subject_type = $1 AND subject_id = $2
             ORDER BY created_at, id"
        ))
        .bind(enum_str(&subject.kind))
        .bind(subject.id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Removes the decisions of deleted subjects (there is no FK).
    pub async fn delete_for_subjects(
        conn: &mut PgConnection,
        kind: SubjectType,
        ids: &[Uuid],
    ) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM decision WHERE subject_type = $1 AND subject_id = ANY($2)")
                .bind(enum_str(&kind))
                .bind(ids)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}
