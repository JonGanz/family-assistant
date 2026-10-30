use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{HouseholdId, Hue, Member, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum, parse_role, role_str};
use crate::{Result, StoreError, Versioned};

pub struct MemberRepo;

/// PIN lockout counters of an adult member.
#[derive(Debug, Clone, Copy)]
pub struct MemberLockout {
    pub failed_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

struct Row {
    id: Uuid,
    household_id: Uuid,
    display_name: String,
    role: String,
    avatar: Option<String>,
    color: Option<String>,
    birthdate: Option<NaiveDate>,
    grade: Option<String>,
    interests: Vec<String>,
    hobbies: Vec<String>,
    responsibilities: Vec<String>,
    aliases: Vec<String>,
    gender: Option<String>,
    relevance_tags: Vec<String>,
    pin_hash: Option<String>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn parse_hue(s: &str) -> Result<Hue> {
    s.parse()
        .map_err(|_| StoreError::Corrupt(format!("hue {s}")))
}

impl TryFrom<Row> for Versioned<Member> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Member {
                id: r.id.into(),
                household_id: r.household_id.into(),
                display_name: r.display_name,
                role: parse_role(&r.role)?,
                avatar: r.avatar,
                color: r.color.as_deref().map(parse_hue).transpose()?,
                birthdate: r.birthdate,
                grade: r.grade,
                interests: r.interests,
                hobbies: r.hobbies,
                responsibilities: r.responsibilities,
                aliases: r.aliases,
                gender: r.gender.as_deref().map(parse_enum).transpose()?,
                relevance_tags: r.relevance_tags,
                pin_hash: r.pin_hash,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl MemberRepo {
    /// `config_key` is the stable key used by the household TOML import.
    pub async fn insert(
        conn: &mut PgConnection,
        m: &Member,
        config_key: Option<&str>,
        created_by: Option<MemberId>,
    ) -> Result<Versioned<Member>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO member (id, household_id, config_key, display_name, role, avatar, color,
                                 birthdate, grade, interests, hobbies, responsibilities, pin_hash, created_by,
                                 aliases, gender, relevance_tags)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
             RETURNING id, household_id, display_name, role, avatar, color, birthdate, grade,
                       interests, hobbies, responsibilities, aliases, gender, relevance_tags, pin_hash, archived_at, version,
                       created_at, updated_at",
            m.id.as_uuid(),
            m.household_id.as_uuid(),
            config_key,
            m.display_name,
            role_str(m.role),
            m.avatar,
            m.color.map(Hue::as_str),
            m.birthdate,
            m.grade,
            &m.interests,
            &m.hobbies,
            &m.responsibilities,
            m.pin_hash,
            created_by.map(|c| c.as_uuid()),
            &m.aliases,
            m.gender.as_ref().map(enum_str),
            &m.relevance_tags
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(conn: &mut PgConnection, id: MemberId) -> Result<Option<Versioned<Member>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, display_name, role, avatar, color, birthdate, grade,
                    interests, hobbies, responsibilities, aliases, gender, relevance_tags, pin_hash, archived_at, version,
                    created_at, updated_at
             FROM member WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn get_by_config_key(
        conn: &mut PgConnection,
        household_id: HouseholdId,
        config_key: &str,
    ) -> Result<Option<Versioned<Member>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, display_name, role, avatar, color, birthdate, grade,
                    interests, hobbies, responsibilities, aliases, gender, relevance_tags, pin_hash, archived_at, version,
                    created_at, updated_at
             FROM member WHERE household_id = $1 AND config_key = $2",
            household_id.as_uuid(),
            config_key
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn config_key(conn: &mut PgConnection, id: MemberId) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar!("SELECT config_key FROM member WHERE id = $1", id.as_uuid())
                .fetch_optional(conn)
                .await?
                .flatten(),
        )
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<Versioned<Member>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, display_name, role, avatar, color, birthdate, grade,
                    interests, hobbies, responsibilities, aliases, gender, relevance_tags, pin_hash, archived_at, version,
                    created_at, updated_at
             FROM member WHERE $1 OR archived_at IS NULL ORDER BY created_at, id",
            include_archived
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Updates profile fields (not the PIN, config key or archive state). Returns the new version.
    pub async fn update(conn: &mut PgConnection, m: &Member, expected_version: i32) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE member SET display_name = $2, role = $3, avatar = $4, color = $5, birthdate = $6,
                    grade = $7, interests = $8, hobbies = $9, responsibilities = $10,
                    aliases = $11, gender = $12, relevance_tags = $13,
                    version = version + 1
             WHERE id = $1 AND version = $14
             RETURNING version",
            m.id.as_uuid(),
            m.display_name,
            role_str(m.role),
            m.avatar,
            m.color.map(Hue::as_str),
            m.birthdate,
            m.grade,
            &m.interests,
            &m.hobbies,
            &m.responsibilities,
            &m.aliases,
            m.gender.as_ref().map(enum_str),
            &m.relevance_tags,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "member", m.id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: MemberId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE member SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
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

