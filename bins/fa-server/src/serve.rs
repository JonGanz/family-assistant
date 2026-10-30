use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use fa_api::{ApiConfig, AppState};
use fa_auth::{AuthConfig, AuthService};
use fa_config::{LogFormat, ProcessConfig, RUNTIME_KEYS, RuntimeSnapshot};
use fa_jobs::{
    BackupVerify, JobContext, JobHandler, Scheduler, SessionCleanup, Worker, WorkerConfig,
};
use fa_store::{InstanceSettingRepo, MasterKey, Store};
use serde_json::Value;
use sqlx::postgres::{PgListener, PgPoolOptions};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::cli::ServeMode;
use crate::wiring;

const DEBOUNCE: Duration = Duration::from_millis(500);

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

fn parse_cidrs(list: &[String], what: &str) -> Result<Vec<fa_api::IpNet>> {
    list.iter()
        .map(|s| s.parse().map_err(anyhow::Error::msg))
        .collect::<Result<_>>()
        .with_context(|| what.to_owned())
}

/// The stored settings with secrets decrypted, as `load_layered` expects them. A secret that
/// cannot be decrypted is skipped (logged by key only).
async fn read_db_settings(store: &Store, key: &MasterKey) -> Result<BTreeMap<String, Value>> {
    let mut tx = store.tx_system().await?;
    let rows = InstanceSettingRepo::all(&mut tx).await?;
    let mut map = BTreeMap::new();
    for row in rows {
        if row.has_secret {
            match InstanceSettingRepo::secret(&mut tx, &row.key, key).await {
                Ok(Some(v)) => {
                    map.insert(row.key, Value::String(v));
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(key = %row.key, error = %e, "cannot decrypt stored setting")
                }
            }
        } else if let Some(v) = row.value {
            map.insert(row.key, v);
        }
    }
    tx.rollback().await?;
    Ok(map)
}

/// The resolved runtime values (secrets included, never logged): two passes with equal
/// fingerprints need no restart.
fn fingerprint(cfg: &ProcessConfig) -> Value {
    let json = serde_json::to_value(cfg).unwrap_or_default();
    Value::Object(
        RUNTIME_KEYS
            .iter()
            .map(|rk| {
                (
                    rk.key.to_owned(),
                    json.get(rk.key).cloned().unwrap_or(Value::Null),
                )
            })
            .collect(),
    )
}

/// The config with stored settings applied, for the operator commands; the environment-only
/// config when the stored settings cannot be read.
pub async fn layered_config(env_cfg: &ProcessConfig, store: &Store) -> ProcessConfig {
    let layered = async {
        let key = wiring::load_key(env_cfg)?;
        Ok::<_, anyhow::Error>(resolve(store, &key).await?.cfg)
    }
    .await;
    layered.unwrap_or_else(|e| {
        tracing::warn!(error = %format!("{e:#}"), "stored settings not applied");
        env_cfg.clone()
    })
}

struct Resolved {
    cfg: ProcessConfig,
    snapshot: RuntimeSnapshot,
    fingerprint: Value,
}

async fn resolve(store: &Store, key: &MasterKey) -> Result<Resolved> {
    let db = read_db_settings(store, key).await?;
    let (cfg, snapshot) = ProcessConfig::load_layered(&db)?;
    Ok(Resolved {
        fingerprint: fingerprint(&cfg),
        cfg,
        snapshot,
    })
}

/// A running set of tasks built from one resolved configuration.
struct Pass {
    cancel: CancellationToken,
    tasks: JoinSet<()>,
}

impl Pass {
    async fn stop(mut self) {
        self.cancel.cancel();
        while self.tasks.join_next().await.is_some() {}
    }
}

