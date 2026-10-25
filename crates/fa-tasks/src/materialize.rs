//! `tasks.materialize`, `routines.materialize`, `tasks.from_weather`.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use fa_domain::tasks::{due_instant, visible_from};
use fa_domain::*;
use fa_store::{
    EventRepo, HouseholdRepo, MemberRepo, NewTaskRow, RoutineRepo, RoutineRunRepo, Store,
    TaskDefinitionRepo, TaskListRepo, TaskRepo, Tx, WeatherRuleRepo,
};

use crate::error::{Result, TaskError};
use crate::producers;
use crate::schedule;
use crate::service::{
    TaskService, household_of, local_date, publish_routines, publish_tasks, recompute_next, tz_of,
};
use crate::strategy::assign_for;

/// A condition result older than this counts as unknown (true).
const CONDITION_FRESH: Duration = Duration::hours(6);

pub(crate) async fn household_ids(store: &Store) -> Result<Vec<HouseholdId>> {
    let mut tx = store.tx_system().await?;
    Ok(HouseholdRepo::list(&mut tx)
        .await?
        .into_iter()
        .map(|h| h.value.id)
        .collect())
}

async fn set_next(tx: &mut Tx, d: &TaskDefinition, next: Option<DateTime<Utc>>) -> Result<()> {
    TaskDefinitionRepo::set_schedule_state(tx, d.id, d.rotation_cursor, d.last_done_at, next)
        .await?;
    Ok(())
}

impl TaskService {
    /// All households. Returns how many tasks were created.
    pub async fn materialize_all(&self, now: DateTime<Utc>) -> Result<usize> {
        let due = {
            let mut tx = self.store.tx_system().await?;
            TaskDefinitionRepo::due_for_materialize(&mut tx, now).await?
        };
        let mut created = 0;
        for (h, d) in due {
            match self.materialize_definition(h, d, now).await {
                Ok(n) => created += n,
                Err(e) => tracing::warn!(error = %e, "materialize failed for one definition"),
            }
        }
        Ok(created)
    }

    /// Returns how many tasks it created (0 or 1).
    pub async fn materialize_definition(
        &self,
        household: HouseholdId,
        id: TaskDefinitionId,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let Some(mut def) = TaskDefinitionRepo::lock(&mut tx, id).await? else {
            return Ok(0);
        };
        if def.archived_at.is_some() || def.paused {
            return Ok(0);
        }
        if def.next_due_at.is_none() {
            def.next_due_at = recompute_next(&def, tz, now);
            set_next(&mut tx, &def, def.next_due_at).await?;
            if def.next_due_at.is_none() {
                tx.commit().await?;
                return Ok(0);
            }
        }
        let Some(mut due) = def.next_due_at else {
            return Ok(0);
        };
        if !schedule::due_in_season(&def, tz, due) {
            let Some(moved) = schedule::move_into_season(&def, tz, due) else {
                tx.commit().await?;
                return Ok(0);
            };
            def.next_due_at = Some(moved);
            set_next(&mut tx, &def, def.next_due_at).await?;
            due = moved;
        }
        let date = local_date(due, tz);
        if now < visible_from(date, def.lead_days, tz) {
            tx.commit().await?;
            return Ok(0);
        }
        let fresh = def
            .condition_checked_at
            .is_some_and(|c| now - c <= CONDITION_FRESH);
        if def.condition_ok == Some(false) && fresh {
            tx.commit().await?;
            return Ok(0);
        }

        let is_rrule = matches!(def.schedule, Schedule::Rrule { .. });
        let open = TaskRepo::open_for_definition(&mut tx, id).await?;
        let mut create = true;
        if open.iter().any(|t| t.due_at == Some(due)) {
            create = false;
        } else if !open.is_empty() {
            match def.on_missed {
                OnMissed::RollOver => create = false,
                OnMissed::AutoSkip => {
                    for t in &open {
                        TaskRepo::set_status(&mut tx, t.id, TaskStatus::Skipped, None, None, None)
                            .await?;
                    }
                }
                OnMissed::PileUp => {}
            }
        }
        let mut made = 0;
        if create {
            let list = TaskListRepo::get(&mut tx, def.list_id)
                .await?
                .ok_or(TaskError::NotFound)?;
            let cands = TaskDefinitionRepo::candidates(&mut tx, id).await?;
            let assignees = assign_for(&mut tx, &def, &cands, date).await?;
            let new_id = TaskId::new();
            let inserted = TaskRepo::insert(
                &mut tx,
                &NewTaskRow {
                    id: new_id,
                    household_id: household,
                    list_id: def.list_id,
                    definition_id: Some(id),
                    weather_rule_id: None,
                    title: def.title.clone(),
                    notes: def.notes.clone(),
                    due_at: Some(due),
                    visible_from: Some(visible_from(date, def.lead_days, tz)),
                    status: TaskStatus::Open,
                    origin: TaskOrigin::System,
                    source_ref: None,
                    priority_flag: def.priority_flag,
                    calendar_display: list.value.calendar_default,
                    created_by: None,
                },
                &assignees,
                &def.steps_template,
            )
            .await?;
            if let Some(t) = inserted {
                made = 1;
                TaskRepo::set_created_at(&mut tx, new_id, now).await?;
                if def.strategy != Strategy::Pool {
                    producers::assigned(
                        &mut tx,
                        &t.value,
                        &assignees,
                        None,
                        format!("{} · due {date}", list.value.name),
                    )
                    .await?;
                }
            }
        }
        if is_rrule {
            let next = schedule::advance_rrule(&def, tz, due);
            set_next(&mut tx, &def, next).await?;
        }
        if made > 0 || !open.is_empty() {
            publish_tasks(&mut tx, household).await?;
        }
        tx.commit().await?;
        Ok(made)
    }

