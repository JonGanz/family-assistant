//! `TaskService`: CRUD, completion, claims, views (plan/M2.md, Service APIs).
//! Authorisation is done in fa-api; this layer enforces data rules and returns typed errors.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use fa_domain::tasks::{
    NowWeights, ScoreInput, default_on_missed, format_mmdd, now_score, parse_mmdd,
};
use fa_domain::tasks_api::*;
use fa_domain::*;
use fa_store::{
    ChangeBus, HouseholdRepo, MemberAwayRepo, MemberRepo, NewTaskRow, RoutineFull, RoutineRepo,
    RoutineRunRepo, RoutineStepSpec, RunRow, Store, TaskDefinitionRepo, TaskFilter, TaskListRepo,
    TaskRepo, TaskRow, Tx, Versioned,
};

use crate::error::{Result, TaskError};
use crate::producers;
use crate::schedule;
use crate::strategy::{AdvanceCtx, assign_for, strategy_for};

/// Undo window of a completion.
pub const UNDO_MINUTES: i64 = 10;

pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

#[derive(Clone)]
pub struct TaskService {
    pub(crate) store: Store,
    pub(crate) clock: Clock,
}

/// Who is looking: a signed-in member or a dashboard device.
#[derive(Debug, Clone, Copy, Default)]
pub struct Viewer {
    pub member: Option<MemberId>,
    pub role: Option<Role>,
    pub device: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskViewMode {
    Now,
    Mine,
    #[default]
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    Open,
    Done,
    Skipped,
    Suggested,
    Any,
}

/// `GET /tasks` parameters. No `status` means open tasks. With `from` or `to`, the calendar
/// rule applies: `calendar_display` not hidden, open tasks, plus done `always` tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    pub view: TaskViewMode,
    pub member: Option<MemberId>,
    pub list: Option<TaskListId>,
    pub status: Option<StatusFilter>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub(crate) fn tz_of(h: &Household) -> Tz {
    h.tz.parse().unwrap_or(Tz::UTC)
}

pub(crate) fn local_date(t: DateTime<Utc>, tz: Tz) -> NaiveDate {
    t.with_timezone(&tz).date_naive()
}

pub(crate) async fn household_of(tx: &mut Tx, id: HouseholdId) -> Result<Household> {
    Ok(HouseholdRepo::get(tx, id)
        .await?
        .ok_or(TaskError::NotFound)?
        .value)
}

pub(crate) async fn publish_tasks(tx: &mut Tx, h: HouseholdId) -> Result<()> {
    ChangeBus::publish(tx, h, "tasks.updated", &serde_json::json!({})).await?;
    Ok(())
}

pub(crate) async fn publish_routines(tx: &mut Tx, h: HouseholdId, date: NaiveDate) -> Result<()> {
    ChangeBus::publish(
        tx,
        h,
        "routines.updated",
        &serde_json::json!({ "date": date }),
    )
    .await?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Undo {
    rotation_cursor: i32,
    last_done_at: Option<DateTime<Utc>>,
    next_due_at: Option<DateTime<Utc>>,
}

pub(crate) fn list_view(l: &Versioned<TaskList>) -> TaskListView {
    let v = &l.value;
    TaskListView {
        id: v.id,
        name: v.name.clone(),
        kind: v.kind,
        completion_policy: v.completion_policy,
        child_can_add: v.child_can_add,
        dashboard_add: v.dashboard_add,
        calendar_default: v.calendar_default,
        claim_ttl_minutes: v.claim_ttl_minutes,
        visible_on_dashboard: v.visible_on_dashboard,
        color: v.color.clone(),
        sort: v.sort,
        version: l.version,
    }
}

pub(crate) struct ViewEnv {
    pub now: DateTime<Utc>,
    pub tz: Tz,
    pub weights: NowWeights,
    pub away: Vec<MemberAway>,
}

impl ViewEnv {
    pub async fn load(tx: &mut Tx, h: &Household, now: DateTime<Utc>) -> Result<Self> {
        Ok(Self {
            now,
            tz: tz_of(h),
            weights: h.now_weights,
            away: MemberAwayRepo::list(tx, None).await?,
        })
    }

    fn is_away(&self, m: MemberId, date: NaiveDate) -> bool {
        self.away
            .iter()
            .any(|a| a.member_id == m && a.from_date <= date && date <= a.until_date)
    }

    pub fn task_view(&self, r: &TaskRow) -> TaskView {
        let t = &r.task.value;
        let date = t
            .due_at
            .map_or_else(|| local_date(self.now, self.tz), |d| local_date(d, self.tz));
        let assignee_away = t.status == TaskStatus::Open
            && !r.assignees.is_empty()
            && r.assignees.iter().all(|m| self.is_away(*m, date));
        let undo_until = (t.status == TaskStatus::Done && t.completion_undo.is_some())
            .then(|| t.completed_at.map(|c| c + Duration::minutes(UNDO_MINUTES)))
            .flatten();
        TaskView {
            id: t.id,
            list: t.list_id,
            list_name: r.list_name.clone(),
            definition: t.definition_id,
            title: t.title.clone(),
            notes: t.notes.clone(),
            due_at: t.due_at,
            visible_from: t.visible_from,
            status: t.status,
            origin: t.origin,
            priority_flag: t.priority_flag,
            calendar_display: t.calendar_display,
            assignees: r.assignees.clone(),
            assignee_away,
            steps: r
                .steps
                .iter()
                .map(|s| TaskStepView {
                    id: s.id,
                    ord: s.ord,
                    text: s.text.clone(),
                    done_at: s.done_at,
                    done_by: s.done_by,
                })
                .collect(),
            claimed_by: t.claimed_by,
            claimed_until: t.claimed_until,
            completed_at: t.completed_at,
            completed_by: t.completed_by,
            undo_until,
            score: now_score(
                &ScoreInput {
                    visible_from: t.visible_from,
                    due_at: t.due_at,
                    priority_flag: t.priority_flag,
                    origin: t.origin,
                },
                &self.weights,
                self.now,
            ),
            version: r.task.version,
        }
    }
}

pub(crate) async fn row_of(tx: &mut Tx, id: TaskId) -> Result<TaskRow> {
    TaskRepo::get_view_rows(
        tx,
        &TaskFilter {
            ids: Some(vec![id]),
            ..Default::default()
        },
    )
    .await?
    .into_iter()
    .next()
    .ok_or(TaskError::NotFound)
}

fn calendar_rule(r: &TaskRow) -> bool {
    let t = &r.task.value;
    t.calendar_display != CalendarDisplay::Hidden
        && match t.status {
            TaskStatus::Open => true,
            TaskStatus::Done => t.calendar_display == CalendarDisplay::Always,
            _ => false,
        }
}

fn status_list(f: StatusFilter) -> Option<Vec<TaskStatus>> {
    match f {
        StatusFilter::Open => Some(vec![TaskStatus::Open]),
        StatusFilter::Done => Some(vec![TaskStatus::Done]),
        StatusFilter::Skipped => Some(vec![TaskStatus::Skipped]),
        StatusFilter::Suggested => Some(vec![TaskStatus::Suggested]),
        StatusFilter::Any => None,
    }
}

fn cmp_now(a: &TaskView, b: &TaskView) -> std::cmp::Ordering {
    b.score
        .total_cmp(&a.score)
        .then_with(|| match (a.due_at, b.due_at) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
}

impl TaskService {
    pub fn new(store: Store) -> Self {
        Self::with_clock(store, Arc::new(Utc::now))
    }

    pub fn with_clock(store: Store, clock: Clock) -> Self {
        Self { store, clock }
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    // ---- reading tasks ----

    pub async fn list_tasks(
        &self,
        household: HouseholdId,
        q: &TaskQuery,
        viewer: Viewer,
    ) -> Result<Vec<TaskView>> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let member = match q.view {
            TaskViewMode::Mine => match q.member.or(viewer.member) {
                Some(m) => Some(m),
                None => return Ok(Vec::new()),
            },
            _ => q.member,
        };
        let calendar = q.from.is_some() || q.to.is_some();
        let statuses = if q.view == TaskViewMode::Now {
            Some(vec![TaskStatus::Open])
        } else if calendar {
            q.status.and_then(status_list)
        } else {
            q.status.map_or(Some(vec![TaskStatus::Open]), status_list)
        };
        let mut rows = TaskRepo::get_view_rows(
            &mut tx,
            &TaskFilter {
                statuses,
                list: q.list,
                member,
                due_from: q.from,
                due_to: q.to,
                visible_before: (q.view == TaskViewMode::Now).then_some(now),
                calendar_display_not_hidden: calendar,
                ..Default::default()
            },
        )
        .await?;
        if calendar {
            rows.retain(calendar_rule);
        }
        if viewer.role == Some(Role::Child) {
            rows.retain(|r| {
                r.task.value.status != TaskStatus::Suggested
                    && (r.assignees.is_empty()
                        || viewer.member.is_some_and(|m| r.assignees.contains(&m)))
            });
        }
        let env = ViewEnv::load(&mut tx, &h, now).await?;
        let mut views: Vec<TaskView> = rows.iter().map(|r| env.task_view(r)).collect();
        if q.view == TaskViewMode::Now {
            views.sort_by(cmp_now);
            views.truncate(h.now_list_size.max(1) as usize);
        }
        Ok(views)
    }

    pub async fn get_task(&self, household: HouseholdId, id: TaskId) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let row = row_of(&mut tx, id).await?;
        let env = ViewEnv::load(&mut tx, &h, now).await?;
        Ok(env.task_view(&row))
    }

    /// Tasks of the dashboard: open ones visible now (suggested excluded), plus `until_done` and
    /// `always` tasks due in `[from, to)` (done `always` included). Lists hidden from the
    /// dashboard are left out.
    pub async fn dashboard_tasks(
        &self,
        household: HouseholdId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<TaskView>> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let hidden: HashSet<TaskListId> = TaskListRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .filter(|l| !l.value.visible_on_dashboard)
            .map(|l| l.value.id)
            .collect();
        let mut rows = TaskRepo::get_view_rows(
            &mut tx,
            &TaskFilter {
                statuses: Some(vec![TaskStatus::Open]),
                visible_before: Some(now),
                ..Default::default()
            },
        )
        .await?;
        let ids: HashSet<TaskId> = rows.iter().map(|r| r.task.value.id).collect();
        let cal = TaskRepo::get_view_rows(
            &mut tx,
            &TaskFilter {
                due_from: Some(from),
                due_to: Some(to),
                calendar_display_not_hidden: true,
                ..Default::default()
            },
        )
        .await?;
        rows.extend(
            cal.into_iter()
                .filter(|r| calendar_rule(r) && !ids.contains(&r.task.value.id)),
        );
        rows.retain(|r| !hidden.contains(&r.task.value.list_id));
        let env = ViewEnv::load(&mut tx, &h, now).await?;
        Ok(rows.iter().map(|r| env.task_view(r)).collect())
    }

    // ---- writing tasks ----

    pub async fn create_task(
        &self,
        household: HouseholdId,
        new: NewTask,
        by: Option<MemberId>,
        device: Option<DeviceId>,
    ) -> Result<TaskView> {
        let _ = device;
        let now = self.now();
        let title = new.title.trim().to_owned();
        if title.is_empty() {
            return Err(TaskError::invalid("title", "a title is required"));
        }
        let creator = by.or(new.member);
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let list = TaskListRepo::get(&mut tx, new.list)
            .await?
            .filter(|l| l.value.archived_at.is_none())
            .ok_or_else(|| TaskError::invalid("list", "unknown task list"))?
            .value;
        let id = TaskId::new();
        let created = TaskRepo::insert(
            &mut tx,
            &NewTaskRow {
                id,
                household_id: household,
                list_id: list.id,
                definition_id: None,
                weather_rule_id: None,
                title,
                notes: new.notes,
                due_at: new.due_at,
                visible_from: None,
                status: TaskStatus::Open,
                origin: TaskOrigin::Manual,
                source_ref: None,
                priority_flag: new.priority_flag.unwrap_or(PriorityFlag::Normal),
                calendar_display: new.calendar_display.unwrap_or(list.calendar_default),
                created_by: creator,
            },
            &new.assignees,
            &new.steps,
        )
        .await?
        .ok_or(TaskError::Conflict)?;
        TaskRepo::set_created_at(&mut tx, id, now).await?;
        producers::assigned(
            &mut tx,
            &created.value,
            &new.assignees,
            creator,
            list.name.clone(),
        )
        .await?;
        publish_tasks(&mut tx, household).await?;
        let row = row_of(&mut tx, id).await?;
        let env = ViewEnv::load(&mut tx, &h, now).await?;
        let view = env.task_view(&row);
        tx.commit().await?;
        Ok(view)
    }

    pub async fn update_task(
        &self,
        household: HouseholdId,
        id: TaskId,
        patch: TaskPatch,
        expected_version: i32,
    ) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let cur = row_of(&mut tx, id).await?;
        let mut t = cur.task.value.clone();
        if let Some(title) = patch.title {
            if title.trim().is_empty() {
                return Err(TaskError::invalid("title", "a title is required"));
            }
            t.title = title.trim().to_owned();
        }
        if let Some(n) = patch.notes {
            t.notes = n;
        }
        if let Some(d) = patch.due_at {
            t.due_at = d;
        }
        if let Some(p) = patch.priority_flag {
            t.priority_flag = p;
        }
        if let Some(c) = patch.calendar_display {
            t.calendar_display = c;
        }
        TaskRepo::update(
            &mut tx,
            &t,
            patch.assignees.as_deref(),
            patch.steps.as_deref(),
            expected_version,
        )
        .await?;
        if let Some(new) = &patch.assignees {
            let added: Vec<MemberId> = new
                .iter()
                .copied()
                .filter(|m| !cur.assignees.contains(m))
                .collect();
            if t.status == TaskStatus::Open {
                producers::assigned(&mut tx, &t, &added, None, cur.list_name.clone()).await?;
            }
        }
        publish_tasks(&mut tx, household).await?;
        let row = row_of(&mut tx, id).await?;
        let env = ViewEnv::load(&mut tx, &h, now).await?;
        let view = env.task_view(&row);
        tx.commit().await?;
        Ok(view)
    }

    pub async fn archive_task(&self, household: HouseholdId, id: TaskId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        TaskRepo::archive(&mut tx, id).await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn finish(&self, mut tx: Tx, household: HouseholdId, id: TaskId) -> Result<TaskView> {
        publish_tasks(&mut tx, household).await?;
        let h = household_of(&mut tx, household).await?;
        let row = row_of(&mut tx, id).await?;
        let env = ViewEnv::load(&mut tx, &h, self.now()).await?;
        let view = env.task_view(&row);
        tx.commit().await?;
        Ok(view)
    }

    async fn open_task(
        tx: &mut Tx,
        id: TaskId,
        allow_suggested: bool,
    ) -> Result<(Task, Vec<MemberId>)> {
        let row = row_of(tx, id).await?;
        let t = row.task.value;
        let ok =
            t.status == TaskStatus::Open || (allow_suggested && t.status == TaskStatus::Suggested);
        if !ok {
            return Err(TaskError::WrongStatus {
                expected: if allow_suggested {
                    "open or suggested"
                } else {
                    "open"
                },
                actual: t.status.as_str().to_owned(),
            });
        }
        Ok((t, row.assignees))
    }

    pub async fn complete(
        &self,
        household: HouseholdId,
        task: TaskId,
        member: Option<MemberId>,
        device: Option<DeviceId>,
    ) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let tz = tz_of(&h);
        let (t, assignees) = Self::open_task(&mut tx, task, false).await?;
        let mut undo = None;
        if let Some(did) = t.definition_id
            && let Some(def) = TaskDefinitionRepo::lock_blocking(&mut tx, did).await?
        {
            undo = serde_json::to_value(Undo {
                rotation_cursor: def.rotation_cursor,
                last_done_at: def.last_done_at,
                next_due_at: def.next_due_at,
            })
            .ok();
            let cands = TaskDefinitionRepo::candidates(&mut tx, did).await?;
            let cursor = strategy_for(def.strategy)
                .advance(&AdvanceCtx {
                    candidates: &cands,
                    advance_on: def.advance_on,
                    skip_advances: def.skip_advances,
                    assignees: &assignees,
                    completed_by: member,
                    skipped: false,
                })
                .unwrap_or(def.rotation_cursor);
            let next = match def.schedule {
                Schedule::AfterDone { .. } => schedule::after_done_next(&def, tz, now),
                Schedule::Rrule { .. } => def.next_due_at,
            };
            TaskDefinitionRepo::set_schedule_state(&mut tx, did, cursor, Some(now), next).await?;
        }
        TaskRepo::set_status(&mut tx, task, TaskStatus::Done, member, device, undo).await?;
        TaskRepo::set_completed_at(&mut tx, task, now).await?;
        self.finish(tx, household, task).await
    }

    pub async fn skip(
        &self,
        household: HouseholdId,
        task: TaskId,
        by: Option<MemberId>,
    ) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let tz = tz_of(&h);
        let (t, assignees) = Self::open_task(&mut tx, task, true).await?;
        if t.status == TaskStatus::Open
            && let Some(did) = t.definition_id
            && let Some(def) = TaskDefinitionRepo::lock_blocking(&mut tx, did).await?
        {
            let cands = TaskDefinitionRepo::candidates(&mut tx, did).await?;
            let cursor = strategy_for(def.strategy)
                .advance(&AdvanceCtx {
                    candidates: &cands,
                    advance_on: def.advance_on,
                    skip_advances: def.skip_advances,
                    assignees: &assignees,
                    completed_by: None,
                    skipped: true,
                })
                .unwrap_or(def.rotation_cursor);
            let next = match def.schedule {
                Schedule::AfterDone { .. } => schedule::after_done_next(&def, tz, now),
                Schedule::Rrule { .. } => def.next_due_at,
            };
            TaskDefinitionRepo::set_schedule_state(&mut tx, did, cursor, def.last_done_at, next)
                .await?;
        }
        TaskRepo::set_status(&mut tx, task, TaskStatus::Skipped, by, None, None).await?;
        self.finish(tx, household, task).await
    }