/// Builds the services and starts the tasks of one pass. Errors here leave nothing running.
async fn start_pass(
    cfg: &ProcessConfig,
    snapshot: RuntimeSnapshot,
    store: &Store,
    mode: ServeMode,
    parent: &CancellationToken,
) -> Result<Pass> {
    let services = wiring::build_services(cfg, store)?;
    let cancel = parent.child_token();
    let mut pass = Pass {
        cancel: cancel.clone(),
        tasks: JoinSet::new(),
    };
    let run_api = matches!(mode, ServeMode::Api | ServeMode::All);
    let run_worker = matches!(mode, ServeMode::Worker | ServeMode::All);

    if run_api {
        let auth = AuthService::new(
            store.clone(),
            AuthConfig {
                public_base_url: cfg.effective_base_url().to_owned(),
                ..AuthConfig::default()
            },
        );
        let web_dist = cfg.web_dist_dir.is_dir().then(|| cfg.web_dist_dir.clone());
        if web_dist.is_none() {
            tracing::warn!(dir = %cfg.web_dist_dir.display(), "web dist dir not found, static serving disabled");
        }
        let lan_base_url = cfg
            .lan_dashboard_url
            .as_deref()
            .map(|v| v.trim().trim_end_matches('/').to_owned())
            .filter(|v| !v.is_empty());
        let state = AppState::new(
            store.clone(),
            auth,
            services.clone(),
            ApiConfig {
                cookie_secure: cfg.cookie_secure,
                runtime: Arc::new(snapshot),
                master_key_path: cfg.master_key_path.clone(),
                tailnet_base_url: cfg.tailnet_base_url.clone(),
                web_dist_dir: web_dist,
                enable_docs: docs_enabled(),
                trusted_proxies: parse_cidrs(&cfg.trusted_proxies, "trusted_proxies")?,
                lan_base_url: lan_base_url.clone(),
                build: fa_api::BuildInfo {
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                    commit: option_env!("FA_GIT_COMMIT").map(str::to_owned),
                    built_at: option_env!("FA_BUILD_TIME").map(str::to_owned),
                },
                ..ApiConfig::default()
            },
            cancel.clone(),
        );
        // Bind before spawning anything so a failure leaves nothing running.
        let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
            .await
            .with_context(|| format!("binding {}", cfg.bind_addr))?;
        let lan = if lan_base_url.is_some() {
            let allowed = if cfg.lan_allowed_cidrs.is_empty() {
                fa_api::LanConfig::default_allowed()
            } else {
                parse_cidrs(&cfg.lan_allowed_cidrs, "lan_allowed_cidrs")?
            };
            let lan = fa_api::LanConfig {
                allowed,
                allow_pin: cfg.lan_allow_pin,
            };
            let bind = cfg.lan_dashboard_bind.clone();
            let l = tokio::net::TcpListener::bind(&bind)
                .await
                .with_context(|| format!("binding {bind} (FA_LAN_DASHBOARD_BIND)"))?;
            tracing::info!(addr = %bind, pin = lan.allow_pin, "home-network dashboard listening");
            Some((l, lan))
        } else {
            None
        };

        // The worker (possibly another process) announces changes via NOTIFY; this turns them into SSE.
        pass.tasks.spawn(fa_api::run_bridge(
            store.clone(),
            state.hub.clone(),
            cancel.clone(),
        ));
        if let Some((listener, lan)) = lan {
            let app = fa_api::lan_router(state.clone(), lan);
            let c = cancel.clone();
            pass.tasks.spawn(async move {
                if let Err(e) = axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async move { c.cancelled().await })
                .await
                {
                    tracing::error!(error = %e, "home-network dashboard listener failed");
                }
            });
        }
        let app = fa_api::router(state);
        tracing::info!(addr = %cfg.bind_addr, "api listening");
        let c = cancel.clone();
        pass.tasks.spawn(async move {
            if let Err(e) = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown({
                let c = c.clone();
                async move { c.cancelled().await }
            })
            .await
            {
                tracing::error!(error = %e, "api server failed");
            }
            // The api ending on its own ends the process.
            c.cancel();
        });
    }

    if run_worker {
        let mut handlers: Vec<Arc<dyn JobHandler>> =
            vec![Arc::new(SessionCleanup), Arc::new(BackupVerify)];
        handlers.extend(wiring::feature_handlers(cfg, store, &services)?);
        let ctx = JobContext {
            store: store.clone(),
            backup_dir: cfg.backup_dir.clone(),
        };
        let worker = Worker::new(ctx, handlers, WorkerConfig::default());
        let c = cancel.clone();
        pass.tasks.spawn(async move { worker.run(c).await });
        let scheduler = Scheduler::new(store.clone());
        let c = cancel.clone();
        pass.tasks.spawn(async move { scheduler.run(c).await });
        tracing::info!("worker and scheduler started");
    }
    Ok(pass)
}

