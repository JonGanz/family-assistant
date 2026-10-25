use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use fa_domain::{
    DeviceId, HouseholdId, MemberAway, MemberAwayId, MemberId, Routine, RoutineId, RoutineRun,
    RoutineRunId, RoutineStep, RoutineStepId, RunStatus,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct RoutineRepo;
pub struct RoutineRunRepo;
pub struct MemberAwayRepo;

/// A step to store (its `ord` is the position in the slice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutineStepSpec {
    pub text: String,
    pub target_time: Option<NaiveTime>,
}

/// A routine with its members and ordered steps.
#[derive(Debug, Clone)]
pub struct RoutineFull {
    pub routine: Versioned<Routine>,
    pub members: Vec<MemberId>,
    pub steps: Vec<RoutineStep>,
}

struct Row {
    id: Uuid,
    household_id: Uuid,
    name: String,
    starts_at: NaiveTime,
    done_by: NaiveTime,
    day_filter: String,
    school_calendar_id: Option<Uuid>,
    no_school_match: Vec<String>,
    show_on_dashboard: bool,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<Routine> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Routine {
                id: r.id.into(),
                household_id: r.household_id.into(),
                name: r.name,
                starts_at: r.starts_at,
                done_by: r.done_by,
                day_filter: parse_enum(&r.day_filter)?,
                school_calendar_id: r.school_calendar_id.map(Into::into),
                no_school_match: r.no_school_match,
                show_on_dashboard: r.show_on_dashboard,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

struct StepRow {
    id: Uuid,
    household_id: Uuid,
    routine_id: Uuid,
    ord: i32,
    text: String,
    target_time: Option<NaiveTime>,
}

impl From<StepRow> for RoutineStep {
    fn from(r: StepRow) -> Self {
        RoutineStep {
            id: r.id.into(),
            household_id: r.household_id.into(),
            routine_id: r.routine_id.into(),
            ord: r.ord,
            text: r.text,
            target_time: r.target_time,
        }
    }
}

async fn replace_members(
    conn: &mut PgConnection,
    household: HouseholdId,
    routine: RoutineId,
    members: &[MemberId],
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM routine_member WHERE routine_id = $1",
        routine.as_uuid()
    )
    .execute(&mut *conn)
    .await?;
    let ids: Vec<Uuid> = members.iter().map(|m| m.as_uuid()).collect();
    sqlx::query!(
        "INSERT INTO routine_member (routine_id, member_id, household_id)
         SELECT $1, m, $2 FROM unnest($3::uuid[]) AS m ON CONFLICT DO NOTHING",
        routine.as_uuid(),
        household.as_uuid(),
        &ids
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// A step keeps its id when the step at the same `ord` has the same text (its target time is
/// updated); other old steps are deleted and new ones inserted.
async fn replace_steps(
    conn: &mut PgConnection,
    household: HouseholdId,
    routine: RoutineId,
    specs: &[RoutineStepSpec],
) -> Result<()> {
    let existing = RoutineRepo::steps(conn, routine).await?;
    let by_ord: HashMap<i32, &RoutineStep> = existing.iter().map(|s| (s.ord, s)).collect();
    let mut keep: Vec<Uuid> = Vec::new();
    for (i, spec) in specs.iter().enumerate() {
        if let Some(s) = by_ord.get(&(i as i32)).filter(|s| s.text == spec.text) {
            keep.push(s.id.as_uuid());
        }
    }
    sqlx::query!(
        "DELETE FROM routine_step WHERE routine_id = $1 AND NOT (id = ANY($2))",
        routine.as_uuid(),
        &keep
    )
    .execute(&mut *conn)
    .await?;
    for (i, spec) in specs.iter().enumerate() {
        let ord = i as i32;
        match by_ord.get(&ord).filter(|s| s.text == spec.text) {
            Some(s) => {
                sqlx::query!(
                    "UPDATE routine_step SET target_time = $2 WHERE id = $1",
                    s.id.as_uuid(),
                    spec.target_time
                )
                .execute(&mut *conn)
                .await?;
            }
            None => {
                sqlx::query!(
                    "INSERT INTO routine_step (id, household_id, routine_id, ord, text, target_time)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                    RoutineStepId::new().as_uuid(),
                    household.as_uuid(),
                    routine.as_uuid(),
                    ord,
                    spec.text,
                    spec.target_time
                )
                .execute(&mut *conn)
                .await?;
            }
        }
    }
    Ok(())
}

impl RoutineRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        r: &Routine,
        members: &[MemberId],
        steps: &[RoutineStepSpec],
    ) -> Result<RoutineFull> {
        sqlx::query!(
            "INSERT INTO routine (id, household_id, name, starts_at, done_by, day_filter,
                                  school_calendar_id, no_school_match, show_on_dashboard)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            r.id.as_uuid(),
            r.household_id.as_uuid(),
            r.name,
            r.starts_at,
            r.done_by,
            enum_str(&r.day_filter),
            r.school_calendar_id.map(|c| c.as_uuid()),
            &r.no_school_match,
            r.show_on_dashboard
        )
        .execute(&mut *conn)
        .await?;
        replace_members(conn, r.household_id, r.id, members).await?;
        replace_steps(conn, r.household_id, r.id, steps).await?;
        Self::get(conn, r.id).await?.ok_or(StoreError::NotFound)
    }

