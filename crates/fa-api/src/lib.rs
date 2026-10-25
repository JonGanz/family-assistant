//! fa-api: the axum HTTP API (v1), SSE hub, OpenAPI document and static web serving.

mod bridge;
mod error;
mod extract;
mod feeds;
mod hub;
mod notify;
mod openapi;
mod routes;
mod tasks;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, patch, post, put};
use fa_auth::{AuthService, OAuthService};
use fa_calendar::CalendarService;
use fa_store::Store;
use fa_weather::WeatherService;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub use bridge::{deliver, run_bridge, run_bridge_signalled};
pub use error::{ApiError, Problem};
pub use extract::IpNet;
pub use extract::{Auth, require};
pub use hub::{Hub, HubEvent};
pub use openapi::openapi;

#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// `Secure` attribute of the session cookie.
    pub cookie_secure: bool,
    /// Built web app; `None` disables static serving.
    pub web_dist_dir: Option<PathBuf>,
    /// Serve Swagger UI at `/api/docs` (development only).
    pub enable_docs: bool,
    pub request_timeout: Duration,
    pub heartbeat_interval: Duration,
    /// Peers in these networks are trusted reverse proxies (e.g. `tailscale serve` on the host, seen
    /// from inside Docker as the bridge gateway); for them the client IP is the rightmost
    /// `X-Forwarded-For` entry, which the proxy appended.
    pub trusted_proxies: Vec<IpNet>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            cookie_secure: true,
            web_dist_dir: None,
            enable_docs: false,
            request_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(15),
            trusted_proxies: IpNet::default_trusted(),
        }
    }
}

/// The feature services the M1 routes call, built once by the server from the process config.
#[derive(Clone)]
pub struct Services {
    pub calendar: Arc<CalendarService>,
    pub weather: WeatherService,
    pub oauth: OAuthService,
    pub tasks: Arc<fa_tasks::TaskService>,
    pub notify: fa_notify::NotifyService,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub auth: AuthService,
    pub services: Services,
    pub hub: Hub,
    pub cfg: Arc<ApiConfig>,
    /// Ends SSE streams on graceful shutdown.
    pub shutdown: CancellationToken,
}

impl AppState {
    pub fn new(
        store: Store,
        auth: AuthService,
        services: Services,
        cfg: ApiConfig,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            store,
            auth,
            services,
            hub: Hub::default(),
            cfg: Arc::new(cfg),
            shutdown,
        }
    }
}

pub const CSRF_HEADER: &str = "x-fa-request";
pub const SESSION_COOKIE: &str = "fa_session";