    /// `suggested` to `open`.
    pub async fn accept(&self, household: HouseholdId, task: TaskId) -> Result<TaskView> {
        let mut tx = self.store.tx_for(household).await?;
        let row = row_of(&mut tx, task).await?;
        if row.task.value.status != TaskStatus::Suggested {
            return Err(TaskError::WrongStatus {
                expected: "suggested",
                actual: row.task.value.status.as_str().to_owned(),
            });
        }
        TaskRepo::set_status(&mut tx, task, TaskStatus::Open, None, None, None).await?;
        let pool = row
            .task
            .value
            .source_ref
            .as_ref()
            .and_then(|s| s.get("strategy"))
            .and_then(|s| s.as_str())
            == Some("pool");
        if !pool {
            producers::assigned(
                &mut tx,
                &row.task.value,
                &row.assignees,
                None,
                row.list_name.clone(),
            )
            .await?;
        }
        self.finish(tx, household, task).await
    }

    /// Back to `open`. Within 10 minutes of a completion it also restores the definition and
    /// deletes the untouched occurrences made since.
    pub async fn reopen(&self, household: HouseholdId, task: TaskId) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let row = row_of(&mut tx, task).await?;
        let t = &row.task.value;
        if !matches!(t.status, TaskStatus::Done | TaskStatus::Skipped) {
            return Err(TaskError::WrongStatus {
                expected: "done or skipped",
                actual: t.status.as_str().to_owned(),
            });
        }
        if t.status == TaskStatus::Done
            && let (Some(did), Some(done_at), Some(undo)) =
                (t.definition_id, t.completed_at, t.completion_undo.clone())
            && now <= done_at + Duration::minutes(UNDO_MINUTES)
            && let Ok(undo) = serde_json::from_value::<Undo>(undo)
            && TaskDefinitionRepo::lock_blocking(&mut tx, did)
                .await?
                .is_some()
        {
            TaskDefinitionRepo::set_schedule_state(
                &mut tx,
                did,
                undo.rotation_cursor,
                undo.last_done_at,
                undo.next_due_at,
            )
            .await?;
            TaskRepo::delete_created_after(&mut tx, did, done_at).await?;
        }
        TaskRepo::reopen(&mut tx, task).await?;
        self.finish(tx, household, task).await
    }

    async fn pool_check(tx: &mut Tx, t: &Task) -> Result<bool> {
        if let Some(did) = t.definition_id {
            return Ok(TaskDefinitionRepo::get(tx, did)
                .await?
                .is_some_and(|d| d.value.strategy == Strategy::Pool));
        }
        Ok(t.source_ref
            .as_ref()
            .and_then(|s| s.get("strategy"))
            .and_then(|s| s.as_str())
            == Some("pool"))
    }

    /// A pool candidate (or an adult) takes a task until the claim expires.
    pub async fn claim(
        &self,
        household: HouseholdId,
        task: TaskId,
        member: MemberId,
    ) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let tz = tz_of(&h);
        let (t, assignees) = Self::open_task(&mut tx, task, false).await?;
        if !Self::pool_check(&mut tx, &t).await? {
            return Err(TaskError::NotPool);
        }
        let adult = MemberRepo::get(&mut tx, member)
            .await?
            .is_some_and(|m| m.value.role == Role::Adult);
        if !adult && !assignees.contains(&member) {
            return Err(TaskError::NotCandidate);
        }
        if let (Some(other), Some(until)) = (t.claimed_by, t.claimed_until)
            && other != member
            && until > now
        {
            return Err(TaskError::ClaimedByOther);
        }
        let ttl = TaskListRepo::get(&mut tx, t.list_id)
            .await?
            .and_then(|l| l.value.claim_ttl_minutes);
        let until = match ttl {
            Some(m) => now + Duration::minutes(i64::from(m)),
            None => fa_domain::tasks::due_instant(local_date(now, tz), None, tz).max(now),
        };
        TaskRepo::set_claim(&mut tx, task, Some(member), Some(until)).await?;
        self.finish(tx, household, task).await
    }

    pub async fn release(
        &self,
        household: HouseholdId,
        task: TaskId,
        member: MemberId,
    ) -> Result<TaskView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let (t, _) = Self::open_task(&mut tx, task, false).await?;
        if let (Some(other), Some(until)) = (t.claimed_by, t.claimed_until)
            && other != member
            && until > now
        {
            let adult = MemberRepo::get(&mut tx, member)
                .await?
                .is_some_and(|m| m.value.role == Role::Adult);
            if !adult {
                return Err(TaskError::ClaimedByOther);
            }
        }
        TaskRepo::set_claim(&mut tx, task, None, None).await?;
        self.finish(tx, household, task).await
    }

    /// Ticking every step does not complete the task.
    pub async fn tick_step(
        &self,
        household: HouseholdId,
        task: TaskId,
        step: TaskStepId,
        done: bool,
        member: Option<MemberId>,
        device: Option<DeviceId>,
    ) -> Result<TaskView> {
        let _ = device;
        let mut tx = self.store.tx_for(household).await?;
        Self::open_task(&mut tx, task, false).await?;
        TaskRepo::tick_step(&mut tx, task, step, done, member).await?;
        self.finish(tx, household, task).await
    }

    // ---- lists ----

    pub async fn list_task_lists(&self, household: HouseholdId) -> Result<Vec<TaskListView>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(TaskListRepo::list(&mut tx, false)
            .await?
            .iter()
            .map(list_view)
            .collect())
    }

    pub async fn get_task_list(
        &self,
        household: HouseholdId,
        id: TaskListId,
    ) -> Result<TaskListView> {
        let mut tx = self.store.tx_for(household).await?;
        let l = TaskListRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        Ok(list_view(&l))
    }

    pub async fn create_task_list(
        &self,
        household: HouseholdId,
        i: TaskListInput,
    ) -> Result<TaskListView> {
        if i.name.trim().is_empty() {
            return Err(TaskError::invalid("name", "a name is required"));
        }
        if i.claim_ttl_minutes.is_some_and(|m| m < 1) {
            return Err(TaskError::invalid(
                "claim_ttl_minutes",
                "must be at least 1",
            ));
        }
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let l = TaskListRepo::insert(
            &mut tx,
            &TaskList {
                id: TaskListId::new(),
                household_id: household,
                name: i.name.trim().to_owned(),
                kind: i.kind,
                completion_policy: i.completion_policy,
                child_can_add: i.child_can_add,
                dashboard_add: i.dashboard_add,
                calendar_default: i.calendar_default,
                claim_ttl_minutes: i.claim_ttl_minutes,
                visible_on_dashboard: i.visible_on_dashboard,
                color: i.color,
                sort: i.sort,
                archived_at: None,
                created_at: now,
                updated_at: now,
            },
        )
        .await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(list_view(&l))
    }

    pub async fn update_task_list(
        &self,
        household: HouseholdId,
        id: TaskListId,
        patch: TaskListPatch,
        expected_version: i32,
    ) -> Result<TaskListView> {
        let mut tx = self.store.tx_for(household).await?;
        TaskListRepo::update(&mut tx, id, &patch, expected_version).await?;
        let l = TaskListRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(list_view(&l))
    }

    pub async fn archive_task_list(&self, household: HouseholdId, id: TaskListId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        TaskListRepo::archive(&mut tx, id).await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- definitions ----

    async fn definition_view(
        tx: &mut Tx,
        d: &Versioned<TaskDefinition>,
        tz: Tz,
    ) -> Result<TaskDefinitionView> {
        let v = &d.value;
        let cands = TaskDefinitionRepo::candidates(tx, v.id).await?;
        let next_assignee = match (v.strategy, v.next_due_at) {
            (Strategy::Pool, _) => None,
            (_, Some(due)) => assign_for(tx, v, &cands, local_date(due, tz))
                .await?
                .first()
                .copied(),
            (_, None) => cands.first().map(|c| c.0),
        };
        Ok(TaskDefinitionView {
            id: v.id,
            list: v.list_id,
            title: v.title.clone(),
            notes: v.notes.clone(),
            steps_template: v.steps_template.clone(),
            schedule: v.schedule.clone(),
            start_date: v.start_date,
            due_time: v.due_time,
            lead_days: v.lead_days,
            on_missed: v.on_missed,
            strategy: v.strategy,
            candidates: cands.iter().map(|c| c.0).collect(),
            advance_on: v.advance_on,
            skip_advances: v.skip_advances,
            next_assignee,
            priority_flag: v.priority_flag,
            season: v.season.map(|(f, u)| Season {
                from: format_mmdd(f),
                until: format_mmdd(u),
            }),
            season_condition: v.season_condition.clone(),
            condition_ok: v.condition_ok,
            last_done_at: v.last_done_at,
            next_due_at: v.next_due_at,
            paused: v.paused,
            version: d.version,
        })
    }

    pub async fn list_definitions(
        &self,
        household: HouseholdId,
        list: Option<TaskListId>,
    ) -> Result<Vec<TaskDefinitionView>> {
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let defs = TaskDefinitionRepo::list(&mut tx, list, false).await?;
        let mut out = Vec::with_capacity(defs.len());
        for d in &defs {
            out.push(Self::definition_view(&mut tx, d, tz).await?);
        }
        Ok(out)
    }

    pub async fn get_definition(
        &self,
        household: HouseholdId,
        id: TaskDefinitionId,
    ) -> Result<TaskDefinitionView> {
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let d = TaskDefinitionRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        Self::definition_view(&mut tx, &d, tz).await
    }

    pub async fn create_definition(
        &self,
        household: HouseholdId,
        i: TaskDefinitionInput,
    ) -> Result<TaskDefinitionView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        TaskListRepo::get(&mut tx, i.list)
            .await?
            .filter(|l| l.value.archived_at.is_none())
            .ok_or_else(|| TaskError::invalid("list", "unknown task list"))?;
        let mut d = TaskDefinition {
            id: TaskDefinitionId::new(),
            household_id: household,
            list_id: i.list,
            title: i.title,
            notes: i.notes,
            steps_template: i.steps_template,
            on_missed: i
                .on_missed
                .unwrap_or_else(|| default_on_missed(i.schedule.kind())),
            schedule: i.schedule,
            start_date: i.start_date,
            due_time: i.due_time,
            lead_days: i.lead_days,
            strategy: i.strategy,
            advance_on: i.advance_on,
            skip_advances: i.skip_advances,
            rotation_cursor: 0,
            priority_flag: i.priority_flag,
            season: parse_season(i.season.as_ref())?,
            season_condition: i.season_condition.filter(|s| !s.trim().is_empty()),
            condition_ok: None,
            condition_checked_at: None,
            last_done_at: None,
            next_due_at: None,
            paused: i.paused,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        validate_definition(&d)?;
        d.next_due_at = recompute_next(&d, tz, now);
        let ins = TaskDefinitionRepo::insert(&mut tx, &d, &i.candidates).await?;
        publish_tasks(&mut tx, household).await?;
        let v = Self::definition_view(&mut tx, &ins, tz).await?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn update_definition(
        &self,
        household: HouseholdId,
        id: TaskDefinitionId,
        p: TaskDefinitionPatch,
        expected_version: i32,
    ) -> Result<TaskDefinitionView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let cur = TaskDefinitionRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        let mut d = cur.value.clone();
        let mut reschedule = false;
        if let Some(l) = p.list {
            TaskListRepo::get(&mut tx, l)
                .await?
                .filter(|l| l.value.archived_at.is_none())
                .ok_or_else(|| TaskError::invalid("list", "unknown task list"))?;
            d.list_id = l;
        }
        if let Some(v) = p.title {
            d.title = v;
        }
        if let Some(v) = p.notes {
            d.notes = v;
        }
        if let Some(v) = p.steps_template {
            d.steps_template = v;
        }
        if let Some(v) = p.schedule {
            reschedule |= v != d.schedule;
            d.schedule = v;
        }
        if let Some(v) = p.start_date {
            reschedule |= v != d.start_date;
            d.start_date = v;
        }
        if let Some(v) = p.due_time {
            reschedule |= v != d.due_time;
            d.due_time = v;
        }
        if let Some(v) = p.lead_days {
            d.lead_days = v;
        }
        if let Some(v) = p.on_missed {
            d.on_missed = v;
        }
        if let Some(v) = p.strategy {
            d.strategy = v;
        }
        if let Some(v) = p.advance_on {
            d.advance_on = v;
        }
        if let Some(v) = p.skip_advances {
            d.skip_advances = v;
        }
        if let Some(v) = p.priority_flag {
            d.priority_flag = v;
        }
        if let Some(v) = p.season {
            let s = parse_season(v.as_ref())?;
            reschedule |= s != d.season;
            d.season = s;
        }
        if let Some(v) = p.season_condition {
            d.season_condition = v.filter(|s| !s.trim().is_empty());
        }
        if let Some(v) = p.paused {
            d.paused = v;
        }
        validate_definition(&d)?;
        if reschedule {
            d.next_due_at = recompute_next(&d, tz, now);
        }
        TaskDefinitionRepo::update(&mut tx, &d, p.candidates.as_deref(), expected_version).await?;
        let now_def = TaskDefinitionRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        publish_tasks(&mut tx, household).await?;
        let v = Self::definition_view(&mut tx, &now_def, tz).await?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn archive_definition(
        &self,
        household: HouseholdId,
        id: TaskDefinitionId,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        TaskDefinitionRepo::archive(&mut tx, id).await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- routines ----

    fn routine_view(f: &RoutineFull) -> RoutineView {
        let r = &f.routine.value;
        RoutineView {
            id: r.id,
            name: r.name.clone(),
            members: f.members.clone(),
            steps: f
                .steps
                .iter()
                .map(|s| RoutineStepView {
                    id: s.id,
                    ord: s.ord,
                    text: s.text.clone(),
                    target_time: s.target_time,
                })
                .collect(),
            starts_at: r.starts_at,
            done_by: r.done_by,
            day_filter: r.day_filter,
            school_calendar: r.school_calendar_id,
            no_school_match: r.no_school_match.clone(),
            show_on_dashboard: r.show_on_dashboard,
            version: f.routine.version,
        }
    }

    fn specs(steps: &[RoutineStepInput]) -> Vec<RoutineStepSpec> {
        steps
            .iter()
            .map(|s| RoutineStepSpec {
                text: s.text.clone(),
                target_time: s.target_time,
            })
            .collect()
    }

    pub async fn list_routines(&self, household: HouseholdId) -> Result<Vec<RoutineView>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(RoutineRepo::list(&mut tx, false)
            .await?
            .iter()
            .map(Self::routine_view)
            .collect())
    }

    pub async fn get_routine(&self, household: HouseholdId, id: RoutineId) -> Result<RoutineView> {
        let mut tx = self.store.tx_for(household).await?;
        let f = RoutineRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        Ok(Self::routine_view(&f))
    }

    pub async fn create_routine(
        &self,
        household: HouseholdId,
        i: RoutineInput,
    ) -> Result<RoutineView> {
        if i.name.trim().is_empty() {
            return Err(TaskError::invalid("name", "a name is required"));
        }
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let r = Routine {
            id: RoutineId::new(),
            household_id: household,
            name: i.name.trim().to_owned(),
            starts_at: i.starts_at,
            done_by: i.done_by,
            day_filter: i.day_filter,
            school_calendar_id: i.school_calendar,
            no_school_match: i.no_school_match.unwrap_or_else(|| {
                ["no school", "holiday", "inset", "closed"]
                    .map(String::from)
                    .to_vec()
            }),
            show_on_dashboard: i.show_on_dashboard,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let f = RoutineRepo::insert(&mut tx, &r, &i.members, &Self::specs(&i.steps)).await?;
        publish_routines(&mut tx, household, local_date(now, tz)).await?;
        tx.commit().await?;
        Ok(Self::routine_view(&f))
    }

    pub async fn update_routine(
        &self,
        household: HouseholdId,
        id: RoutineId,
        p: RoutinePatch,
        expected_version: i32,
    ) -> Result<RoutineView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let cur = RoutineRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        let mut r = cur.routine.value.clone();
        if let Some(v) = p.name {
            r.name = v;
        }
        if let Some(v) = p.starts_at {
            r.starts_at = v;
        }
        if let Some(v) = p.done_by {
            r.done_by = v;
        }
        if let Some(v) = p.day_filter {
            r.day_filter = v;
        }
        if let Some(v) = p.school_calendar {
            r.school_calendar_id = v;
        }
        if let Some(v) = p.no_school_match {
            r.no_school_match = v;
        }
        if let Some(v) = p.show_on_dashboard {
            r.show_on_dashboard = v;
        }
        let specs = p.steps.as_deref().map(Self::specs);
        RoutineRepo::update(
            &mut tx,
            &r,
            p.members.as_deref(),
            specs.as_deref(),
            expected_version,
        )
        .await?;
        let f = RoutineRepo::get(&mut tx, id)
            .await?
            .ok_or(TaskError::NotFound)?;
        publish_routines(&mut tx, household, local_date(now, tz)).await?;
        tx.commit().await?;
        Ok(Self::routine_view(&f))
    }

    pub async fn archive_routine(&self, household: HouseholdId, id: RoutineId) -> Result<()> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        RoutineRepo::archive(&mut tx, id).await?;
        publish_routines(&mut tx, household, local_date(now, tz)).await?;
        tx.commit().await?;
        Ok(())
    }

    fn run_view(r: &RunRow) -> RoutineRunView {
        let done = r.steps.iter().filter(|s| s.done_at.is_some()).count() as i32;
        RoutineRunView {
            id: r.run.id,
            routine: r.run.routine_id,
            routine_name: r.routine_name.clone(),
            member: r.run.member_id,
            run_date: r.run.run_date,
            status: r.run.status,
            starts_at: r.starts_at,
            done_by: r.done_by,
            steps: r
                .steps
                .iter()
                .map(|s| RoutineRunStepView {
                    step: s.step_id,
                    ord: s.ord,
                    text: s.text.clone(),
                    target_time: s.target_time,
                    done_at: s.done_at,
                })
                .collect(),
            done,
            total: r.steps.len() as i32,
        }
    }

    /// Today's runs (household-local date), optionally of one member.
    pub async fn routines_today(
        &self,
        household: HouseholdId,
        member: Option<MemberId>,
    ) -> Result<Vec<RoutineRunView>> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let runs = RoutineRunRepo::for_date(&mut tx, local_date(now, tz), member).await?;
        Ok(runs.iter().map(Self::run_view).collect())
    }

    /// Today's runs of routines shown on the dashboard.
    pub async fn dashboard_routines(&self, household: HouseholdId) -> Result<Vec<RoutineRunView>> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let runs = RoutineRunRepo::for_date(&mut tx, local_date(now, tz), None).await?;
        Ok(runs
            .iter()
            .filter(|r| r.show_on_dashboard)
            .map(Self::run_view)
            .collect())
    }

    pub async fn tick_routine_step(
        &self,
        household: HouseholdId,
        run: RoutineRunId,
        step: RoutineStepId,
        done: bool,
        member: Option<MemberId>,
        device: Option<DeviceId>,
    ) -> Result<RoutineRunView> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let cur = RoutineRunRepo::get(&mut tx, run)
            .await?
            .ok_or(TaskError::NotFound)?;
        if cur.run.status == RunStatus::Skipped {
            return Err(TaskError::WrongStatus {
                expected: "open or done",
                actual: "skipped".into(),
            });
        }
        RoutineRunRepo::tick(&mut tx, run, step, done, member, device).await?;
        let out = RoutineRunRepo::get(&mut tx, run)
            .await?
            .ok_or(TaskError::NotFound)?;
        publish_routines(&mut tx, household, local_date(now, tz)).await?;
        tx.commit().await?;
        Ok(Self::run_view(&out))
    }

    // ---- absences ----

    pub async fn list_away(
        &self,
        household: HouseholdId,
        member: Option<MemberId>,
    ) -> Result<Vec<MemberAwayView>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(MemberAwayRepo::list(&mut tx, member)
            .await?
            .iter()
            .map(away_view)
            .collect())
    }

    pub async fn add_away(
        &self,
        household: HouseholdId,
        member: MemberId,
        a: NewAway,
    ) -> Result<MemberAwayView> {
        if a.until < a.from {
            return Err(TaskError::invalid("until", "must not be before the start"));
        }
        let mut tx = self.store.tx_for(household).await?;
        let ins = MemberAwayRepo::insert(
            &mut tx,
            &MemberAway {
                id: MemberAwayId::new(),
                household_id: household,
                member_id: member,
                from_date: a.from,
                until_date: a.until,
                note: a.note,
                created_at: self.now(),
            },
        )
        .await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(away_view(&ins))
    }

    pub async fn delete_away(&self, household: HouseholdId, id: MemberAwayId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        MemberAwayRepo::delete(&mut tx, id).await?;
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(())
    }
}

