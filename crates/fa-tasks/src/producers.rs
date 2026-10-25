//! Notification outbox rows (plan/M2.md, Notifications: producers).

use fa_domain::{MemberId, NotifyTrigger, Task, TaskId};
use fa_store::{NewNotification, NotificationRepo};
use sqlx::PgConnection;

use crate::error::Result;

pub(crate) fn task_path(id: TaskId) -> String {
    format!("/tasks/{}", id.as_uuid())
}

/// `assigned:{task}:{member}` rows for the new assignees (never the one who made the task).
pub(crate) async fn assigned(
    conn: &mut PgConnection,
    task: &Task,
    members: &[MemberId],
    except: Option<MemberId>,
    body: String,
) -> Result<()> {
    for m in members.iter().filter(|m| Some(**m) != except) {
        NotificationRepo::insert_dedupe(
            conn,
            &NewNotification {
                household_id: task.household_id,
                member_id: *m,
                trigger: NotifyTrigger::Assigned,
                subject_type: "task".into(),
                subject_id: Some(task.id.as_uuid()),
                title: format!("New task: {}", task.title),
                body: body.clone(),
                click_path: Some(task_path(task.id)),
                priority: 3,
                dedupe_key: format!("assigned:{}:{}", task.id.as_uuid(), m.as_uuid()),
                not_before: None,
            },
        )
        .await?;
    }
    Ok(())
}
