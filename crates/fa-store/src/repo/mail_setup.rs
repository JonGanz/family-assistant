//! Household mail setup: user rules and categories.

use chrono::{DateTime, Utc};
use fa_domain::{
    AutoAction, Category, CategoryId, DecisionPointKey, MailRule, MailRuleId, RuleCondition,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

fn one(n: u64) -> Result<()> {
    if n == 0 {
        Err(StoreError::NotFound)
    } else {
        Ok(())
    }
}

// ---- rules ----

pub struct MailRuleRepo;

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    household_id: Uuid,
    decision_point: String,
    name: String,
    priority: i32,
    conditions: serde_json::Value,
    value: serde_json::Value,
    confidence: f32,
    enabled: bool,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const RULE_COLS: &str = "id, household_id, decision_point, name, priority, conditions, value,
    confidence, enabled, archived_at, version, created_at, updated_at";

impl TryFrom<RuleRow> for Versioned<MailRule> {
    type Error = StoreError;
    fn try_from(r: RuleRow) -> Result<Self> {
        let conditions: Vec<RuleCondition> = serde_json::from_value(r.conditions)
            .map_err(|e| StoreError::Corrupt(format!("mail_rule conditions: {e}")))?;
        Ok(Versioned {
            version: r.version,
            value: MailRule {
                id: r.id.into(),
                household_id: r.household_id.into(),
                decision_point: parse_enum(&r.decision_point)?,
                name: r.name,
                priority: r.priority,
                conditions,
                value: r.value,
                confidence: r.confidence,
                enabled: r.enabled,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

fn conditions_json(c: &[RuleCondition]) -> Result<serde_json::Value> {
    serde_json::to_value(c).map_err(|e| StoreError::Corrupt(format!("mail_rule conditions: {e}")))
}

impl MailRuleRepo {
    pub async fn insert(conn: &mut PgConnection, r: &MailRule) -> Result<Versioned<MailRule>> {
        let row: RuleRow = sqlx::query_as(&format!(
            "INSERT INTO mail_rule (id, household_id, decision_point, name, priority, conditions,
                                    value, confidence, enabled)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING {RULE_COLS}"
        ))
        .bind(r.id.as_uuid())
        .bind(r.household_id.as_uuid())
        .bind(enum_str(&r.decision_point))
        .bind(&r.name)
        .bind(r.priority)
        .bind(conditions_json(&r.conditions)?)
        .bind(&r.value)
        .bind(r.confidence)
        .bind(r.enabled)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: MailRuleId,
    ) -> Result<Option<Versioned<MailRule>>> {
        let row: Option<RuleRow> =
            sqlx::query_as(&format!("SELECT {RULE_COLS} FROM mail_rule WHERE id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// By priority (lowest first), then creation order.
    pub async fn list(
        conn: &mut PgConnection,
        dp: Option<DecisionPointKey>,
        include_archived: bool,
    ) -> Result<Vec<Versioned<MailRule>>> {
        let rows: Vec<RuleRow> = sqlx::query_as(&format!(
            "SELECT {RULE_COLS} FROM mail_rule
             WHERE ($1::text IS NULL OR decision_point = $1) AND ($2 OR archived_at IS NULL)
             ORDER BY priority, created_at, id"
        ))
        .bind(dp.map(|k| k.as_str()))
        .bind(include_archived)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn update(
        conn: &mut PgConnection,
        r: &MailRule,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE mail_rule SET decision_point = $2, name = $3, priority = $4, conditions = $5,
                    value = $6, confidence = $7, enabled = $8, version = version + 1
             WHERE id = $1 AND version = $9 RETURNING version",
        )
        .bind(r.id.as_uuid())
        .bind(enum_str(&r.decision_point))
        .bind(&r.name)
        .bind(r.priority)
        .bind(conditions_json(&r.conditions)?)
        .bind(&r.value)
        .bind(r.confidence)
        .bind(r.enabled)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "mail_rule", r.id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: MailRuleId) -> Result<()> {
        one(sqlx::query(
            "UPDATE mail_rule SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(conn)
        .await?
        .rows_affected())
    }
}

// ---- categories ----

pub struct CategoryRepo;

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: Uuid,
    household_id: Uuid,
    key: String,
    label: String,
    behaviour: String,
    notify_time_sensitive: bool,
    auto_actions: Vec<String>,
    email_tasks: String,
    color: Option<String>,
    ord: i32,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const CATEGORY_COLS: &str = "id, household_id, key, label, behaviour, notify_time_sensitive,
    auto_actions, email_tasks, color, ord, archived_at, version, created_at, updated_at";

impl TryFrom<CategoryRow> for Versioned<Category> {
    type Error = StoreError;
    fn try_from(r: CategoryRow) -> Result<Self> {
        let auto_actions = r
            .auto_actions
            .iter()
            .map(|a| parse_enum::<AutoAction>(a))
            .collect::<Result<Vec<_>>>()?;
        Ok(Versioned {
            version: r.version,
            value: Category {
                id: r.id.into(),
                household_id: r.household_id.into(),
                key: r.key,
                label: r.label,
                behaviour: parse_enum(&r.behaviour)?,
                notify_time_sensitive: r.notify_time_sensitive,
                auto_actions,
                email_tasks: parse_enum(&r.email_tasks)?,
                color: r.color,
                ord: r.ord,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

fn actions(c: &Category) -> Vec<String> {
    c.auto_actions.iter().map(enum_str).collect()
}

impl CategoryRepo {
    pub async fn insert(conn: &mut PgConnection, c: &Category) -> Result<Versioned<Category>> {
        let row: CategoryRow = sqlx::query_as(&format!(
            "INSERT INTO category (id, household_id, key, label, behaviour, notify_time_sensitive,
                                   auto_actions, email_tasks, color, ord)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING {CATEGORY_COLS}"
        ))
        .bind(c.id.as_uuid())
        .bind(c.household_id.as_uuid())
        .bind(&c.key)
        .bind(&c.label)
        .bind(enum_str(&c.behaviour))
        .bind(c.notify_time_sensitive)
        .bind(actions(c))
        .bind(enum_str(&c.email_tasks))
        .bind(&c.color)
        .bind(c.ord)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<Versioned<Category>>> {
        let rows: Vec<CategoryRow> = sqlx::query_as(&format!(
            "SELECT {CATEGORY_COLS} FROM category WHERE $1 OR archived_at IS NULL ORDER BY ord, key"
        ))
        .bind(include_archived)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: CategoryId,
    ) -> Result<Option<Versioned<Category>>> {
        let row: Option<CategoryRow> = sqlx::query_as(&format!(
            "SELECT {CATEGORY_COLS} FROM category WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn get_by_key(
        conn: &mut PgConnection,
        key: &str,
    ) -> Result<Option<Versioned<Category>>> {
        let row: Option<CategoryRow> = sqlx::query_as(&format!(
            "SELECT {CATEGORY_COLS} FROM category WHERE key = $1"
        ))
        .bind(key)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Updates everything but the key. `archived_at` follows `c.archived_at` (set or clear).
    pub async fn update(
        conn: &mut PgConnection,
        c: &Category,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE category SET label = $2, behaviour = $3, notify_time_sensitive = $4,
                    auto_actions = $5, email_tasks = $6, color = $7, ord = $8, archived_at = $9,
                    version = version + 1
             WHERE id = $1 AND version = $10 RETURNING version",
        )
        .bind(c.id.as_uuid())
        .bind(&c.label)
        .bind(enum_str(&c.behaviour))
        .bind(c.notify_time_sensitive)
        .bind(actions(c))
        .bind(enum_str(&c.email_tasks))
        .bind(&c.color)
        .bind(c.ord)
        .bind(c.archived_at)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "category", c.id.as_uuid()).await),
        }
    }
}
