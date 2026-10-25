//! Job handlers and cron schedules (plan/M2.md, Jobs).

use std::sync::Arc;

use chrono::NaiveDate;
use fa_domain::{HouseholdId, WeatherRuleId};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};

use crate::error::TaskError;
use crate::service::TaskService;

impl From<TaskError> for JobError {
    fn from(e: TaskError) -> Self {
        match e {
            TaskError::Store(s) => JobError::Store(s),
            other => JobError::Handler(other.to_string()),
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Materialize,
    Tick,
    Digest,
    Routines,
    FromWeather,
}

struct Handler(Kind, Arc<TaskService>);

fn field<T: std::str::FromStr>(p: &serde_json::Value, k: &str) -> Result<T, JobError> {
    p.get(k)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| JobError::Handler(format!("payload lacks a valid `{k}`")))
}

impl JobHandler for Handler {
    fn kind(&self) -> &'static str {
        match self.0 {
            Kind::Materialize => "tasks.materialize",
            Kind::Tick => "tasks.tick",
            Kind::Digest => "tasks.digest",
            Kind::Routines => "routines.materialize",
            Kind::FromWeather => "tasks.from_weather",
        }
    }

    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let svc = &self.1;
            let now = svc.now();
            match self.0 {
                Kind::Materialize => {
                    let n = svc.materialize_all(now).await?;
                    if n > 0 {
                        tracing::info!(created = n, "tasks.materialize");
                    }
                }
                Kind::Tick => svc.tick_all(now).await?,
                Kind::Digest => svc.digest_all(now).await?,
                Kind::Routines => {
                    svc.routines_materialize_all(now).await?;
                }
                Kind::FromWeather => {
                    let h: HouseholdId = field(&payload, "household")?;
                    let r: WeatherRuleId = field(&payload, "rule")?;
                    let d: NaiveDate = field(&payload, "date")?;
                    svc.from_weather(h, r, d).await?;
                }
            }
            Ok(())
        })
    }
}

pub fn handlers(service: Arc<TaskService>) -> Vec<Arc<dyn JobHandler>> {
    [
        Kind::Materialize,
        Kind::Tick,
        Kind::Digest,
        Kind::Routines,
        Kind::FromWeather,
    ]
    .into_iter()
    .map(|k| Arc::new(Handler(k, service.clone())) as Arc<dyn JobHandler>)
    .collect()
}

/// `(job kind, cron)`; `tasks.from_weather` is only ever enqueued.
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("tasks.materialize", "* * * * *"),
        ("tasks.tick", "* * * * *"),
        ("tasks.digest", "*/15 * * * *"),
        ("routines.materialize", "*/15 * * * *"),
    ]
}
