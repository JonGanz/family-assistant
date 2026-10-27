use std::sync::Arc;

use fa_connectors::Http;
use fa_domain::HouseholdId;
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};

use crate::WeatherError;
use crate::service::{WeatherConfig, WeatherService, job_err};

/// `(kind, cron)` for `fa-server` to ensure.
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("weather.fetch", "*/30 * * * *"),
        ("weather.retention", "45 3 * * *"),
    ]
}

pub fn handlers(config: WeatherConfig) -> Result<Vec<Arc<dyn JobHandler>>, WeatherError> {
    let http = Http::with_policy(config.allow_http, config.allow_private_hosts)?;
    let shared = Shared { http, config };
    Ok(vec![
        Arc::new(Fetch(shared.clone())),
        Arc::new(Evaluate(shared.clone())),
        Arc::new(Retention(shared)),
    ])
}

#[derive(Clone)]
struct Shared {
    http: Http,
    config: WeatherConfig,
}

impl Shared {
    fn service(&self, ctx: &JobContext) -> WeatherService {
        WeatherService::with_http(ctx.store.clone(), self.http.clone(), self.config.clone())
    }
}

fn household(payload: &serde_json::Value) -> Result<HouseholdId, JobError> {
    payload
        .get("household")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| JobError::Handler("payload needs a `household` id".into()))
}

struct Fetch(Shared);
struct Evaluate(Shared);
struct Retention(Shared);

impl JobHandler for Fetch {
    fn kind(&self) -> &'static str {
        "weather.fetch"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let svc = self.0.service(ctx);
            let mut first_err = None;
            for id in svc.households_with_location().await.map_err(job_err)? {
                if let Err(e) = svc.fetch(id).await {
                    tracing::warn!(error = %e, "weather fetch failed");
                    first_err.get_or_insert(e);
                }
            }
            first_err.map_or(Ok(()), |e| Err(job_err(e)))
        })
    }
}

impl JobHandler for Evaluate {
    fn kind(&self) -> &'static str {
        "weather.evaluate"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let id = household(&payload)?;
            self.0.service(ctx).evaluate(id).await.map_err(job_err)
        })
    }
}

impl JobHandler for Retention {
    fn kind(&self) -> &'static str {
        "weather.retention"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let svc = self.0.service(ctx);
            let mut mine = ctx.store.tx_system().await?;
            let all = fa_store::HouseholdRepo::list(&mut mine).await?;
            mine.commit().await?;
            for h in all {
                let (snaps, advisories) = svc.retention(h.value.id).await.map_err(job_err)?;
                tracing::info!(snaps, advisories, "weather retention");
            }
            Ok(())
        })
    }
}
