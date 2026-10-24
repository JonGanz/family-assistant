use chrono::{DateTime, Utc};
use fa_domain::{RuleAction, RuleStatus, Severity, WeatherRule, WeatherRuleId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct WeatherRuleRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    name: String,
    condition: String,
    action: String,
    severity: String,
    message: String,
    task_definition: Option<serde_json::Value>,
    status: String,
    suggested_by: String,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<WeatherRule> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: WeatherRule {
                id: r.id.into(),
                household_id: r.household_id.into(),
                name: r.name,
                condition: r.condition,
                action: parse_enum(&r.action)?,
                severity: parse_enum(&r.severity)?,
                message: r.message,
                task_definition: r.task_definition,
                status: parse_enum(&r.status)?,
                suggested_by: parse_enum(&r.suggested_by)?,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl WeatherRuleRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        r: &WeatherRule,
    ) -> Result<Versioned<WeatherRule>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO weather_rule (id, household_id, name, condition, action, severity, message,
                                       task_definition, status, suggested_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING id, household_id, name, condition, action, severity, message, task_definition,
                       status, suggested_by, archived_at, version, created_at, updated_at",
            r.id.as_uuid(),
            r.household_id.as_uuid(),
            r.name,
            r.condition,
            enum_str(&r.action),
            enum_str(&r.severity),
            r.message,
            r.task_definition,
            enum_str(&r.status),
            enum_str(&r.suggested_by)
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: WeatherRuleId,
    ) -> Result<Option<Versioned<WeatherRule>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, condition, action, severity, message, task_definition,
                    status, suggested_by, archived_at, version, created_at, updated_at
             FROM weather_rule WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Non-archived rules, optionally only those with one of the given statuses.
    pub async fn list(
        conn: &mut PgConnection,
        statuses: Option<&[RuleStatus]>,
    ) -> Result<Vec<Versioned<WeatherRule>>> {
        let st: Option<Vec<String>> = statuses.map(|s| s.iter().map(enum_str).collect());
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, name, condition, action, severity, message, task_definition,
                    status, suggested_by, archived_at, version, created_at, updated_at
             FROM weather_rule
             WHERE archived_at IS NULL AND ($1::text[] IS NULL OR status = ANY($1))
             ORDER BY created_at, id",
            st.as_deref()
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Replaces the editable fields. Returns the new version.
    pub async fn update(
        conn: &mut PgConnection,
        r: &WeatherRule,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE weather_rule SET name = $2, condition = $3, action = $4, severity = $5, message = $6,
                    task_definition = $7, status = $8, version = version + 1
             WHERE id = $1 AND version = $9
             RETURNING version",
            r.id.as_uuid(),
            r.name,
            r.condition,
            enum_str(&r.action),
            enum_str(&r.severity),
            r.message,
            r.task_definition,
            enum_str(&r.status),
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "weather_rule", r.id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: WeatherRuleId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE weather_rule SET archived_at = COALESCE(archived_at, now()), version = version + 1
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

    /// Seeds a default rule for the transaction's household unless one with that name and
    /// `suggested_by = default` exists already (archived ones count). Returns whether it inserted.
    pub async fn ensure_default(
        conn: &mut PgConnection,
        name: &str,
        condition: &str,
        action: RuleAction,
        severity: Severity,
        message: &str,
    ) -> Result<bool> {
        let n = sqlx::query!(
            "INSERT INTO weather_rule (id, household_id, name, condition, action, severity, message,
                                       status, suggested_by)
             SELECT $1, app_household(), $2, $3, $4, $5, $6, 'active', 'default'
             WHERE app_household() IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM weather_rule WHERE name = $2 AND suggested_by = 'default')",
            WeatherRuleId::new().as_uuid(),
            name,
            condition,
            enum_str(&action),
            enum_str(&severity),
            message
        )
        .execute(conn)
        .await?
        .rows_affected();
        Ok(n > 0)
    }
}
