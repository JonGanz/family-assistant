use chrono::{DateTime, Utc};
use fa_domain::{Household, HouseholdId, Location};
use sqlx::PgConnection;
use uuid::Uuid;

use super::conflict_or_missing;
use crate::{Result, Versioned};

pub struct HouseholdRepo;

struct Row {
    id: Uuid,
    name: String,
    tz: String,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    location_label: Option<String>,
    meal_plan_horizon_weeks: i32,
    dashboard_completion_override: bool,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<Row> for Versioned<Household> {
    fn from(r: Row) -> Self {
        let location = match (r.location_lat, r.location_lon, r.location_label) {
            (Some(lat), Some(lon), Some(label)) => Some(Location { lat, lon, label }),
            _ => None,
        };
        Versioned {
            version: r.version,
            value: Household {
                id: r.id.into(),
                name: r.name,
                tz: r.tz,
                location,
                meal_plan_horizon_weeks: r.meal_plan_horizon_weeks,
                dashboard_completion_override: r.dashboard_completion_override,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        }
    }
}

impl HouseholdRepo {
    pub async fn insert(conn: &mut PgConnection, h: &Household) -> Result<Versioned<Household>> {
        let (lat, lon, label) = match &h.location {
            Some(l) => (Some(l.lat), Some(l.lon), Some(l.label.clone())),
            None => (None, None, None),
        };
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO household (id, name, tz, location_lat, location_lon, location_label,
                                    meal_plan_horizon_weeks, dashboard_completion_override)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, name, tz, location_lat, location_lon, location_label,
                       meal_plan_horizon_weeks, dashboard_completion_override, version,
                       created_at, updated_at",
            h.id.as_uuid(),
            h.name,
            h.tz,
            lat,
            lon,
            label,
            h.meal_plan_horizon_weeks,
            h.dashboard_completion_override
        )
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: HouseholdId,
    ) -> Result<Option<Versioned<Household>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, name, tz, location_lat, location_lon, location_label,
                    meal_plan_horizon_weeks, dashboard_completion_override, version,
                    created_at, updated_at
             FROM household WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// All households visible to the transaction (all of them in a system tx).
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<Versioned<Household>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, name, tz, location_lat, location_lon, location_label,
                    meal_plan_horizon_weeks, dashboard_completion_override, version,
                    created_at, updated_at
             FROM household ORDER BY created_at, id"
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Returns the new version, or `Conflict` if `expected_version` is stale.
    pub async fn update(
        conn: &mut PgConnection,
        h: &Household,
        expected_version: i32,
    ) -> Result<i32> {
        let (lat, lon, label) = match &h.location {
            Some(l) => (Some(l.lat), Some(l.lon), Some(l.label.clone())),
            None => (None, None, None),
        };
        let v = sqlx::query_scalar!(
            "UPDATE household SET name = $2, tz = $3, location_lat = $4, location_lon = $5,
                    location_label = $6, meal_plan_horizon_weeks = $7,
                    dashboard_completion_override = $8, version = version + 1
             WHERE id = $1 AND version = $9
             RETURNING version",
            h.id.as_uuid(),
            h.name,
            h.tz,
            lat,
            lon,
            label,
            h.meal_plan_horizon_weeks,
            h.dashboard_completion_override,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "household", h.id.as_uuid()).await),
        }
    }
}
