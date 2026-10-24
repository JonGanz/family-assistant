use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::conflict_or_missing;
use crate::{Result, StoreError, Versioned};

#[derive(Debug, Clone)]
pub struct NotifyPref {
    pub id: Uuid,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    /// Currently always `ntfy`.
    pub channel: String,
    pub topic: String,
    pub triggers: Vec<String>,
    pub archived_at: Option<DateTime<Utc>>,
}

pub struct NotifyPrefRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    member_id: Uuid,
    channel: String,
    topic: String,
    triggers: Vec<String>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
}

impl From<Row> for Versioned<NotifyPref> {
    fn from(r: Row) -> Self {
        Versioned {
            version: r.version,
            value: NotifyPref {
                id: r.id,
                household_id: r.household_id.into(),
                member_id: r.member_id.into(),
                channel: r.channel,
                topic: r.topic,
                triggers: r.triggers,
                archived_at: r.archived_at,
            },
        }
    }
}

impl NotifyPrefRepo {
    pub async fn insert(conn: &mut PgConnection, p: &NotifyPref) -> Result<Versioned<NotifyPref>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO notify_pref (id, household_id, member_id, channel, topic, triggers)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, household_id, member_id, channel, topic, triggers, archived_at, version",
            p.id,
            p.household_id.as_uuid(),
            p.member_id.as_uuid(),
            p.channel,
            p.topic,
            &p.triggers
        )
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }

    pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<Option<Versioned<NotifyPref>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, channel, topic, triggers, archived_at, version
             FROM notify_pref WHERE id = $1",
            id
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    pub async fn list_for_member(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Vec<Versioned<NotifyPref>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, channel, topic, triggers, archived_at, version
             FROM notify_pref WHERE member_id = $1 AND archived_at IS NULL ORDER BY created_at, id",
            member.as_uuid()
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update(
        conn: &mut PgConnection,
        p: &NotifyPref,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE notify_pref SET channel = $2, topic = $3, triggers = $4, version = version + 1
             WHERE id = $1 AND version = $5 RETURNING version",
            p.id,
            p.channel,
            p.topic,
            &p.triggers,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "notify_pref", p.id).await),
        }
    }

    pub async fn archive(conn: &mut PgConnection, id: Uuid) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE notify_pref SET archived_at = COALESCE(archived_at, now()), version = version + 1 WHERE id = $1",
            id
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
