//! `tasks.tick` (claims, overdue, due_soon, routine nudges) and `tasks.digest`.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveTime, Utc};
use fa_domain::notify::{effective_quiet, in_quiet};
use fa_domain::tasks::due_instant;
use fa_domain::*;
use fa_store::{
    InboxRepo, MemberRepo, NewNotification, NotificationRepo, NotifyPref, NotifyPrefRepo,
    RoutineRunRepo, TaskFilter, TaskRepo, Versioned,
};

use crate::error::Result;
use crate::materialize::household_ids;
use crate::producers::task_path;
use crate::service::{TaskService, household_of, local_date, publish_tasks, tz_of};

fn enabled(p: &Versioned<NotifyPref>, t: NotifyTrigger) -> bool {
    p.value.triggers.iter().any(|x| x == t.as_str())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

impl TaskService {
    pub async fn tick_all(&self, now: DateTime<Utc>) -> Result<()> {
        for h in household_ids(&self.store).await? {
            if let Err(e) = self.tick(h, now).await {
                tracing::warn!(error = %e, "tasks.tick failed for a household");
            }
        }
        Ok(())
    }

    pub async fn tick(&self, household: HouseholdId, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let tz = tz_of(&household_of(&mut tx, household).await?);
        if TaskRepo::clear_expired_claims(&mut tx, now).await? > 0 {
            publish_tasks(&mut tx, household).await?;
        }
        let prefs: HashMap<MemberId, Versioned<NotifyPref>> = NotifyPrefRepo::list_all(&mut tx)
            .await?
            .into_iter()
            .filter(|p| p.value.archived_at.is_none())
            .map(|p| (p.value.member_id, p))
            .collect();

        for (task, title, members) in TaskRepo::overdue_titled(&mut tx, now).await? {
            for m in members {
                NotificationRepo::insert_dedupe(
                    &mut tx,
                    &NewNotification {
                        household_id: household,
                        member_id: m,
                        trigger: NotifyTrigger::Overdue,
                        subject_type: "task".into(),
                        subject_id: Some(task.as_uuid()),
                        title: format!("Overdue: {title}"),
                        body: String::new(),
                        click_path: Some(task_path(task)),
                        priority: 4,
                        dedupe_key: format!("overdue:{}:{}", task.as_uuid(), m.as_uuid()),
                        not_before: None,
                    },
                )
                .await?;
            }
        }

        let horizon = prefs
            .values()
            .filter(|p| enabled(p, NotifyTrigger::DueSoon))
            .map(|p| p.value.due_soon_minutes)
            .max();
        if let Some(minutes) = horizon {
            for (task, due, members, title) in
                TaskRepo::due_soon_candidates(&mut tx, now, Duration::minutes(i64::from(minutes)))
                    .await?
            {
                for m in members {
                    let Some(p) = prefs.get(&m).filter(|p| enabled(p, NotifyTrigger::DueSoon))
                    else {
                        continue;
                    };
                    if due - Duration::minutes(i64::from(p.value.due_soon_minutes)) > now {
                        continue;
                    }
                    NotificationRepo::insert_dedupe(
                        &mut tx,
                        &NewNotification {
                            household_id: household,
                            member_id: m,
                            trigger: NotifyTrigger::DueSoon,
                            subject_type: "task".into(),
                            subject_id: Some(task.as_uuid()),
                            title: format!("Due soon: {title}"),
                            body: String::new(),
                            click_path: Some(task_path(task)),
                            priority: 3,
                            dedupe_key: format!("due_soon:{}:{}", task.as_uuid(), m.as_uuid()),
                            not_before: None,
                        },
                    )
                    .await?;
                }
            }
        }

        let local = now.with_timezone(&tz);
        let (today, time) = (local.date_naive(), local.time());
        for run in RoutineRunRepo::open_with_steps_left(&mut tx, today).await? {
            let Some(p) = prefs
                .get(&run.run.member_id)
                .filter(|p| enabled(p, NotifyTrigger::RoutineNudge))
            else {
                continue;
            };
            let from = run.done_by - Duration::minutes(i64::from(p.value.nudge_minutes));
            if !(from <= run.done_by && time >= from && time < run.done_by) {
                continue;
            }
            let left = run.steps.iter().filter(|s| s.done_at.is_none()).count();
            NotificationRepo::insert_dedupe(
                &mut tx,
                &NewNotification {
                    household_id: household,
                    member_id: run.run.member_id,
                    trigger: NotifyTrigger::RoutineNudge,
                    subject_type: "routine_run".into(),
                    subject_id: Some(run.run.id.as_uuid()),
                    title: format!("Routine: {}", plural(left, "step left", "steps left")),
                    body: run.routine_name.clone(),
                    click_path: Some("/routines".into()),
                    priority: 4,
                    dedupe_key: format!("nudge:{}", run.run.id.as_uuid()),
                    not_before: None,
                },
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn digest_all(&self, now: DateTime<Utc>) -> Result<()> {
        for h in household_ids(&self.store).await? {
            if let Err(e) = self.digest(h, now).await {
                tracing::warn!(error = %e, "tasks.digest failed for a household");
            }
        }
        Ok(())
    }

    /// One digest per member and local day, once their quiet hours are over. Skipped when the
    /// member has nothing to do.
    pub async fn digest(&self, household: HouseholdId, now: DateTime<Utc>) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let tz = tz_of(&h);
        let local = now.with_timezone(&tz);
        let today = local.date_naive();
        let end_of_day = due_instant(
            today,
            Some(NaiveTime::from_hms_opt(23, 59, 59).unwrap_or(NaiveTime::MIN)),
            tz,
        );
        let inbox_total = InboxRepo::counts(&mut tx).await?.total;
        for p in NotifyPrefRepo::list_all(&mut tx).await? {
            if p.value.archived_at.is_some() || !enabled(&p, NotifyTrigger::Digest) {
                continue;
            }
            let m = p.value.member_id;
            let (from, until) = effective_quiet(
                p.value.quiet_from,
                p.value.quiet_until,
                h.quiet_from,
                h.quiet_until,
            );
            if local.time() < until || in_quiet(local.time(), from, until) {
                continue;
            }
            let Some(member) = MemberRepo::get(&mut tx, m)
                .await?
                .filter(|x| x.value.archived_at.is_none())
            else {
                continue;
            };
            let tasks = TaskRepo::get_view_rows(
                &mut tx,
                &TaskFilter {
                    statuses: Some(vec![TaskStatus::Open]),
                    member: Some(m),
                    due_to: Some(end_of_day + Duration::seconds(1)),
                    visible_before: Some(now),
                    ..Default::default()
                },
            )
            .await?;
            let runs: Vec<_> = RoutineRunRepo::for_date(&mut tx, today, Some(m))
                .await?
                .into_iter()
                .filter(|r| r.run.status == RunStatus::Open)
                .collect();
            let waiting =
                if p.value.approval_mode == ApprovalMode::Off || member.value.role != Role::Adult {
                    0
                } else {
                    inbox_total
                };
            if tasks.is_empty() && runs.is_empty() && waiting == 0 {
                continue;
            }
            let title = if tasks.is_empty() && runs.is_empty() {
                "Good morning".to_owned()
            } else if tasks.is_empty() {
                format!(
                    "Good morning: {} today",
                    plural(runs.len(), "routine", "routines")
                )
            } else {
                format!(
                    "Good morning: {} today",
                    plural(tasks.len(), "task", "tasks")
                )
            };
            let mut lines: Vec<String> = tasks.iter().map(|t| t.task.value.title.clone()).collect();
            lines.extend(runs.iter().map(|r| format!("Routine: {}", r.routine_name)));
            if waiting > 0 {
                lines.push(format!("{waiting} items waiting for approval"));
            }
            NotificationRepo::insert_dedupe(
                &mut tx,
                &NewNotification {
                    household_id: household,
                    member_id: m,
                    trigger: NotifyTrigger::Digest,
                    subject_type: "member".into(),
                    subject_id: Some(m.as_uuid()),
                    title,
                    body: lines.join("\n"),
                    click_path: Some(
                        if tasks.is_empty() && runs.is_empty() && waiting > 0 {
                            "/inbox"
                        } else if tasks.is_empty() {
                            "/routines"
                        } else {
                            "/tasks"
                        }
                        .into(),
                    ),
                    priority: 2,
                    dedupe_key: format!("digest:{}:{}", m.as_uuid(), today),
                    not_before: None,
                },
            )
            .await?;
        }
        let _ = local_date(now, tz);
        tx.commit().await?;
        Ok(())
    }
}