fn away_view(a: &MemberAway) -> MemberAwayView {
    MemberAwayView {
        id: a.id,
        member: a.member_id,
        from: a.from_date,
        until: a.until_date,
        note: a.note.clone(),
    }
}

fn parse_season(s: Option<&Season>) -> Result<Option<(i32, i32)>> {
    let Some(s) = s else { return Ok(None) };
    let f = parse_mmdd(&s.from).map_err(|e| TaskError::invalid("season", format!("from: {e}")))?;
    let u =
        parse_mmdd(&s.until).map_err(|e| TaskError::invalid("season", format!("until: {e}")))?;
    Ok(Some((f, u)))
}

pub(crate) fn validate_definition(d: &TaskDefinition) -> Result<()> {
    if d.title.trim().is_empty() {
        return Err(TaskError::invalid("title", "a title is required"));
    }
    if d.lead_days < 0 {
        return Err(TaskError::invalid("lead_days", "must not be negative"));
    }
    match &d.schedule {
        Schedule::AfterDone { interval_days } if *interval_days < 1 => {
            return Err(TaskError::invalid(
                "schedule",
                "the interval must be at least 1 day",
            ));
        }
        Schedule::Rrule { rrule } => {
            schedule::validate_rrule(rrule).map_err(|m| TaskError::invalid("rrule", m))?;
        }
        _ => {}
    }
    Ok(())
}

/// `next_due_at` after creating a definition or changing its schedule.
pub(crate) fn recompute_next(
    d: &TaskDefinition,
    tz: Tz,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    match (&d.schedule, d.last_done_at) {
        (Schedule::AfterDone { .. }, Some(done)) => schedule::after_done_next(d, tz, done),
        _ => schedule::first_due(d, tz, now),
    }
}
