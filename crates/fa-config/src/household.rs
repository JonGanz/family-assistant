//! Creating a household from the setup wizard's first step.

use chrono::Utc;
use fa_domain::system_api::NewSetupHousehold;
use fa_domain::timefmt::parse_hhmm;
use fa_domain::{Household, HouseholdId, Location, Units};
use fa_store::{HouseholdRepo, Store, StoreError};

pub const DEFAULT_CALENDAR_PAST_DAYS: i32 = 30;
pub const DEFAULT_CALENDAR_FUTURE_DAYS: i32 = 400;
pub const DEFAULT_WEATHER_RETENTION_DAYS: i32 = 7;
pub const DEFAULT_PRIVATE_EVENTS_AS_BUSY: bool = true;

#[derive(Debug, thiserror::Error)]
pub enum HouseholdError {
    #[error("invalid household:\n  - {}", .0.join("\n  - "))]
    Invalid(Vec<String>),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// A household with every setting at its default.
fn new_household(
    id: HouseholdId,
    name: &str,
    tz: &str,
    location: Option<Location>,
    units: Units,
    currency: String,
) -> Household {
    let now = Utc::now();
    Household {
        id,
        name: name.to_string(),
        tz: tz.to_string(),
        location,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: true,
        units,
        week_start: Default::default(),
        calendar_past_days: DEFAULT_CALENDAR_PAST_DAYS,
        calendar_future_days: DEFAULT_CALENDAR_FUTURE_DAYS,
        weather_retention_days: DEFAULT_WEATHER_RETENTION_DAYS,
        private_events_as_busy: DEFAULT_PRIVATE_EVENTS_AS_BUSY,
        now_list_size: 7,
        now_weights: Default::default(),
        quiet_from: quiet(None, 21),
        quiet_until: quiet(None, 7),
        currency,
        gender_terms: Default::default(),
        disabled_signals: vec![],
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: now,
        updated_at: now,
    }
}

/// Creates a household with defaults from the setup wizard's first step.
pub async fn create_household(
    store: &Store,
    new: &NewSetupHousehold,
) -> Result<HouseholdId, HouseholdError> {
    let mut errs = Vec::new();
    let name = new.name.trim();
    if name.is_empty() {
        errs.push("household name must not be empty".to_string());
    }
    if new.tz.parse::<chrono_tz::Tz>().is_err() {
        errs.push(format!("'{}' is not an IANA time zone", new.tz));
    }
    let currency = new.currency.trim().to_ascii_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        errs.push(format!(
            "'{}' is not an ISO 4217 currency code",
            new.currency
        ));
    }
    if let Some(l) = &new.location
        && !((-90.0..=90.0).contains(&l.lat) && (-180.0..=180.0).contains(&l.lon))
    {
        errs.push("location latitude or longitude is out of range".to_string());
    }
    if !errs.is_empty() {
        return Err(HouseholdError::Invalid(errs));
    }
    let id = HouseholdId::new();
    let h = new_household(
        id,
        name,
        &new.tz,
        new.location.as_ref().map(|l| Location {
            lat: l.lat,
            lon: l.lon,
            label: l.label.clone(),
        }),
        new.units,
        currency,
    );
    let mut tx = store.tx_system().await?;
    HouseholdRepo::insert(&mut tx, &h).await?;
    tx.commit().await?;
    tracing::info!(household = %id, "household created");
    Ok(id)
}

fn quiet(s: Option<&str>, default_hour: u32) -> chrono::NaiveTime {
    s.and_then(parse_hhmm)
        .or_else(|| chrono::NaiveTime::from_hms_opt(default_hour, 0, 0))
        .unwrap_or(chrono::NaiveTime::MIN)
}