pub fn router(state: AppState) -> Router {
    let v1 = Router::new()
        .route("/auth/login", post(routes::login))
        .route("/auth/logout", post(routes::logout))
        .route("/me", get(routes::me))
        .route(
            "/members",
            get(routes::list_members).post(routes::create_member),
        )
        .route("/members/{id}", patch(routes::patch_member))
        .route(
            "/members/{id}/avatar",
            put(feeds::put_avatar)
                .delete(feeds::delete_avatar)
                .get(feeds::get_avatar)
                .layer(DefaultBodyLimit::max(feeds::AVATAR_MAX_BYTES)),
        )
        .route("/members/{id}/account", post(routes::create_account))
        .route(
            "/members/{id}/account/password",
            post(routes::reset_password),
        )
        .route("/members/{id}/pin", put(routes::set_pin))
        .route(
            "/devices",
            get(routes::list_devices).post(routes::pair_device),
        )
        .route(
            "/devices/{id}",
            get(feeds::get_device)
                .patch(feeds::patch_device)
                .delete(routes::revoke_device),
        )
        .route(
            "/household",
            get(feeds::get_household).patch(feeds::patch_household),
        )
        .route("/oauth/google/start", get(feeds::oauth_start))
        .route("/oauth/google/callback", get(feeds::oauth_callback))
        .route("/oauth/accounts", get(feeds::list_oauth_accounts))
        .route(
            "/calendar-sources",
            get(feeds::list_sources).post(feeds::create_source),
        )
        .route("/calendar-sources/test", post(feeds::test_source))
        .route(
            "/calendar-sources/{id}",
            patch(feeds::patch_source).delete(feeds::delete_source),
        )
        .route("/calendar-sources/{id}/sync", post(feeds::sync_source))
        .route("/calendars", get(feeds::list_calendars))
        .route("/calendars/{id}", patch(feeds::patch_calendar))
        .route("/events", get(feeds::list_events))
        .route("/weather", get(feeds::get_weather))
        .route("/advisories", get(feeds::list_advisories))
        .route("/advisories/{id}/dismiss", post(feeds::dismiss_advisory))
        .route(
            "/weather-rules",
            get(feeds::list_rules).post(feeds::create_rule),
        )
        .route("/weather-rules/validate", post(feeds::validate_rule))
        .route(
            "/weather-rules/{id}",
            patch(feeds::patch_rule).delete(feeds::delete_rule),
        )
        .route("/tasks", get(tasks::list_tasks).post(tasks::create_task))
        .route(
            "/tasks/{id}",
            get(tasks::get_task)
                .patch(tasks::patch_task)
                .delete(tasks::delete_task),
        )
        .route("/tasks/{id}/complete", post(tasks::complete_task))
        .route("/tasks/{id}/reopen", post(tasks::reopen_task))
        .route("/tasks/{id}/skip", post(tasks::skip_task))
        .route("/tasks/{id}/accept", post(tasks::accept_task))
        .route("/tasks/{id}/claim", post(tasks::claim_task))
        .route("/tasks/{id}/release", post(tasks::release_task))
        .route("/tasks/{id}/steps/{step_id}", patch(tasks::tick_task_step))
        .route(
            "/task-lists",
            get(tasks::list_task_lists).post(tasks::create_task_list),
        )
        .route(
            "/task-lists/{id}",
            patch(tasks::patch_task_list).delete(tasks::delete_task_list),
        )
        .route(
            "/task-definitions",
            get(tasks::list_definitions).post(tasks::create_definition),
        )
        .route(
            "/task-definitions/validate",
            post(tasks::validate_definition),
        )
        .route(
            "/task-definitions/{id}",
            patch(tasks::patch_definition).delete(tasks::delete_definition),
        )
        .route(
            "/routines",
            get(tasks::list_routines).post(tasks::create_routine),
        )
        .route("/routines/today", get(tasks::routines_today))
        .route(
            "/routines/{id}",
            patch(tasks::patch_routine).delete(tasks::delete_routine),
        )
        .route(
            "/routine-runs/{id}/steps/{step_id}",
            patch(tasks::tick_routine_step),
        )
        .route(
            "/members/{id}/away",
            get(tasks::list_away).post(tasks::add_away),
        )
        .route("/away/{id}", delete(tasks::delete_away))
        .route(
            "/members/{id}/notify-prefs",
            get(notify::get_prefs).put(notify::put_prefs),
        )
        .route("/members/{id}/notify-prefs/test", post(notify::test_prefs))
        .route("/dashboard", get(feeds::get_dashboard))
        .route("/status", get(feeds::get_status))
        .route("/device/elevate", post(routes::elevate))
        .route("/sessions", get(routes::list_sessions))
        .route("/sessions/{id}", delete(routes::revoke_session))
        .route("/stream", get(routes::stream))
        .route("/openapi.json", get(routes::openapi_json))
        .fallback(|| async { ApiError::NotFound });

    let mut app = Router::new()
        .route("/healthz", get(routes::healthz))
        .route("/readyz", get(routes::readyz))
        .nest("/api/v1", v1)
        .route("/api/{*rest}", any(|| async { ApiError::NotFound }));

    if state.cfg.enable_docs {
        app = app.merge(
            utoipa_swagger_ui::SwaggerUi::new("/api/docs")
                .config(utoipa_swagger_ui::Config::from("/api/v1/openapi.json")),
        );
    }
    if let Some(dir) = &state.cfg.web_dist_dir {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)));
    }

    let timeout =
        TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, state.cfg.request_timeout);
    app.layer(
        ServiceBuilder::new()
            .layer(middleware::from_fn(request_id))
            .layer(
                TraceLayer::new_for_http().make_span_with(|req: &Request| {
                    let id = req
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("-");
                    tracing::info_span!("http", method = %req.method(), path = %req.uri().path(), request_id = %id)
                }),
            )
            .layer(timeout)
            .layer(CompressionLayer::new())
            .layer(middleware::from_fn(csrf)),
    )
    .with_state(state)
}

/// Mutating `/api/` requests must carry `X-FA-Request: 1` (cross-site forms cannot set it).
async fn csrf(req: Request, next: Next) -> Response {
    let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    let ok = req
        .headers()
        .get(CSRF_HEADER)
        .is_some_and(|v| v.as_bytes() == b"1");
    if !safe && !ok && req.uri().path().starts_with("/api/") {
        return ApiError::Forbidden("missing X-FA-Request header".into()).into_response();
    }
    next.run(req).await
}

async fn request_id(mut req: Request, next: Next) -> Response {
    let name = HeaderName::from_static("x-request-id");
    let value = match req.headers().get(&name) {
        Some(v) => v.clone(),
        None => {
            let v = HeaderValue::from_str(&uuid_string()).unwrap_or(HeaderValue::from_static("-"));
            req.headers_mut().insert(name.clone(), v.clone());
            v
        }
    };
    let mut res = next.run(req).await;
    res.headers_mut().insert(name, value);
    // The dashboard pairing URL carries a device token in its query string; never leak URLs.
    res.headers_mut().insert(
        axum::http::header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    res
}

fn uuid_string() -> String {
    fa_domain::MemberId::new().to_string()
}