    /// `routines.materialize` for all households.
    pub async fn routines_materialize_all(&self, now: DateTime<Utc>) -> Result<usize> {
        let mut total = 0;
        for h in household_ids(&self.store).await? {
            match self.routines_materialize(h, now).await {
                Ok(n) => total += n,
                Err(e) => tracing::warn!(error = %e, "routines.materialize failed for a household"),
            }
        }
        Ok(total)
    }

    /// Creates today's runs and skips older open ones. Returns the number of new runs.
    pub async fn routines_materialize(
        &self,
        household: HouseholdId,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        let today = local_date(now, tz);
        let members: Vec<MemberId> = MemberRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .map(|m| m.value.id)
            .collect();
        let mut created = 0;
        for f in RoutineRepo::list(&mut tx, false).await? {
            let r = &f.routine.value;
            let titles = match (r.day_filter, r.school_calendar_id) {
                (DayFilter::SchoolDays, Some(cal)) => {
                    Some(EventRepo::all_day_titles(&mut tx, cal, today).await?)
                }
                _ => None,
            };
            if !fa_domain::routines::day_qualifies(
                r.day_filter,
                today,
                titles.as_deref(),
                &r.no_school_match,
            ) {
                continue;
            }
            let steps: Vec<RoutineStepId> = f.steps.iter().map(|s| s.id).collect();
            for m in f.members.iter().filter(|m| members.contains(m)) {
                if RoutineRunRepo::ensure(&mut tx, r.id, *m, today, &steps).await? {
                    created += 1;
                }
            }
        }
        let skipped = RoutineRunRepo::skip_before(&mut tx, today).await?;
        if created > 0 || skipped > 0 {
            publish_routines(&mut tx, household, today).await?;
        }
        tx.commit().await?;
        Ok(created)
    }

    /// `tasks.from_weather`: one task per rule and day. `Ok(false)` when nothing was created.
    pub async fn from_weather(
        &self,
        household: HouseholdId,
        rule: WeatherRuleId,
        date: NaiveDate,
    ) -> Result<bool> {
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let tz: Tz = tz_of(&household_of(&mut tx, household).await?);
        let Some(rule) = WeatherRuleRepo::get(&mut tx, rule).await? else {
            tracing::debug!("from_weather: rule not found");
            return Ok(false);
        };
        let rule = rule.value;
        let def_id = rule
            .task_definition
            .as_ref()
            .and_then(|v| v.get("definition"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<TaskDefinitionId>().ok());
        if rule.archived_at.is_some() || rule.action != RuleAction::Task {
            return Ok(false);
        }
        let Some(def) = match def_id {
            Some(d) => TaskDefinitionRepo::get(&mut tx, d).await?,
            None => None,
        }
        .map(|d| d.value)
        .filter(|d| d.archived_at.is_none()) else {
            tracing::debug!("from_weather: the rule has no usable task definition");
            return Ok(false);
        };
        let due = due_instant(date, def.due_time, tz);
        if TaskRepo::weather_task_exists(&mut tx, rule.id, due).await? {
            return Ok(false);
        }
        let list = TaskListRepo::get(&mut tx, def.list_id)
            .await?
            .ok_or(TaskError::NotFound)?;
        let cands = TaskDefinitionRepo::candidates(&mut tx, def.id).await?;
        let assignees = assign_for(&mut tx, &def, &cands, date).await?;
        let suggest = rule.task_mode == TaskMode::Suggest;
        let id = TaskId::new();
        let inserted = TaskRepo::insert(
            &mut tx,
            &NewTaskRow {
                id,
                household_id: household,
                list_id: def.list_id,
                definition_id: None,
                weather_rule_id: Some(rule.id),
                title: def.title.clone(),
                notes: def.notes.clone(),
                due_at: Some(due),
                visible_from: None,
                status: if suggest {
                    TaskStatus::Suggested
                } else {
                    TaskStatus::Open
                },
                origin: TaskOrigin::Weather,
                source_ref: Some(serde_json::json!({
                    "rule": rule.id,
                    "date": date,
                    "strategy": def.strategy.as_str(),
                })),
                priority_flag: def.priority_flag,
                calendar_display: list.value.calendar_default,
                created_by: None,
            },
            &assignees,
            &def.steps_template,
        )
        .await?;
        let Some(t) = inserted else {
            return Ok(false);
        };
        TaskRepo::set_created_at(&mut tx, id, now).await?;
        if !suggest && def.strategy != Strategy::Pool {
            producers::assigned(&mut tx, &t.value, &assignees, None, list.value.name.clone())
                .await?;
        }
        publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(true)
    }
}
