use chrono::{DateTime, NaiveTime, Utc};
use fa_domain::{ApprovalMode, HouseholdId, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

#[derive(Debug, Clone)]
pub struct NotifyPref {
    pub id: Uuid,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    /// Currently always `ntfy`.
    pub channel: String,
    pub topic: String,
    /// The enabled triggers (`NotifyTrigger::as_str` values).
    pub triggers: Vec<String>,
    pub due_soon_minutes: i32,
    pub nudge_minutes: i32,
    pub quiet_from: Option<NaiveTime>,
    pub quiet_until: Option<NaiveTime>,
    pub copy_to_member_id: Option<MemberId>,
    /// How `approval_needed` notifications are delivered.
    pub approval_mode: ApprovalMode,
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
    due_soon_minutes: i32,
    nudge_minutes: i32,
    quiet_from: Option<NaiveTime>,
    quiet_until: Option<NaiveTime>,
    copy_to_member_id: Option<Uuid>,
    approval_mode: String,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
}

impl From<Row> for Versioned<NotifyPref> {
    fn from(r: Row) -> Self {
        // The column is CHECK-constrained to the enum's values.
        let approval_mode = parse_enum(&r.approval_mode).unwrap_or_default();
        Versioned {
            version: r.version,
            value: NotifyPref {
                id: r.id,
                household_id: r.household_id.into(),
                member_id: r.member_id.into(),
                channel: r.channel,
                topic: r.topic,
                triggers: r.triggers,
                due_soon_minutes: r.due_soon_minutes,
                nudge_minutes: r.nudge_minutes,
                quiet_from: r.quiet_from,
                quiet_until: r.quiet_until,
                copy_to_member_id: r.copy_to_member_id.map(Into::into),
                approval_mode,
                archived_at: r.archived_at,
            },
        }
    }
}

impl NotifyPrefRepo {
    pub async fn insert(conn: &mut PgConnection, p: &NotifyPref) -> Result<Versioned<NotifyPref>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO notify_pref (id, household_id, member_id, channel, topic, triggers,
                                      due_soon_minutes, nudge_minutes, quiet_from, quiet_until,
                                      copy_to_member_id, approval_mode)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version",
            p.id,
            p.household_id.as_uuid(),
            p.member_id.as_uuid(),
            p.channel,
            p.topic,
            &p.triggers,
            p.due_soon_minutes,
            p.nudge_minutes,
            p.quiet_from,
            p.quiet_until,
            p.copy_to_member_id.map(|m| m.as_uuid()),
            enum_str(&p.approval_mode)
        )
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }

    pub async fn get(conn: &mut PgConnection, id: Uuid) -> Result<Option<Versioned<NotifyPref>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version
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
            "SELECT id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version
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
            "UPDATE notify_pref SET channel = $2, topic = $3, triggers = $4, due_soon_minutes = $5,
                    nudge_minutes = $6, quiet_from = $7, quiet_until = $8, copy_to_member_id = $9,
                    approval_mode = $10, version = version + 1
             WHERE id = $1 AND version = $11 RETURNING version",
            p.id,
            p.channel,
            p.topic,
            &p.triggers,
            p.due_soon_minutes,
            p.nudge_minutes,
            p.quiet_from,
            p.quiet_until,
            p.copy_to_member_id.map(|m| m.as_uuid()),
            enum_str(&p.approval_mode),
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

    /// The member's active (non-archived) preferences.
    pub async fn get_for_member(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Option<Versioned<NotifyPref>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version
             FROM notify_pref WHERE member_id = $1 AND archived_at IS NULL",
            member.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Creates the member's preferences or replaces them (the `id` of an existing row is kept).
    /// With `expected_version`, a stale existing row gives `Conflict`.
    pub async fn upsert_for_member(
        conn: &mut PgConnection,
        p: &NotifyPref,
        expected_version: Option<i32>,
    ) -> Result<Versioned<NotifyPref>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO notify_pref (id, household_id, member_id, channel, topic, triggers,
                                      due_soon_minutes, nudge_minutes, quiet_from, quiet_until,
                                      copy_to_member_id, approval_mode)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $13)
             ON CONFLICT (member_id) WHERE archived_at IS NULL DO UPDATE SET
                 channel = EXCLUDED.channel, topic = EXCLUDED.topic, triggers = EXCLUDED.triggers,
                 due_soon_minutes = EXCLUDED.due_soon_minutes, nudge_minutes = EXCLUDED.nudge_minutes,
                 quiet_from = EXCLUDED.quiet_from, quiet_until = EXCLUDED.quiet_until,
                 copy_to_member_id = EXCLUDED.copy_to_member_id, approval_mode = EXCLUDED.approval_mode,
                 version = notify_pref.version + 1
             WHERE $12::int IS NULL OR notify_pref.version = $12
             RETURNING id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version",
            p.id,
            p.household_id.as_uuid(),
            p.member_id.as_uuid(),
            p.channel,
            p.topic,
            &p.triggers,
            p.due_soon_minutes,
            p.nudge_minutes,
            p.quiet_from,
            p.quiet_until,
            p.copy_to_member_id.map(|m| m.as_uuid()),
            expected_version,
            enum_str(&p.approval_mode)
        )
        .fetch_optional(conn)
        .await?;
        row.map(Into::into).ok_or(StoreError::Conflict)
    }

    /// Every member's active preferences (all households in a system tx).
    pub async fn list_all(conn: &mut PgConnection) -> Result<Vec<Versioned<NotifyPref>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, member_id, channel, topic, triggers, due_soon_minutes, nudge_minutes, quiet_from, quiet_until, copy_to_member_id, approval_mode, archived_at, version
             FROM notify_pref WHERE archived_at IS NULL ORDER BY created_at, id"
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
