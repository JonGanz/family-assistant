use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use fa_api::{ApiConfig, AppState};
use fa_auth::{AuthConfig, AuthService};
use fa_config::{LogFormat, ProcessConfig};
use fa_jobs::{
    BackupVerify, JobContext, JobHandler, Scheduler, SessionCleanup, Worker, WorkerConfig,
};
use fa_store::Store;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::cli::ServeMode;
use crate::wiring;

pub fn init_logging(cfg: &ProcessConfig) {
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    let b = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);
    match cfg.log_format {
        LogFormat::Json => b.json().init(),
        LogFormat::Pretty => b.init(),
    }
}

/// Cancels the token on SIGTERM or Ctrl-C.
fn spawn_signal_handler(cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut term =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "cannot install SIGTERM handler");
                    return;
                }
            };
        tokio::select! {
            _ = term.recv() => tracing::info!("SIGTERM received, shutting down"),
            _ = tokio::signal::ctrl_c() => tracing::info!("interrupt received, shutting down"),
        }
        cancel.cancel();
    });
}

/// `FA_TRUSTED_PROXIES`: comma-separated CIDRs whose `X-Forwarded-For` is trusted (empty = none).
fn trusted_proxies() -> Result<Vec<fa_api::IpNet>> {
    match std::env::var("FA_TRUSTED_PROXIES") {
        Err(_) => Ok(fa_api::IpNet::default_trusted()),
        Ok(v) => v
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.parse().map_err(anyhow::Error::msg))
            .collect::<Result<_>>()
            .context("FA_TRUSTED_PROXIES"),
    }
}

fn docs_enabled() -> bool {
    cfg!(debug_assertions)
        || std::env::var("FA_API_DOCS").is_ok_and(|v| matches!(v.as_str(), "1" | "true"))
}

pub async fn run(cfg: ProcessConfig, store: Store, mode: ServeMode) -> Result<()> {
    // Fail fast on a missing or malformed master key.
    wiring::load_key(&cfg)?;
    let services = wiring::build_services(&cfg, &store)?;

    let cancel = CancellationToken::new();
    spawn_signal_handler(cancel.clone());

    let run_api = matches!(mode, ServeMode::Api | ServeMode::All);
    let run_worker = matches!(mode, ServeMode::Worker | ServeMode::All);

    if run_api {
        store.migrate().await.context("running migrations")?;
    } else {
        wait_for_migrations(&store, &cancel).await?;
    }

    wiring::seed_default_rules(&store).await?;

    let mut tasks = tokio::task::JoinSet::new();
    if run_worker {
        fa_jobs::ensure_default_schedules(&store).await?;
        wiring::ensure_feature_schedules(&store).await?;
        let mut handlers: Vec<Arc<dyn JobHandler>> =
            vec![Arc::new(SessionCleanup), Arc::new(BackupVerify)];
        handlers.extend(wiring::feature_handlers(&cfg, &store, &services)?);
        let ctx = JobContext {
            store: store.clone(),
            backup_dir: cfg.backup_dir.clone(),
        };
        let worker = Worker::new(ctx, handlers, WorkerConfig::default());
        let c = cancel.clone();
        tasks.spawn(async move { worker.run(c).await });
        let scheduler = Scheduler::new(store.clone());
        let c = cancel.clone();
        tasks.spawn(async move { scheduler.run(c).await });
        tracing::info!("worker and scheduler started");
    }

    if run_api {
        let auth = AuthService::new(
            store.clone(),
            AuthConfig {
                public_base_url: cfg.public_base_url.clone(),
                ..AuthConfig::default()
            },
        );
        let web_dist = cfg.web_dist_dir.is_dir().then(|| cfg.web_dist_dir.clone());
        if web_dist.is_none() {
            tracing::warn!(dir = %cfg.web_dist_dir.display(), "web dist dir not found, static serving disabled");
        }
        let state = AppState::new(
            store.clone(),
            auth,
            services,
            ApiConfig {
                cookie_secure: cfg.cookie_secure,
                web_dist_dir: web_dist,
                enable_docs: docs_enabled(),
                trusted_proxies: trusted_proxies()?,
                ..ApiConfig::default()
            },
            cancel.clone(),
        );
        // The worker (possibly another process) announces changes via NOTIFY; this turns them into SSE.
        tasks.spawn(fa_api::run_bridge(
            store.clone(),
            state.hub.clone(),
            cancel.clone(),
        ));
        let app = fa_api::router(state);
        let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
            .await
            .with_context(|| format!("binding {}", cfg.bind_addr))?;
        tracing::info!(addr = %cfg.bind_addr, "api listening");
        let c = cancel.clone();
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { c.cancelled().await })
        .await
        .context("serving")?;
        cancel.cancel();
    } else {
        cancel.cancelled().await;
    }

    while tasks.join_next().await.is_some() {}
    tracing::info!("shutdown complete");
    Ok(())
}

async fn wait_for_migrations(store: &Store, cancel: &CancellationToken) -> Result<()> {
    while !store.migrations_applied().await.unwrap_or(false) {
        tracing::info!("waiting for migrations to be applied by the api");
        tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
    Ok(())
}
