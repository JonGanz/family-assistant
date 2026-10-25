use std::sync::Arc;

use chrono::Utc;
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};

use crate::NotifyService;

/// `(kind, cron)` for `fa-server` to ensure.
pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![("notify.send", "* * * * *")]
}

pub fn handlers(service: NotifyService) -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(Send(service))]
}

struct Send(NotifyService);

impl JobHandler for Send {
    fn kind(&self) -> &'static str {
        "notify.send"
    }

    fn run<'a>(
        &'a self,
        _ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            self.0
                .run_send_at(Utc::now())
                .await
                .map_err(|e| JobError::Handler(e.to_string()))
        })
    }
}