/// Starts a pass from the stored settings; if that fails (a bad stored value, say), logs the
/// error and starts one from the environment alone rather than crash-loop.
async fn start_resolved(
    resolved: Resolved,
    store: &Store,
    mode: ServeMode,
    parent: &CancellationToken,
) -> Result<Pass> {
    match start_pass(&resolved.cfg, resolved.snapshot, store, mode, parent).await {
        Ok(p) => Ok(p),
        Err(e) => {
            tracing::error!(error = %format!("{e:#}"), "cannot start with the stored settings, falling back to the environment");
            let (cfg, snapshot) = ProcessConfig::load_layered(&BTreeMap::new())?;
            start_pass(&cfg, snapshot, store, mode, parent).await
        }
    }
}

async fn settings_listener(store: &Store) -> Result<PgListener, sqlx::Error> {
    let options = store.pool().connect_options().as_ref().clone();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut l = PgListener::connect_with(&pool).await?;
    l.listen("fa_settings").await?;
    Ok(l)
}

/// Waits for a `fa_settings` notification, then for 500 ms of quiet.
async fn next_settings_change(listener: &mut PgListener) {
    if let Err(e) = listener.recv().await {
        tracing::warn!(error = %e, "settings listener error");
        tokio::time::sleep(Duration::from_secs(1)).await;
        return;
    }
    while let Ok(Ok(_)) = tokio::time::timeout(DEBOUNCE, listener.recv()).await {}
}

pub async fn run(env_cfg: ProcessConfig, store: Store, mode: ServeMode) -> Result<()> {
    // Fail fast on a missing or malformed master key.
    let key = wiring::load_key(&env_cfg)?;

    let cancel = CancellationToken::new();
    spawn_signal_handler(cancel.clone());

    let run_api = matches!(mode, ServeMode::Api | ServeMode::All);
    if run_api {
        store.migrate().await.context("running migrations")?;
    } else {
        wait_for_migrations(&store, &cancel).await?;
    }
    if cancel.is_cancelled() {
        return Ok(());
    }

    wiring::seed_default_rules(&store).await?;
    if matches!(mode, ServeMode::Worker | ServeMode::All) {
        fa_jobs::ensure_default_schedules(&store).await?;
        wiring::ensure_feature_schedules(&store).await?;
    }

    let mut listener = settings_listener(&store)
        .await
        .context("listening for settings changes")?;
    let mut current = resolve(&store, &key).await?;
    loop {
        let running = current.fingerprint.clone();
        let pass = start_resolved(current, &store, mode, &cancel).await?;
        // Wait for a change that matters, a shutdown, or the pass ending on its own.
        let next = loop {
            tokio::select! {
                _ = pass.cancel.cancelled() => break None,
                () = next_settings_change(&mut listener) => {
                    match resolve(&store, &key).await {
                        Ok(r) if r.fingerprint != running => break Some(r),
                        Ok(_) => {}
                        Err(e) => tracing::error!(error = %format!("{e:#}"), "cannot read settings, keeping the running ones"),
                    }
                }
            }
        };
        pass.stop().await;
        match next {
            Some(r) if !cancel.is_cancelled() => {
                tracing::info!("settings changed, restarting");
                current = r;
            }
            _ => break,
        }
    }
    cancel.cancel();
    tracing::info!("shutdown complete");
    Ok(())
}

fn docs_enabled() -> bool {
    cfg!(debug_assertions)
        || std::env::var("FA_API_DOCS").is_ok_and(|v| matches!(v.as_str(), "1" | "true"))
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
