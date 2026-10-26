//! Builds the feature services from the process config and registers their jobs.

use std::sync::Arc;

use anyhow::{Context, Result};
use fa_api::{MailServices, Services};
use fa_auth::OAuthService;
use fa_calendar::{CalendarService, ConnectorConfig};
use fa_config::ProcessConfig;
use fa_connectors::Http;
use fa_connectors::google::oauth::GoogleOAuth;
use fa_jobs::JobHandler;
use fa_mail::{MailConfig, MailDeps};
use fa_store::{HouseholdRepo, MasterKey, ScheduleRepo, Store};
use fa_weather::{WeatherConfig, WeatherService};

pub fn load_key(cfg: &ProcessConfig) -> Result<MasterKey> {
    MasterKey::from_file(&cfg.master_key_path)
        .with_context(|| format!("master key at {}", cfg.master_key_path.display()))
}

pub fn weather_config(cfg: &ProcessConfig) -> WeatherConfig {
    WeatherConfig {
        open_meteo_url: cfg.open_meteo_url.as_str().to_owned(),
        allow_http: cfg.allow_http_feeds,
    }
}

pub fn notify_config(cfg: &ProcessConfig) -> fa_notify::NotifyConfig {
    fa_notify::NotifyConfig {
        ntfy_url: cfg.ntfy_url.as_ref().map(|u| u.as_str().to_owned()),
        ntfy_token: cfg.ntfy_token.as_ref().map(|t| t.expose().to_owned()),
        public_base_url: cfg.public_base_url.clone(),
    }
}

fn google_client(cfg: &ProcessConfig) -> Option<GoogleOAuth> {
    let id = cfg
        .google_client_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())?;
    let secret = cfg
        .google_client_secret
        .as_ref()
        .map(|s| s.expose())
        .filter(|s| !s.trim().is_empty())?;
    Some(GoogleOAuth::new(id, secret))
}

/// The base of the OAuth redirect: `FA_PUBLIC_URL`, else the public base URL of the app.
fn public_url(cfg: &ProcessConfig) -> String {
    cfg.public_url
        .as_ref()
        .map_or_else(|| cfg.public_base_url.clone(), |u| u.as_str().to_owned())
        .trim_end_matches('/')
        .to_owned()
}

/// `FA_GOOGLE_API_BASE` without a trailing slash, or the default.
fn google_api_base(cfg: &ProcessConfig) -> String {
    cfg.google_api_base.as_ref().map_or_else(
        || fa_connectors::google::DEFAULT_API_BASE.to_owned(),
        |u| u.as_str().trim_end_matches('/').to_owned(),
    )
}

/// Everything the fa-mail services and jobs share. Gmail calls go through `Http`, which only
/// allows plain http when feeds do or when the Google API base is an http stub.
pub fn mail_deps(
    cfg: &ProcessConfig,
    store: &Store,
    oauth: &OAuthService,
) -> Result<Arc<MailDeps>> {
    let stub = cfg
        .google_api_base
        .as_ref()
        .is_some_and(|u| u.scheme() == "http");
    Ok(Arc::new(MailDeps {
        store: store.clone(),
        http: Http::new(cfg.allow_http_feeds || stub)?,
        tokens: Arc::new(oauth.clone()),
        key: Arc::new(load_key(cfg)?),
        cfg: MailConfig {
            gmail_api_base: google_api_base(cfg),
            allow_plain_imap: cfg.allow_plain_imap,
            imap_tls_insecure: cfg.imap_tls_insecure,
        },
    }))
}

pub fn build_services(cfg: &ProcessConfig, store: &Store) -> Result<Services> {
    let oauth = OAuthService::new(
        store.clone(),
        load_key(cfg)?,
        google_client(cfg),
        public_url(cfg),
        Http::new(cfg.allow_http_feeds)?,
    );
    let calendar = CalendarService::new(
        store.clone(),
        load_key(cfg)?,
        ConnectorConfig {
            allow_http: cfg.allow_http_feeds,
            google_api_base: google_api_base(cfg),
        },
        Arc::new(oauth.clone()),
    )?;
    let weather = WeatherService::new(store.clone(), weather_config(cfg))?;
    let mail = MailServices::new(mail_deps(cfg, store, &oauth)?);
    Ok(Services {
        mail,
        calendar: Arc::new(calendar),
        weather,
        oauth,
        tasks: Arc::new(fa_tasks::TaskService::new(store.clone())),
        notify: fa_notify::NotifyService::new(store.clone(), notify_config(cfg)),
    })
}

/// Job handlers of the feature crates (the M0 handlers are added by the caller).
pub fn feature_handlers(
    cfg: &ProcessConfig,
    store: &Store,
    services: &Services,
) -> Result<Vec<Arc<dyn JobHandler>>> {
    let mut all = fa_calendar::handlers(services.calendar.clone());
    all.extend(fa_weather::handlers(weather_config(cfg))?);
    all.extend(fa_auth::handlers());
    all.extend(fa_tasks::handlers(services.tasks.clone()));
    all.extend(fa_notify::handlers(services.notify.clone()));
    all.extend(crate::mail::handlers(&mail_deps(
        cfg,
        store,
        &services.oauth,
    )?));
    Ok(all)
}

/// Idempotently registers every feature crate's default cron schedules.
pub async fn ensure_feature_schedules(store: &Store) -> Result<()> {
    let schedules = fa_calendar::default_schedules()
        .into_iter()
        .chain(fa_weather::default_schedules())
        .chain(fa_auth::default_schedules())
        .chain(fa_tasks::default_schedules())
        .chain(fa_notify::default_schedules())
        .chain(fa_mail::default_schedules());
    let mut tx = store.tx_system().await?;
    for (kind, cron) in schedules {
        ScheduleRepo::upsert(&mut tx, kind, cron, kind, &serde_json::json!({}), true).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Seeds the default weather rules of every household (idempotent).
pub async fn seed_default_rules(store: &Store) -> Result<()> {
    let mut sys = store.tx_system().await?;
    let all = HouseholdRepo::list(&mut sys).await?;
    sys.rollback().await?;
    for h in all {
        let mut tx = store.tx_for(h.value.id).await?;
        let n = fa_weather::ensure_default_rules(&mut tx).await?;
        tx.commit().await?;
        if n > 0 {
            tracing::info!(household = %h.value.id, added = n, "default weather rules added");
        }
    }
    Ok(())
}
