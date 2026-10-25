use chrono::{DateTime, Utc};
use fa_domain::api::HouseholdPatch;
use fa_domain::{Household, HouseholdId, Location};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

pub struct HouseholdRepo;

fn weights(w: &fa_domain::tasks::NowWeights) -> Result<serde_json::Value> {
    serde_json::to_value(w).map_err(|e| StoreError::Corrupt(format!("now_weights: {e}")))
}

struct Row {
    id: Uuid,
    name: String,
    tz: String,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    location_label: Option<String>,
    meal_plan_horizon_weeks: i32,
    dashboard_completion_override: bool,
    units: String,
    calendar_past_days: i32,
    calendar_future_days: i32,
    weather_retention_days: i32,
    private_events_as_busy: bool,
    now_list_size: i32,
    now_weights: serde_json::Value,
    quiet_from: chrono::NaiveTime,
    quiet_until: chrono::NaiveTime,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Versioned<Household> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        let location = match (r.location_lat, r.location_lon, r.location_label) {
            (Some(lat), Some(lon), Some(label)) => Some(Location { lat, lon, label }),
            _ => None,
        };
        Ok(Versioned {
            version: r.version,
            value: Household {
                id: r.id.into(),
                name: r.name,
                tz: r.tz,
                location,
                meal_plan_horizon_weeks: r.meal_plan_horizon_weeks,
                dashboard_completion_override: r.dashboard_completion_override,
                units: parse_enum(&r.units)?,
                calendar_past_days: r.calendar_past_days,
                calendar_future_days: r.calendar_future_days,
                weather_retention_days: r.weather_retention_days,
                private_events_as_busy: r.private_events_as_busy,
                now_list_size: r.now_list_size,
                now_weights: serde_json::from_value(r.now_weights)
                    .map_err(|e| StoreError::Corrupt(format!("now_weights: {e}")))?,
                quiet_from: r.quiet_from,
                quiet_until: r.quiet_until,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
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
                                    meal_plan_horizon_weeks, dashboard_completion_override,
                                    units, calendar_past_days, calendar_future_days,
                                    weather_retention_days, private_events_as_busy,
                                    now_list_size, now_weights, quiet_from, quiet_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
             RETURNING id, name, tz, location_lat, location_lon, location_label,
                       meal_plan_horizon_weeks, dashboard_completion_override, units,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, version,
                       created_at, updated_at",
            h.id.as_uuid(),
            h.name,
            h.tz,
            lat,
            lon,
            label,
            h.meal_plan_horizon_weeks,
            h.dashboard_completion_override,
            enum_str(&h.units),
            h.calendar_past_days,
            h.calendar_future_days,
            h.weather_retention_days,
            h.private_events_as_busy,
            h.now_list_size,
            weights(&h.now_weights)?,
            h.quiet_from,
            h.quiet_until
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: HouseholdId,
    ) -> Result<Option<Versioned<Household>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, name, tz, location_lat, location_lon, location_label,
                    meal_plan_horizon_weeks, dashboard_completion_override, units,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, version,
                    created_at, updated_at
             FROM household WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// All households visible to the transaction (all of them in a system tx).
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<Versioned<Household>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, name, tz, location_lat, location_lon, location_label,
                    meal_plan_horizon_weeks, dashboard_completion_override, units,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, version,
                    created_at, updated_at
             FROM household ORDER BY created_at, id"
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
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
                    dashboard_completion_override = $8, units = $9, calendar_past_days = $10,
                    calendar_future_days = $11, weather_retention_days = $12,
                    private_events_as_busy = $13, now_list_size = $14, now_weights = $15,
                    quiet_from = $16, quiet_until = $17, version = version + 1
             WHERE id = $1 AND version = $18
             RETURNING version",
            h.id.as_uuid(),
            h.name,
            h.tz,
            lat,
            lon,
            label,
            h.meal_plan_horizon_weeks,
            h.dashboard_completion_override,
            enum_str(&h.units),
            h.calendar_past_days,
            h.calendar_future_days,
            h.weather_retention_days,
            h.private_events_as_busy,
            h.now_list_size,
            weights(&h.now_weights)?,
            h.quiet_from,
            h.quiet_until,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "household", h.id.as_uuid()).await),
        }
    }

    /// Applies the set fields of a patch. Returns the new version, or `Conflict` if stale.
    pub async fn update_settings(
        conn: &mut PgConnection,
        id: HouseholdId,
        p: &HouseholdPatch,
        expected_version: i32,
    ) -> Result<i32> {
        let (lat, lon, label) = match &p.location {
            Some(l) => (Some(l.lat), Some(l.lon), Some(l.label.clone())),
            None => (None, None, None),
        };
        let v = sqlx::query_scalar!(
            "UPDATE household SET
                    name = COALESCE($2, name), tz = COALESCE($3, tz),
                    location_lat = COALESCE($4, location_lat), location_lon = COALESCE($5, location_lon),
                    location_label = COALESCE($6, location_label), units = COALESCE($7, units),
                    calendar_past_days = COALESCE($8, calendar_past_days),
                    calendar_future_days = COALESCE($9, calendar_future_days),
                    weather_retention_days = COALESCE($10, weather_retention_days),
                    private_events_as_busy = COALESCE($11, private_events_as_busy),
                    now_list_size = COALESCE($12, now_list_size),
                    now_weights = COALESCE($13, now_weights),
                    quiet_from = COALESCE($14, quiet_from),
                    quiet_until = COALESCE($15, quiet_until),
                    dashboard_completion_override = COALESCE($16, dashboard_completion_override),
                    version = version + 1
             WHERE id = $1 AND version = $17
             RETURNING version",
            id.as_uuid(),
            p.name,
            p.tz,
            lat,
            lon,
            label,
            p.units.as_ref().map(enum_str),
            p.calendar_past_days,
            p.calendar_future_days,
            p.weather_retention_days,
            p.private_events_as_busy,
            p.now_list_size,
            p.now_weights.as_ref().map(weights).transpose()?,
            p.quiet_from,
            p.quiet_until,
            p.dashboard_completion_override,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "household", id.as_uuid()).await),
        }
    }
}
