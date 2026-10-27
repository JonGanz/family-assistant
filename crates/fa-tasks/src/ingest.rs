//! `tasks.from_mail` and `tasks.from_bill` (plan/M3.md, fa-tasks additions).

use chrono::{DateTime, Duration, NaiveTime, Utc};
use fa_domain::tasks::due_instant;
use fa_domain::*;
use fa_store::{
    BillRepo, MemberRepo, MessageRepo, NewTaskRow, NotificationRepo, TaskListRepo, TaskRepo,
};
use serde::Deserialize;

use crate::error::{Result, TaskError};
use crate::producers;
use crate::service::{TaskService, household_of, tz_of};

/// Payload of `tasks.from_mail`. `household` is optional (looked up from the message).
#[derive(Debug, Clone, Deserialize)]
pub struct MailTaskRequest {
    #[serde(default)]
    pub household: Option<HouseholdId>,
    pub message: MessageId,
    #[serde(default)]
    pub kind: Option<String>,
    pub title: String,
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub assignees: Vec<MemberId>,
    pub mode: MailTaskMode,
    #[serde(default)]
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailTaskMode {
    Suggest,
    Create,
}

fn list_defaults(household: HouseholdId, calendar: CalendarDisplay) -> TaskList {
    TaskList {
        id: TaskListId::new(),
        household_id: household,
        name: String::new(),
        kind: TaskListKind::General,
        completion_policy: ListCompletionPolicy::Assignee,
        child_can_add: ChildCanAdd::None,
        dashboard_add: false,
        calendar_default: calendar,
        claim_ttl_minutes: None,
        visible_on_dashboard: true,
        color: None,
        sort: 0,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

impl TaskService {
    /// Creates the task of a `tasks.from_mail` job. Returns `None` when it already exists.
    pub async fn from_mail(&self, req: MailTaskRequest) -> Result<Option<TaskId>> {
        let title = req.title.trim().to_owned();
        if title.is_empty() {
            return Err(TaskError::invalid("title", "a title is required"));
        }
        let household = match req.household {
            Some(h) => h,
            None => {
                let mut sys = self.store.tx_system().await?;
                MessageRepo::household_of(&mut sys, req.message)
                    .await?
                    .ok_or(TaskError::NotFound)?
            }
        };
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let existing = TaskRepo::derived_for_message(&mut tx, req.message).await?;
        if existing.iter().any(|(_, t, _)| *t == title) {
            return Ok(None);
        }
        let list = TaskListRepo::get_or_create(
            &mut tx,
            TaskListKind::Email,
            "Email",
            &list_defaults(household, CalendarDisplay::UntilDone),
        )
        .await?
        .value;
        let suggest = req.mode == MailTaskMode::Suggest;
        let assignees: Vec<MemberId> = {
            let active: Vec<MemberId> = MemberRepo::list(&mut tx, false)
                .await?
                .into_iter()
                .map(|m| m.value.id)
                .collect();
            let mut seen = Vec::new();
            for a in req.assignees.iter().filter(|a| active.contains(a)) {
                if !seen.contains(a) {
                    seen.push(*a);
                }
            }
            seen
        };
        let id = TaskId::new();
        let mut source = serde_json::json!({
            "kind": "email",
            "message_id": req.message.to_string(),
        });
        if let Some(url) = &req.external_url {
            source["external_url"] = url.clone().into();
        }
        if let Some(k) = &req.kind {
            source["mail_task_kind"] = k.clone().into();
        }
        let Some(created) = TaskRepo::insert(
            &mut tx,
            &NewTaskRow {
                id,
                household_id: household,
                list_id: list.id,
                definition_id: None,
                weather_rule_id: None,
                title: title.clone(),
                notes: None,
                due_at: req.due_at,
                visible_from: None,
                status: if suggest {
                    TaskStatus::Suggested
                } else {
                    TaskStatus::Open
                },
                origin: TaskOrigin::Email,
                source_ref: Some(source),
                priority_flag: PriorityFlag::Normal,
                calendar_display: list.calendar_default,
                created_by: None,
            },
            &assignees,
            &[],
        )
        .await?
        else {
            return Ok(None);
        };
        TaskRepo::set_created_at(&mut tx, id, now).await?;
        if suggest {
            let source = MessageRepo::get(&mut tx, req.message)
                .await?
                .map(|m| m.source_id);
            NotificationRepo::insert_for_source_adults(
                &mut tx,
                source,
                NotifyTrigger::ApprovalNeeded,
                "task",
                Some(id.as_uuid()),
                "Task suggested from email",
                &title,
                Some("/inbox"),
                &format!("approval:task:{}", id.as_uuid()),
            )
            .await?;
            publish_kind(&mut tx, household, "inbox.updated").await?;
        } else {
            producers::assigned(&mut tx, &created.value, &assignees, None, list.name.clone())
                .await?;
        }
        crate::service::publish_tasks(&mut tx, household).await?;
        tx.commit().await?;
        Ok(Some(id))
    }

    /// Creates the "Pay <payee>" task of an open bill. Returns `None` for the no-op cases.
    pub async fn from_bill(
        &self,
        household: Option<HouseholdId>,
        bill: BillId,
    ) -> Result<Option<TaskId>> {
        let household = match household {
            Some(h) => h,
            None => {
                let mut sys = self.store.tx_system().await?;
                BillRepo::household_of(&mut sys, bill)
                    .await?
                    .ok_or(TaskError::NotFound)?
            }
        };
        let now = self.now();
        let mut tx = self.store.tx_for(household).await?;
        let Some(b) = BillRepo::get(&mut tx, bill).await?.map(|b| b.value) else {
            return Ok(None);
        };
        if b.task_id.is_some() || b.status != BillStatus::Open {
            return Ok(None);
        }
        let h = household_of(&mut tx, household).await?;
        let tz = tz_of(&h);
        let due_at = b.due_on.map(|d| {
            let lead = Duration::days(i64::from(h.bill_lead_days.max(0)));
            due_instant(
                d - lead,
                Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap_or(NaiveTime::MIN)),
                tz,
            )
        });
        let list = TaskListRepo::get_or_create(
            &mut tx,
            TaskListKind::General,
            "Bills",
            &list_defaults(household, CalendarDisplay::Hidden),
        )
        .await?
        .value;
        let adults: Vec<MemberId> = MemberRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .filter(|m| m.value.role == Role::Adult)
            .map(|m| m.value.id)
            .collect();
        let id = TaskId::new();
        let mut source = serde_json::json!({
            "kind": "bill",
            "bill_id": b.id.to_string(),
            "strategy": "pool",
        });
        if let Some(m) = b.message_id {
            source["message_id"] = m.to_string().into();
        }
        let Some(_) = TaskRepo::insert(
            &mut tx,
            &NewTaskRow {
                id,
                household_id: household,
                list_id: list.id,
                definition_id: None,
                weather_rule_id: None,
                title: format!("Pay {}", b.payee),
                notes: None,
                due_at,
                visible_from: None,
                status: TaskStatus::Open,
                origin: TaskOrigin::Email,
                source_ref: Some(source),
                priority_flag: PriorityFlag::Normal,
                calendar_display: list.calendar_default,
                created_by: None,
            },
            &adults,
            &[],
        )
        .await?
        else {
            return Ok(None);
        };
        TaskRepo::set_created_at(&mut tx, id, now).await?;
        BillRepo::set_task(&mut tx, bill, id).await?;
        crate::service::publish_tasks(&mut tx, household).await?;
        crate::service::publish_bills(&mut tx, household).await?;
        tx.commit().await?;
        Ok(Some(id))
    }
}

async fn publish_kind(tx: &mut fa_store::Tx, household: HouseholdId, kind: &str) -> Result<()> {
    fa_store::ChangeBus::publish(tx, household, kind, &serde_json::json!({})).await?;
    Ok(())
}
