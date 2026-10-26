//! Task and task list additions for email-derived tasks (M3).

use chrono::{DateTime, Utc};
use fa_domain::{MessageId, TaskId, TaskList, TaskListId, TaskListKind, TaskStatus};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{TaskListRepo, TaskRepo, enum_str, parse_enum};
use crate::{Result, Versioned};

impl TaskRepo {
    /// Tasks created from a message (`source_ref.message_id`), not archived.
    pub async fn derived_for_message(
        conn: &mut PgConnection,
        message: MessageId,
    ) -> Result<Vec<(TaskId, String, TaskStatus)>> {
        let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, title, status FROM task
             WHERE source_ref->>'message_id' = $1 AND archived_at IS NULL
             ORDER BY created_at, id",
        )
        .bind(message.to_string())
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|(id, title, status)| Ok((id.into(), title, parse_enum(&status)?)))
            .collect()
    }

    /// `suggested` email tasks created before `cutoff` (to be skipped when the inbox expires).
    pub async fn suggested_email_older_than(
        conn: &mut PgConnection,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<TaskId>> {
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM task
             WHERE status = 'suggested' AND origin = 'email' AND archived_at IS NULL AND created_at < $1
             ORDER BY created_at, id",
        )
        .bind(cutoff)
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl TaskListRepo {
    /// The non-archived list of this kind and name, created from `defaults` when missing
    /// (`defaults` supplies the household and every setting except kind, name and id).
    pub async fn get_or_create(
        conn: &mut PgConnection,
        kind: TaskListKind,
        name: &str,
        defaults: &TaskList,
    ) -> Result<Versioned<TaskList>> {
        // Serialise concurrent first uses: there is no unique index on (household, name).
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(format!(
                "task_list:{}:{}:{name}",
                defaults.household_id,
                enum_str(&kind)
            ))
            .execute(&mut *conn)
            .await?;
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM task_list WHERE kind = $1 AND name = $2 AND archived_at IS NULL
             ORDER BY created_at, id LIMIT 1",
        )
        .bind(enum_str(&kind))
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(id) = existing {
            let id = TaskListId::from(id);
            return TaskListRepo::get(conn, id)
                .await?
                .ok_or(crate::StoreError::NotFound);
        }
        let list = TaskList {
            id: TaskListId::new(),
            kind,
            name: name.to_string(),
            archived_at: None,
            ..defaults.clone()
        };
        TaskListRepo::insert(conn, &list).await
    }
}
