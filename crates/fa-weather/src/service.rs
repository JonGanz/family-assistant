use std::sync::Arc;

use chrono::{DateTime, Duration, Timelike, Utc};
use chrono_tz::Tz;
use fa_connectors::{Http, openmeteo};
use fa_domain::api::WeatherView;
use fa_domain::{AdvisorySource, Household, HouseholdId, RuleAction, RuleStatus, Units};
use fa_jobs::JobError;
use fa_store::{
    AdvisoryRepo, ChangeBus, JobRepo, NewJob, Store, SystemStatusRepo, TaskDefinitionRepo,
    WeatherRuleRepo, WeatherSnapshotRepo,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::WeatherError;
use crate::dsl::{self, ParseError};
use crate::eval::{SnapshotData, Span, evaluate_data, render_message};

pub const STATUS_KEY: &str = "weather";
const ADVISORY_GRACE_DAYS: i64 = 7;

/// Process settings handed in by `fa-server`.
#[derive(Debug, Clone)]
pub struct WeatherConfig {
    /// Open-Meteo base URL (`FA_OPEN_METEO_URL`).
    pub open_meteo_url: String,
    /// Allow plain-http base URLs (stubs and tests).
    pub allow_http: bool,
    /// Allows loopback, LAN and other private addresses (C-05).
    pub allow_private_hosts: bool,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            open_meteo_url: openmeteo::DEFAULT_BASE.to_string(),
            allow_http: false,
            allow_private_hosts: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Preview {
    pub matches: bool,
    pub value: Option<f64>,
    pub when: Option<Span>,
    /// The rendered message, when one was supplied.
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Validation {
    pub ok: bool,
    pub error: Option<ParseError>,
    pub preview: Option<Preview>,
}

#[derive(Clone)]
pub struct WeatherService {
    store: Store,
    http: Http,
    config: Arc<WeatherConfig>,
}

fn parse_tz(h: &Household) -> Tz {
    h.tz.parse().unwrap_or_else(|_| {
        tracing::warn!("household has an unknown timezone, using UTC");
        Tz::UTC
    })
}

impl WeatherService {
    pub fn new(store: Store, config: WeatherConfig) -> Result<Self, WeatherError> {
        let http = Http::with_policy(config.allow_http, config.allow_private_hosts)?;
        Ok(Self::with_http(store, http, config))
    }

    pub fn with_http(store: Store, http: Http, config: WeatherConfig) -> Self {
        Self {
            store,
            http,
            config: Arc::new(config),
        }
    }

    async fn household(&self, id: HouseholdId) -> Result<Option<Household>, WeatherError> {
        let mut tx = self.store.tx_for(id).await?;
        let h = fa_store::HouseholdRepo::get(&mut tx, id).await?;
        tx.commit().await?;
        Ok(h.map(|v| v.value))
    }

    /// Fetches a forecast, stores the snapshot and queues `weather.evaluate`. Households without a
    /// location are skipped. A failure is recorded in `system_status` and returned so the job retries.
    pub async fn fetch(&self, id: HouseholdId) -> Result<(), WeatherError> {
        let Some(h) = self.household(id).await? else {
            return Ok(());
        };
        let Some(loc) = h.location.clone() else {
            tracing::debug!("household has no location, skipping weather fetch");
            return Ok(());
        };
        let forecast = match openmeteo::forecast(
            &self.http,
            &self.config.open_meteo_url,
            loc.lat,
            loc.lon,
            &h.tz,
        )
        .await
        {
            Ok(f) => f,
            Err(e) => {
                self.set_status(json!({"ok": false, "error": e.to_string(), "at": Utc::now()}))
                    .await?;
                return Err(e.into());
            }
        };
        let mut tx = self.store.tx_for(id).await?;
        let snap = WeatherSnapshotRepo::insert(
            &mut tx,
            "open-meteo",
            &forecast.current,
            &forecast.hourly,
            &forecast.daily,
        )
        .await?;
        let mut job = NewJob::new("weather.evaluate");
        job.household_id = Some(id);
        job.payload = json!({"household": id});
        job.idempotency_key = Some(format!("weather.evaluate:{snap}"));
        JobRepo::enqueue(&mut tx, &job).await?;
        ChangeBus::publish(&mut tx, id, "weather.updated", &json!({})).await?;
        tx.commit().await?;
        self.set_status(json!({"ok": true, "at": Utc::now()})).await
    }

    async fn set_status(&self, v: Value) -> Result<(), WeatherError> {
        let mut tx = self.store.tx_system().await?;
        SystemStatusRepo::set(&mut tx, STATUS_KEY, &v).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn evaluate(&self, id: HouseholdId) -> Result<(), WeatherError> {
        self.evaluate_at(id, Utc::now()).await
    }

    /// Runs every active alert rule against the latest snapshot as of `now`.
    pub async fn evaluate_at(
        &self,
        id: HouseholdId,
        now: DateTime<Utc>,
    ) -> Result<(), WeatherError> {
        let Some(h) = self.household(id).await? else {
            return Ok(());
        };
        let tz = parse_tz(&h);
        let mut tx = self.store.tx_for(id).await?;
        let Some(snapshot) = WeatherSnapshotRepo::latest(&mut tx).await? else {
            return Ok(());
        };
        let data = SnapshotData::new(&snapshot, tz);
        let rules = WeatherRuleRepo::list(&mut tx, Some(&[RuleStatus::Active])).await?;
        let mut changed = false;
        let mut task_jobs: Vec<(Value, String)> = Vec::new();
        let mut keep: Vec<String> = Vec::new();
        let mut all_parsed = true;
        for rule in rules.iter().map(|r| &r.value) {
            let parsed = match dsl::parse(&rule.condition) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(rule = %rule.id, error = %e, "weather rule does not parse");
                    all_parsed = false;
                    continue;
                }
            };
            let ev = evaluate_data(&parsed, &data, tz, now);
            let (true, Some((from, until)), Some(date)) = (ev.matches, ev.when, ev.key_date) else {
                continue;
            };
            if rule.action == RuleAction::Task {
                if rule.task_definition.is_none() {
                    tracing::debug!(rule = %rule.id, "task rule has no task definition, skipping");
                } else {
                    task_jobs.push((
                        json!({"household": id, "rule": rule.id, "date": date}),
                        format!("tasks.from_weather:{}:{date}", rule.id),
                    ));
                }
                continue;
            }
            let from = from
                .with_minute(0)
                .and_then(|d| d.with_second(0))
                .and_then(|d| d.with_nanosecond(0))
                .unwrap_or(from);
            let text = render_message(&rule.message, &ev, h.units);
            let key = format!("rule:{}:{date}", rule.id);
            let (_, c) = AdvisoryRepo::upsert_by_dedupe(
                &mut tx,
                Some(rule.id),
                &text,
                rule.severity,
                from,
                until,
                AdvisorySource::Rule,
                &key,
            )
            .await?;
            changed |= c;
            keep.push(key);
        }
        if all_parsed {
            changed |= AdvisoryRepo::retain_rule_advisories(&mut tx, &keep).await? > 0;
        }
        if changed {
            ChangeBus::publish(&mut tx, id, "advisory.updated", &json!({})).await?;
        }
        let mut warned = false;
        for (def, cond) in TaskDefinitionRepo::with_condition(&mut tx).await? {
            let ok = match dsl::parse(&cond) {
                Ok(p) => Some(evaluate_data(&p, &data, tz, now).matches),
                Err(e) => {
                    if !warned {
                        tracing::warn!(error = %e, "season condition does not parse");
                        warned = true;
                    }
                    None
                }
            };
            TaskDefinitionRepo::set_condition_outcome(&mut tx, def, ok, now).await?;
        }
        tx.commit().await?;
        for (payload, key) in task_jobs {
            fa_jobs::enqueue(
                &self.store,
                "tasks.from_weather",
                payload,
                None,
                0,
                Some(key),
            )
            .await
            .map_err(|e| match e {
                JobError::Store(e) => WeatherError::Store(e),
                other => WeatherError::Job(other.to_string()),
            })?;
        }
        Ok(())
    }

    /// Parses `condition` and previews it against the latest snapshot.
    pub async fn validate(
        &self,
        id: HouseholdId,
        condition: &str,
        message: Option<&str>,
    ) -> Result<Validation, WeatherError> {
        let rule = match dsl::parse(condition) {
            Ok(r) => r,
            Err(e) => {
                return Ok(Validation {
                    ok: false,
                    error: Some(e),
                    preview: None,
                });
            }
        };
        let Some(h) = self.household(id).await? else {
            return Ok(Validation {
                ok: true,
                error: None,
                preview: None,
            });
        };
        let tz = parse_tz(&h);
        let mut tx = self.store.tx_for(id).await?;
        let snapshot = WeatherSnapshotRepo::latest(&mut tx).await?;
        tx.commit().await?;
        let preview = snapshot.map(|s| {
            let ev = evaluate_data(&rule, &SnapshotData::new(&s, tz), tz, Utc::now());
            Preview {
                matches: ev.matches,
                value: ev.value,
                when: ev.when,
                message: message.map(|m| render_message(m, &ev, h.units)),
            }
        });
        Ok(Validation {
            ok: true,
            error: None,
            preview,
        })
    }

    /// Deletes snapshots past `weather_retention_days` and advisories expired over 7 days ago.
    pub async fn retention(&self, id: HouseholdId) -> Result<(u64, u64), WeatherError> {
        self.retention_at(id, Utc::now()).await
    }

    pub async fn retention_at(
        &self,
        id: HouseholdId,
        now: DateTime<Utc>,
    ) -> Result<(u64, u64), WeatherError> {
        let Some(h) = self.household(id).await? else {
            return Ok((0, 0));
        };
        let mut tx = self.store.tx_for(id).await?;
        let snaps = WeatherSnapshotRepo::delete_older_than(
            &mut tx,
            now - Duration::days(i64::from(h.weather_retention_days)),
        )
        .await?;
        let advisories =
            AdvisoryRepo::delete_expired(&mut tx, now - Duration::days(ADVISORY_GRACE_DAYS))
                .await?;
        tx.commit().await?;
        Ok((snaps, advisories))
    }

    pub async fn latest_view(&self, id: HouseholdId) -> Result<Option<WeatherView>, WeatherError> {
        self.latest_view_at(id, Utc::now()).await
    }

    pub async fn latest_view_at(
        &self,
        id: HouseholdId,
        now: DateTime<Utc>,
    ) -> Result<Option<WeatherView>, WeatherError> {
        let Some(h) = self.household(id).await? else {
            return Ok(None);
        };
        let tz = parse_tz(&h);
        let mut tx = self.store.tx_for(id).await?;
        let snapshot = WeatherSnapshotRepo::latest(&mut tx).await?;
        tx.commit().await?;
        Ok(snapshot.map(|s| build_view(&s, tz, now)))
    }

    /// All households with a location.
    pub async fn households_with_location(&self) -> Result<Vec<HouseholdId>, WeatherError> {
        let mut tx = self.store.tx_system().await?;
        let all = fa_store::HouseholdRepo::list(&mut tx).await?;
        tx.commit().await?;
        Ok(all
            .into_iter()
            .filter(|h| h.value.location.is_some())
            .map(|h| h.value.id)
            .collect())
    }

    pub fn units_of(h: &Household) -> Units {
        h.units
    }
}

fn truncate(section: &Value, start: usize, len: usize) -> Value {
    let Some(obj) = section.as_object() else {
        return section.clone();
    };
    obj.iter()
        .map(|(k, v)| {
            let v = match v.as_array() {
                Some(a) => Value::Array(a.iter().skip(start).take(len).cloned().collect()),
                None => v.clone(),
            };
            (k.clone(), v)
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

pub(crate) fn build_view(
    s: &fa_domain::WeatherSnapshot,
    tz: Tz,
    now: DateTime<Utc>,
) -> WeatherView {
    let data = SnapshotData::new(s, tz);
    let fc = data.forecast();
    let start = fc
        .hourly_series("temperature_2m")
        .iter()
        .position(|(t, _)| *t + Duration::hours(1) > now)
        .unwrap_or(0);
    let today = now.with_timezone(&tz).date_naive();
    let day = |field: &str| {
        fc.daily_times(field)
            .into_iter()
            .find(|(d, _)| *d == today)
            .and_then(|(_, t)| t)
    };
    WeatherView {
        fetched_at: s.fetched_at,
        current: s.current.clone(),
        hourly: truncate(&s.hourly, start, 48),
        daily: truncate(&s.daily, 0, 7),
        sunrise_today: day("sunrise"),
        sunset_today: day("sunset"),
    }
}

pub(crate) fn job_err(e: WeatherError) -> JobError {
    match e {
        WeatherError::Store(e) => JobError::Store(e),
        other => JobError::Handler(other.to_string()),
    }
}
