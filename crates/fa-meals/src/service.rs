//! `MealsService`: the handle every route and job uses. The per-area methods live in the sibling
//! modules (`recipes`, `catalogue`, `settings`, `plan`, `templates`, `grocery`).
//!
//! Authorisation stays in `fa-api`; this layer enforces data rules and returns typed errors.

use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use fa_domain::{DeviceId, HouseholdId, MemberId, Units};
use fa_llm::Provider;
use fa_store::{
    ChangeBus, HouseholdRepo, JobRepo, MealsSettingsRepo, MealsSettingsRow, NewJob, Store,
};
use serde_json::json;
use sqlx::PgConnection;

use crate::error::{MealsError, Result};
use crate::normalise::Normaliser;

pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Undo window of a multi-row change.
pub const UNDO_MINUTES: i64 = 10;

/// Who made a change: a signed-in member, a dashboard device, or both (an elevated device).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorRef {
    pub member: Option<MemberId>,
    pub device: Option<DeviceId>,
}

#[derive(Clone)]
pub struct MealsService {
    pub(crate) store: Store,
    pub(crate) llm: Option<Arc<Provider>>,
    pub(crate) allow_private_hosts: bool,
    pub(crate) clock: Clock,
    pub(crate) normaliser: Arc<Normaliser>,
}

/// The household facts the services need.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HouseholdCtx {
    pub tz: Tz,
    pub units: Units,
    pub horizon_weeks: i64,
}

impl MealsService {
    pub fn new(store: Store, llm: Option<Arc<Provider>>, allow_private_hosts: bool) -> Self {
        Self {
            store,
            llm,
            allow_private_hosts,
            clock: Arc::new(Utc::now),
            normaliser: Arc::new(Normaliser::standard()),
        }
    }

    /// A fixed or fake clock, for tests.
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;
        self
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    pub(crate) async fn ctx(
        &self,
        conn: &mut PgConnection,
        h: HouseholdId,
    ) -> Result<HouseholdCtx> {
        let hh = HouseholdRepo::get(conn, h)
            .await?
            .ok_or(MealsError::NotFound)?
            .value;
        Ok(HouseholdCtx {
            tz: hh.tz.parse().unwrap_or(Tz::UTC),
            units: hh.units,
            horizon_weeks: i64::from(hh.meal_plan_horizon_weeks.max(1)),
        })
    }

    /// The household's calendar day.
    pub(crate) fn today(&self, ctx: &HouseholdCtx) -> NaiveDate {
        self.now().with_timezone(&ctx.tz).date_naive()
    }

    /// Last date writes are allowed (inclusive): today plus the horizon in weeks.
    pub(crate) fn horizon_end(&self, ctx: &HouseholdCtx) -> NaiveDate {
        self.today(ctx) + Duration::days(7 * ctx.horizon_weeks)
    }

    /// The settings row and the normaliser that uses the household's descriptor list.
    pub(crate) async fn settings_and_normaliser(
        &self,
        conn: &mut PgConnection,
        h: HouseholdId,
    ) -> Result<(MealsSettingsRow, Arc<Normaliser>)> {
        let s = MealsSettingsRepo::get_or_default(conn, h).await?;
        let n = if s.descriptors.is_empty() {
            self.normaliser.clone()
        } else {
            Arc::new(Normaliser::with_descriptors(&s.descriptors))
        };
        Ok((s, n))
    }

    /// Whether an endpoint is configured for the household.
    pub(crate) async fn llm_ready(&self, h: HouseholdId) -> bool {
        match &self.llm {
            Some(p) => matches!(p.client_for(h).await, Ok(Some(_))),
            None => false,
        }
    }
}

pub(crate) async fn publish(conn: &mut PgConnection, h: HouseholdId, kind: &str) -> Result<()> {
    ChangeBus::publish(conn, h, kind, &json!({})).await?;
    Ok(())
}

/// Enqueues a job inside the caller's transaction. A repeated key is a no-op.
pub(crate) async fn enqueue(
    conn: &mut PgConnection,
    h: HouseholdId,
    kind: &str,
    payload: serde_json::Value,
    key: String,
) -> Result<bool> {
    let mut job = NewJob::new(kind);
    job.household_id = Some(h);
    job.payload = payload;
    job.idempotency_key = Some(key);
    job.max_attempts = 3;
    Ok(JobRepo::enqueue(conn, &job).await?.is_some())
}

pub(crate) fn to_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

pub(crate) fn to_u32(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

pub(crate) fn opt_u32(v: Option<i32>) -> Option<u32> {
    v.and_then(|v| u32::try_from(v).ok())
}
