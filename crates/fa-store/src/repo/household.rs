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

fn gender_terms(g: &fa_domain::GenderTerms) -> Result<serde_json::Value> {
    serde_json::to_value(g).map_err(|e| StoreError::Corrupt(format!("gender_terms: {e}")))
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
    week_start: String,
    calendar_past_days: i32,
    calendar_future_days: i32,
    weather_retention_days: i32,
    private_events_as_busy: bool,
    now_list_size: i32,
    now_weights: serde_json::Value,
    quiet_from: chrono::NaiveTime,
    quiet_until: chrono::NaiveTime,
    currency: String,
    gender_terms: serde_json::Value,
    disabled_signals: Vec<String>,
    events_auto_write: bool,
    events_extract_threshold: f32,
    events_route_threshold: f32,
    bill_lead_days: i32,
    inbox_task_expiry_days: i32,
    inbox_contact_expiry_days: i32,
    mail_content_policy: String,
    mail_examples_per_category: i32,
    mail_max_pending_days: i32,
    dashboard_pending_approvals: bool,
    setup_completed_at: Option<DateTime<Utc>>,
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
                week_start: parse_enum(&r.week_start)?,
                calendar_past_days: r.calendar_past_days,
                calendar_future_days: r.calendar_future_days,
                weather_retention_days: r.weather_retention_days,
                private_events_as_busy: r.private_events_as_busy,
                now_list_size: r.now_list_size,
                now_weights: serde_json::from_value(r.now_weights)
                    .map_err(|e| StoreError::Corrupt(format!("now_weights: {e}")))?,
                quiet_from: r.quiet_from,
                quiet_until: r.quiet_until,
                currency: r.currency,
                gender_terms: serde_json::from_value(r.gender_terms)
                    .map_err(|e| StoreError::Corrupt(format!("gender_terms: {e}")))?,
                disabled_signals: r.disabled_signals,
                events: fa_domain::EventsSettings {
                    auto_write: r.events_auto_write,
                    extract_threshold: r.events_extract_threshold,
                    route_threshold: r.events_route_threshold,
                },
                inbox: fa_domain::InboxSettings {
                    task_expiry_days: r.inbox_task_expiry_days,
                    contact_expiry_days: r.inbox_contact_expiry_days,
                },
                retention_mail: fa_domain::MailRetention {
                    content_policy: parse_enum(&r.mail_content_policy)?,
                    examples_per_category: r.mail_examples_per_category,
                    max_pending_days: r.mail_max_pending_days,
                },
                bill_lead_days: r.bill_lead_days,
                dashboard_pending_approvals: r.dashboard_pending_approvals,
                setup_completed_at: r.setup_completed_at,
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
                                    now_list_size, now_weights, quiet_from, quiet_until,
                                    currency, gender_terms, disabled_signals, events_auto_write,
                                    events_extract_threshold, events_route_threshold, bill_lead_days,
                                    inbox_task_expiry_days, inbox_contact_expiry_days,
                                    mail_content_policy, mail_examples_per_category,
                                    mail_max_pending_days, dashboard_pending_approvals,
                                    setup_completed_at, week_start)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                     $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)
             RETURNING id, name, tz, location_lat, location_lon, location_label,
                       meal_plan_horizon_weeks, dashboard_completion_override, units, week_start,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, currency, gender_terms, disabled_signals, events_auto_write, events_extract_threshold,
                       events_route_threshold, bill_lead_days, inbox_task_expiry_days, inbox_contact_expiry_days,
                       mail_content_policy, mail_examples_per_category, mail_max_pending_days, dashboard_pending_approvals,
                       setup_completed_at, version,
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
            h.quiet_until,
            h.currency,
            gender_terms(&h.gender_terms)?,
            &h.disabled_signals,
            h.events.auto_write,
            h.events.extract_threshold,
            h.events.route_threshold,
            h.bill_lead_days,
            h.inbox.task_expiry_days,
            h.inbox.contact_expiry_days,
            enum_str(&h.retention_mail.content_policy),
            h.retention_mail.examples_per_category,
            h.retention_mail.max_pending_days,
            h.dashboard_pending_approvals,
            h.setup_completed_at,
            enum_str(&h.week_start)
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
                    meal_plan_horizon_weeks, dashboard_completion_override, units, week_start,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, currency, gender_terms, disabled_signals, events_auto_write, events_extract_threshold,
                       events_route_threshold, bill_lead_days, inbox_task_expiry_days, inbox_contact_expiry_days,
                       mail_content_policy, mail_examples_per_category, mail_max_pending_days, dashboard_pending_approvals,
                       setup_completed_at, version,
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
                    meal_plan_horizon_weeks, dashboard_completion_override, units, week_start,
                       calendar_past_days, calendar_future_days, weather_retention_days,
                    private_events_as_busy, now_list_size, now_weights, quiet_from, quiet_until, currency, gender_terms, disabled_signals, events_auto_write, events_extract_threshold,
                       events_route_threshold, bill_lead_days, inbox_task_expiry_days, inbox_contact_expiry_days,
                       mail_content_policy, mail_examples_per_category, mail_max_pending_days, dashboard_pending_approvals,
                       setup_completed_at, version,
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
                    quiet_from = $16, quiet_until = $17, currency = $18, gender_terms = $19,
                    disabled_signals = $20, events_auto_write = $21, events_extract_threshold = $22,
                    events_route_threshold = $23, bill_lead_days = $24, inbox_task_expiry_days = $25,
                    inbox_contact_expiry_days = $26, mail_content_policy = $27,
                    mail_examples_per_category = $28, mail_max_pending_days = $29,
                    dashboard_pending_approvals = $30, week_start = $32, version = version + 1
             WHERE id = $1 AND version = $31
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
            h.currency,
            gender_terms(&h.gender_terms)?,
            &h.disabled_signals,
            h.events.auto_write,
            h.events.extract_threshold,
            h.events.route_threshold,
            h.bill_lead_days,
            h.inbox.task_expiry_days,
            h.inbox.contact_expiry_days,
            enum_str(&h.retention_mail.content_policy),
            h.retention_mail.examples_per_category,
            h.retention_mail.max_pending_days,
            h.dashboard_pending_approvals,
            expected_version,
            enum_str(&h.week_start)
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
                    currency = COALESCE($17, currency), gender_terms = COALESCE($18, gender_terms),
                    events_auto_write = COALESCE($19, events_auto_write),
                    events_extract_threshold = COALESCE($20, events_extract_threshold),
                    events_route_threshold = COALESCE($21, events_route_threshold),
                    bill_lead_days = COALESCE($22, bill_lead_days),
                    inbox_task_expiry_days = COALESCE($23, inbox_task_expiry_days),
                    inbox_contact_expiry_days = COALESCE($24, inbox_contact_expiry_days),
                    mail_content_policy = COALESCE($25, mail_content_policy),
                    mail_examples_per_category = COALESCE($26, mail_examples_per_category),
                    mail_max_pending_days = COALESCE($27, mail_max_pending_days),
                    dashboard_pending_approvals = COALESCE($28, dashboard_pending_approvals),
                    week_start = COALESCE($30, week_start),
                    version = version + 1
             WHERE id = $1 AND version = $29
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
            p.currency,
            p.gender_terms.as_ref().map(gender_terms).transpose()?,
            p.events.as_ref().map(|e| e.auto_write),
            p.events.as_ref().map(|e| e.extract_threshold),
            p.events.as_ref().map(|e| e.route_threshold),
            p.bill_lead_days,
            p.inbox.as_ref().map(|i| i.task_expiry_days),
            p.inbox.as_ref().map(|i| i.contact_expiry_days),
            p.retention_mail.as_ref().map(|r| enum_str(&r.content_policy)),
            p.retention_mail.as_ref().map(|r| r.examples_per_category),
            p.retention_mail.as_ref().map(|r| r.max_pending_days),
            p.dashboard_pending_approvals,
            expected_version,
            p.week_start.as_ref().map(enum_str)
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "household", id.as_uuid()).await),
        }
    }

    /// Built-in signal keys switched off for the household.
    pub async fn disabled_signals(conn: &mut PgConnection, id: HouseholdId) -> Result<Vec<String>> {
        sqlx::query_scalar!(
            "SELECT disabled_signals FROM household WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?
        .ok_or(StoreError::NotFound)
    }

    pub async fn set_disabled_signals(
        conn: &mut PgConnection,
        id: HouseholdId,
        keys: &[String],
    ) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE household SET disabled_signals = $2, version = version + 1 WHERE id = $1",
            id.as_uuid(),
            keys
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