    /// Reverses [`Self::archive`]. Idempotent.
    pub async fn unarchive(conn: &mut PgConnection, id: MemberId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE member SET archived_at = NULL, version = version + 1 WHERE id = $1",
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

    pub async fn list_including_archived(
        conn: &mut PgConnection,
    ) -> Result<Vec<Versioned<Member>>> {
        Self::list(conn, true).await
    }

    /// Sets (or clears) the PIN hash and resets the lockout counters.
    pub async fn set_pin_hash(
        conn: &mut PgConnection,
        id: MemberId,
        pin_hash: Option<&str>,
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE member SET pin_hash = $2, pin_failed_attempts = 0, pin_locked_until = NULL,
                    version = version + 1
             WHERE id = $1",
            id.as_uuid(),
            pin_hash
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

    /// Locks the member row (`FOR UPDATE`) until the transaction ends, so parallel attempts
    /// serialise between this check and `record_pin_failure`.
    pub async fn pin_lockout(conn: &mut PgConnection, id: MemberId) -> Result<MemberLockout> {
        let r = sqlx::query!(
            "SELECT pin_failed_attempts, pin_locked_until FROM member WHERE id = $1 FOR UPDATE",
            id.as_uuid()
        )
        .fetch_one(conn)
        .await?;
        Ok(MemberLockout {
            failed_attempts: r.pin_failed_attempts,
            locked_until: r.pin_locked_until,
        })
    }

    /// Records a failed PIN attempt; locks for `lock_secs` once `max_attempts` is reached.
    /// A member that is already locked is left as it is.
    pub async fn record_pin_failure(
        conn: &mut PgConnection,
        id: MemberId,
        max_attempts: i32,
        lock_secs: i64,
    ) -> Result<MemberLockout> {
        let r = sqlx::query!(
            "UPDATE member SET
                 pin_failed_attempts = CASE WHEN pin_failed_attempts + 1 >= $2 THEN 0 ELSE pin_failed_attempts + 1 END,
                 pin_locked_until = CASE WHEN pin_failed_attempts + 1 >= $2
                                         THEN now() + make_interval(secs => $3) ELSE pin_locked_until END
             WHERE id = $1 AND (pin_locked_until IS NULL OR pin_locked_until <= now())
             RETURNING pin_failed_attempts, pin_locked_until",
            id.as_uuid(),
            max_attempts,
            lock_secs as f64
        )
        .fetch_optional(&mut *conn)
        .await?;
        match r {
            Some(r) => Ok(MemberLockout {
                failed_attempts: r.pin_failed_attempts,
                locked_until: r.pin_locked_until,
            }),
            None => Self::pin_lockout(conn, id).await,
        }
    }

    pub async fn clear_pin_failures(conn: &mut PgConnection, id: MemberId) -> Result<()> {
        sqlx::query!(
            "UPDATE member SET pin_failed_attempts = 0, pin_locked_until = NULL WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}

impl MemberRepo {
    /// Non-archived adults of the transaction's household.
    pub async fn adults(conn: &mut PgConnection) -> Result<Vec<MemberId>> {
        let ids = sqlx::query_scalar!(
            "SELECT id FROM member WHERE role = 'adult' AND archived_at IS NULL ORDER BY created_at, id"
        )
        .fetch_all(conn)
        .await?;
        Ok(ids.into_iter().map(Into::into).collect())
    }
}