    async fn full(conn: &mut PgConnection, routine: Versioned<Routine>) -> Result<RoutineFull> {
        let id = routine.value.id;
        Ok(RoutineFull {
            members: Self::members(conn, id).await?,
            steps: Self::steps(conn, id).await?,
            routine,
        })
    }

    pub async fn get(conn: &mut PgConnection, id: RoutineId) -> Result<Option<RoutineFull>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, starts_at, done_by, day_filter, school_calendar_id,
                    no_school_match, show_on_dashboard, archived_at, version, created_at, updated_at
             FROM routine WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(r) => Ok(Some(Self::full(conn, r.try_into()?).await?)),
            None => Ok(None),
        }
    }

    /// A non-archived routine by exact name.
    pub async fn get_by_name(conn: &mut PgConnection, name: &str) -> Result<Option<RoutineFull>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, starts_at, done_by, day_filter, school_calendar_id,
                    no_school_match, show_on_dashboard, archived_at, version, created_at, updated_at
             FROM routine WHERE name = $1 AND archived_at IS NULL ORDER BY created_at, id LIMIT 1",
            name
        )
        .fetch_optional(&mut *conn)
        .await?;
        match row {
            Some(r) => Ok(Some(Self::full(conn, r.try_into()?).await?)),
            None => Ok(None),
        }
    }

    pub async fn list(conn: &mut PgConnection, include_archived: bool) -> Result<Vec<RoutineFull>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, starts_at, done_by, day_filter, school_calendar_id,
                    no_school_match, show_on_dashboard, archived_at, version, created_at, updated_at
             FROM routine WHERE $1 OR archived_at IS NULL ORDER BY created_at, id",
            include_archived
        )
        .fetch_all(&mut *conn)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(Self::full(conn, r.try_into()?).await?);
        }
        Ok(out)
    }

    pub async fn members(conn: &mut PgConnection, id: RoutineId) -> Result<Vec<MemberId>> {
        let ids = sqlx::query_scalar!(
            "SELECT member_id FROM routine_member WHERE routine_id = $1 ORDER BY member_id",
            id.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(ids.into_iter().map(Into::into).collect())
    }

    pub async fn steps(conn: &mut PgConnection, id: RoutineId) -> Result<Vec<RoutineStep>> {
        let rows = sqlx::query_as!(
            StepRow,
            "SELECT id, household_id, routine_id, ord, text, target_time FROM routine_step
             WHERE routine_id = $1 ORDER BY ord",
            id.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Replaces the editable fields; `members` and `steps` replace the stored sets when `Some`.
    /// Returns the new version.
    pub async fn update(
        conn: &mut PgConnection,
        r: &Routine,
        members: Option<&[MemberId]>,
        steps: Option<&[RoutineStepSpec]>,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE routine SET name = $2, starts_at = $3, done_by = $4, day_filter = $5,
                    school_calendar_id = $6, no_school_match = $7, show_on_dashboard = $8,
                    version = version + 1
             WHERE id = $1 AND version = $9
             RETURNING version",
            r.id.as_uuid(),
            r.name,
            r.starts_at,
            r.done_by,
            enum_str(&r.day_filter),
            r.school_calendar_id.map(|c| c.as_uuid()),
            &r.no_school_match,
            r.show_on_dashboard,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        let Some(v) = v else {
            return Err(conflict_or_missing(conn, "routine", r.id.as_uuid()).await);
        };
        if let Some(m) = members {
            replace_members(conn, r.household_id, r.id, m).await?;
        }
        if let Some(s) = steps {
            replace_steps(conn, r.household_id, r.id, s).await?;
        }
        Ok(v)
    }

    pub async fn archive(conn: &mut PgConnection, id: RoutineId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE routine SET archived_at = COALESCE(archived_at, now()), version = version + 1 WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}

/// One step of a run with its tick.
#[derive(Debug, Clone)]
pub struct RunStepRow {
    pub step_id: RoutineStepId,
    pub ord: i32,
    pub text: String,
    pub target_time: Option<NaiveTime>,
    pub done_at: Option<DateTime<Utc>>,
    pub done_by: Option<MemberId>,
    pub done_via_device: Option<DeviceId>,
}

/// A run with its routine's display data and ordered steps.
#[derive(Debug, Clone)]
pub struct RunRow {
    pub run: RoutineRun,
    pub routine_name: String,
    pub starts_at: NaiveTime,
    pub done_by: NaiveTime,
    pub show_on_dashboard: bool,
    pub steps: Vec<RunStepRow>,
}

struct RunBase {
    id: Uuid,
    household_id: Uuid,
    routine_id: Uuid,
    member_id: Uuid,
    run_date: NaiveDate,
    status: String,
    completed_at: Option<DateTime<Utc>>,
    routine_name: String,
    starts_at: NaiveTime,
    done_by: NaiveTime,
    show_on_dashboard: bool,
}

async fn run_rows(conn: &mut PgConnection, base: Vec<RunBase>) -> Result<Vec<RunRow>> {
    let ids: Vec<Uuid> = base.iter().map(|b| b.id).collect();
    let step_rows = sqlx::query!(
        "SELECT rs.run_id, rs.step_id, s.ord, s.text, s.target_time, rs.done_at, rs.done_by,
                rs.done_via_device
         FROM routine_run_step rs JOIN routine_step s ON s.id = rs.step_id
         WHERE rs.run_id = ANY($1) ORDER BY rs.run_id, s.ord",
        &ids
    )
    .fetch_all(conn)
    .await?;
    let mut steps: HashMap<Uuid, Vec<RunStepRow>> = HashMap::new();
    for r in step_rows {
        steps.entry(r.run_id).or_default().push(RunStepRow {
            step_id: r.step_id.into(),
            ord: r.ord,
            text: r.text,
            target_time: r.target_time,
            done_at: r.done_at,
            done_by: r.done_by.map(Into::into),
            done_via_device: r.done_via_device.map(Into::into),
        });
    }
    base.into_iter()
        .map(|b| {
            Ok(RunRow {
                run: RoutineRun {
                    id: b.id.into(),
                    household_id: b.household_id.into(),
                    routine_id: b.routine_id.into(),
                    member_id: b.member_id.into(),
                    run_date: b.run_date,
                    status: parse_enum(&b.status)?,
                    completed_at: b.completed_at,
                },
                routine_name: b.routine_name,
                starts_at: b.starts_at,
                done_by: b.done_by,
                show_on_dashboard: b.show_on_dashboard,
                steps: steps.remove(&b.id).unwrap_or_default(),
            })
        })
        .collect()
}

impl RoutineRunRepo {
    /// Creates the run and one `routine_run_step` per step id. `false` when the run exists already.
    pub async fn ensure(
        conn: &mut PgConnection,
        routine: RoutineId,
        member: MemberId,
        date: NaiveDate,
        steps: &[RoutineStepId],
    ) -> Result<bool> {
        let run = RoutineRunId::new();
        let inserted = sqlx::query_scalar!(
            "INSERT INTO routine_run (id, household_id, routine_id, member_id, run_date)
             SELECT $1, household_id, id, $3, $4 FROM routine WHERE id = $2
             ON CONFLICT (routine_id, member_id, run_date) DO NOTHING
             RETURNING id",
            run.as_uuid(),
            routine.as_uuid(),
            member.as_uuid(),
            date
        )
        .fetch_optional(&mut *conn)
        .await?;
        if inserted.is_none() {
            return Ok(false);
        }
        let ids: Vec<Uuid> = steps.iter().map(|s| s.as_uuid()).collect();
        sqlx::query!(
            "INSERT INTO routine_run_step (run_id, step_id, household_id)
             SELECT $1, s, (SELECT household_id FROM routine_run WHERE id = $1)
             FROM unnest($2::uuid[]) AS s",
            run.as_uuid(),
            &ids
        )
        .execute(conn)
        .await?;
        Ok(true)
    }

    /// Runs of non-archived routines on `date`, optionally of one member.
    pub async fn for_date(
        conn: &mut PgConnection,
        date: NaiveDate,
        member: Option<MemberId>,
    ) -> Result<Vec<RunRow>> {
        let base = sqlx::query_as!(
            RunBase,
            "SELECT r.id, r.household_id, r.routine_id, r.member_id, r.run_date, r.status,
                    r.completed_at, ro.name AS routine_name, ro.starts_at, ro.done_by,
                    ro.show_on_dashboard
             FROM routine_run r JOIN routine ro ON ro.id = r.routine_id
             WHERE r.run_date = $1 AND ro.archived_at IS NULL
               AND ($2::uuid IS NULL OR r.member_id = $2)
             ORDER BY ro.starts_at, r.member_id, r.id",
            date,
            member.map(|m| m.as_uuid())
        )
        .fetch_all(&mut *conn)
        .await?;
        run_rows(conn, base).await
    }

    /// Ticks or unticks a step and recomputes the run status: `done` when every step is ticked,
    /// else `open`. Returns the new status.
    pub async fn tick(
        conn: &mut PgConnection,
        run: RoutineRunId,
        step: RoutineStepId,
        done: bool,
        by: Option<MemberId>,
        device: Option<DeviceId>,
    ) -> Result<RunStatus> {
        let n = sqlx::query!(
            "UPDATE routine_run_step SET done_at = CASE WHEN $3 THEN now() END,
                    done_by = CASE WHEN $3 THEN $4::uuid END,
                    done_via_device = CASE WHEN $3 THEN $5::uuid END
             WHERE run_id = $1 AND step_id = $2",
            run.as_uuid(),
            step.as_uuid(),
            done,
            by.map(|m| m.as_uuid()),
            device.map(|d| d.as_uuid())
        )
        .execute(&mut *conn)
        .await?
        .rows_affected();
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        let status = sqlx::query_scalar!(
            r#"UPDATE routine_run SET
                   status = CASE WHEN NOT EXISTS (SELECT 1 FROM routine_run_step
                                                  WHERE run_id = $1 AND done_at IS NULL)
                                 THEN 'done' ELSE 'open' END,
                   completed_at = CASE WHEN NOT EXISTS (SELECT 1 FROM routine_run_step
                                                        WHERE run_id = $1 AND done_at IS NULL)
                                       THEN COALESCE(completed_at, now()) END
               WHERE id = $1
               RETURNING status"#,
            run.as_uuid()
        )
        .fetch_one(conn)
        .await?;
        parse_enum(&status)
    }

    /// Open runs dated before `date` become `skipped`.
    pub async fn skip_before(conn: &mut PgConnection, date: NaiveDate) -> Result<u64> {
        Ok(sqlx::query!(
            "UPDATE routine_run SET status = 'skipped' WHERE status = 'open' AND run_date < $1",
            date
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Open runs on `date` with at least one unticked step.
    pub async fn open_with_steps_left(
        conn: &mut PgConnection,
        date: NaiveDate,
    ) -> Result<Vec<RunRow>> {
        let base = sqlx::query_as!(
            RunBase,
            "SELECT r.id, r.household_id, r.routine_id, r.member_id, r.run_date, r.status,
                    r.completed_at, ro.name AS routine_name, ro.starts_at, ro.done_by,
                    ro.show_on_dashboard
             FROM routine_run r JOIN routine ro ON ro.id = r.routine_id
             WHERE r.run_date = $1 AND r.status = 'open' AND ro.archived_at IS NULL
               AND EXISTS (SELECT 1 FROM routine_run_step rs WHERE rs.run_id = r.id AND rs.done_at IS NULL)
             ORDER BY ro.starts_at, r.member_id, r.id",
            date
        )
        .fetch_all(&mut *conn)
        .await?;
        run_rows(conn, base).await
    }
}

impl MemberAwayRepo {
    pub async fn insert(conn: &mut PgConnection, a: &MemberAway) -> Result<MemberAway> {
        let created = sqlx::query_scalar!(
            "INSERT INTO member_away (id, household_id, member_id, from_date, until_date, note)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING created_at",
            a.id.as_uuid(),
            a.household_id.as_uuid(),
            a.member_id.as_uuid(),
            a.from_date,
            a.until_date,
            a.note
        )
        .fetch_one(conn)
        .await?;
        Ok(MemberAway {
            created_at: created,
            ..a.clone()
        })
    }

    pub async fn get(conn: &mut PgConnection, id: MemberAwayId) -> Result<Option<MemberAway>> {
        let r = sqlx::query!(
            "SELECT id, household_id, member_id, from_date, until_date, note, created_at
             FROM member_away WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| MemberAway {
            id: r.id.into(),
            household_id: r.household_id.into(),
            member_id: r.member_id.into(),
            from_date: r.from_date,
            until_date: r.until_date,
            note: r.note,
            created_at: r.created_at,
        }))
    }

    pub async fn list(
        conn: &mut PgConnection,
        member: Option<MemberId>,
    ) -> Result<Vec<MemberAway>> {
        let rows = sqlx::query!(
            "SELECT id, household_id, member_id, from_date, until_date, note, created_at
             FROM member_away WHERE $1::uuid IS NULL OR member_id = $1
             ORDER BY from_date, id",
            member.map(|m| m.as_uuid())
        )
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| MemberAway {
                id: r.id.into(),
                household_id: r.household_id.into(),
                member_id: r.member_id.into(),
                from_date: r.from_date,
                until_date: r.until_date,
                note: r.note,
                created_at: r.created_at,
            })
            .collect())
    }

    pub async fn delete(conn: &mut PgConnection, id: MemberAwayId) -> Result<()> {
        let n = sqlx::query!("DELETE FROM member_away WHERE id = $1", id.as_uuid())
            .execute(conn)
            .await?
            .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Members away on the local `date` (inclusive range).
    pub async fn away_on(conn: &mut PgConnection, date: NaiveDate) -> Result<Vec<MemberId>> {
        let ids = sqlx::query_scalar!(
            "SELECT DISTINCT member_id FROM member_away WHERE from_date <= $1 AND $1 <= until_date",
            date
        )
        .fetch_all(conn)
        .await?;
        Ok(ids.into_iter().map(Into::into).collect())
    }
}
