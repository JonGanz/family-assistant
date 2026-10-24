//! fa-api: the axum HTTP API (v1), SSE hub, OpenAPI document and static web serving.

mod error;
mod extract;
mod hub;
mod openapi;
mod routes;
mod schemas;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, post};
use fa_auth::AuthService;
use fa_store::Store;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub use error::ApiError;
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

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub auth: AuthService,
    pub hub: Hub,
    pub cfg: Arc<ApiConfig>,
    /// Ends SSE streams on graceful shutdown.
    pub shutdown: CancellationToken,
}

impl AppState {
    pub fn new(
        store: Store,
        auth: AuthService,
        cfg: ApiConfig,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            store,
            auth,
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
        .route("/members/{id}", axum::routing::patch(routes::patch_member))
        .route("/members/{id}/account", post(routes::create_account))
        .route(
            "/members/{id}/account/password",
            post(routes::reset_password),
        )
        .route("/members/{id}/pin", axum::routing::put(routes::set_pin))
        .route(
            "/devices",
            get(routes::list_devices).post(routes::pair_device),
        )
        .route("/devices/{id}", delete(routes::revoke_device))
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
    res
}

fn uuid_string() -> String {
    fa_domain::MemberId::new().to_string()
}
