use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use fa_llm::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, PartialEq, schemars::JsonSchema)]
struct Fixture {
    title: String,
    n: i32,
}

type Seen = Arc<Mutex<Vec<(String, Value, bool)>>>;

async fn serve(app: Router) -> url::Url {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    url::Url::parse(&format!("http://{addr}")).unwrap()
}

fn cfg(base: url::Url, dialect: Dialect, key: Option<&str>) -> EndpointConfig {
    EndpointConfig {
        base_url: base,
        model: "qwen3:8b".into(),
        api_key: key.map(Into::into),
        timeout: Duration::from_secs(5),
        thinking: false,
        dialect,
    }
}

fn client(c: EndpointConfig) -> LlmClient {
    LlmClient::new(c, default_http().unwrap())
}

fn req() -> ExtractRequest {
    ExtractRequest {
        system: "s".into(),
        user: "u".into(),
        schema_name: "fixture",
        max_tokens: 64,
    }
}

fn recorder(
    seen: Seen,
    reply: Value,
) -> impl Fn(State<()>, HeaderMap, Json<Value>) -> std::future::Ready<Json<Value>> + Clone {
    move |_, h: HeaderMap, Json(b): Json<Value>| {
        seen.lock()
            .unwrap()
            .push(("".into(), b, h.contains_key("authorization")));
        std::future::ready(Json(reply.clone()))
    }
}

fn openai_reply(content: &str) -> Value {
    json!({"choices":[{"message":{"content": content}}], "usage":{"prompt_tokens":11,"completion_tokens":7}})
}

#[tokio::test]
async fn openai_chat_extract() {
    let seen: Seen = Default::default();
    let app = Router::new()
        .route(
            "/v1/chat/completions",
            post(recorder(
                seen.clone(),
                openai_reply(r#"{"title":"x","n":3}"#),
            )),
        )
        .with_state(());
    let base = serve(app).await.join("/v1").unwrap();
    for (key, trailing) in [(Some("k"), false), (None, true)] {
        let mut b = base.clone();
        if trailing {
            b.set_path("/v1/");
        }
        let e = client(cfg(b, Dialect::OpenaiChat, key))
            .extract::<Fixture>(&req())
            .await
            .unwrap();
        assert_eq!(
            e.value,
            Fixture {
                title: "x".into(),
                n: 3
            }
        );
        assert_eq!((e.prompt_tokens, e.completion_tokens), (Some(11), Some(7)));
        let s = seen.lock().unwrap();
        let (_, body, auth) = s.last().unwrap();
        assert_eq!(*auth, key.is_some());
        assert_eq!(body["model"], "qwen3:8b");
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["additionalProperties"],
            false
        );
        assert!(
            body["response_format"]["json_schema"]["schema"]
                .get("$schema")
                .is_none()
        );
        assert_eq!(body["temperature"], 0);
        assert_eq!(body["messages"][0]["role"], "system");
    }
}

#[tokio::test]
async fn ollama_native_extract() {
    let seen: Seen = Default::default();
    let reply = json!({"message":{"content":"{\"title\":\"y\",\"n\":1}"},"prompt_eval_count":5,"eval_count":9});
    let app = Router::new()
        .route("/api/chat", post(recorder(seen.clone(), reply)))
        .with_state(());
    let base = serve(app).await;
    let e = client(cfg(base, Dialect::OllamaNative, None))
        .extract::<Fixture>(&req())
        .await
        .unwrap();
    assert_eq!(e.value.title, "y");
    assert_eq!((e.prompt_tokens, e.completion_tokens), (Some(5), Some(9)));
    let s = seen.lock().unwrap();
    let (_, body, auth) = &s[0];
    assert!(!auth);
    assert_eq!(body["format"]["type"], "object");
    assert_eq!(body["stream"], false);
    assert_eq!(body["options"]["temperature"], 0);
    assert_eq!(body["options"]["num_predict"], 64);
}

#[tokio::test]
async fn bad_json_and_schema() {
    let mk = |c: &'static str| async move {
        let app = Router::new().route(
            "/chat/completions",
            post(move || async move { Json(openai_reply(c)) }),
        );
        let base = serve(app).await;
        client(cfg(base, Dialect::OpenaiChat, None))
            .extract::<Fixture>(&req())
            .await
            .unwrap_err()
    };
    assert_eq!(mk("not json").await.kind(), "bad_json");
    assert_eq!(mk(r#"{"title":1}"#).await.kind(), "schema");
}

#[tokio::test]
async fn http_500_timeout_unavailable_redirect() {
    let app = Router::new()
        .route(
            "/chat/completions",
            post(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        )
        .route(
            "/api/chat",
            post(|| async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                "x"
            }),
        )
        .route(
            "/models",
            get(|| async { (StatusCode::FOUND, [("location", "/elsewhere")]).into_response() }),
        );
    let base = serve(app).await;

    let e = client(cfg(base.clone(), Dialect::OpenaiChat, None))
        .extract::<Fixture>(&req())
        .await
        .unwrap_err();
    assert!(matches!(e, LlmError::Http(500)));
    assert_eq!(e.kind(), "http");

    let mut c = cfg(base.clone(), Dialect::OllamaNative, None);
    c.timeout = Duration::from_millis(200);
    assert_eq!(
        client(c)
            .extract::<Fixture>(&req())
            .await
            .unwrap_err()
            .kind(),
        "timeout"
    );

    let e = client(cfg(base, Dialect::OpenaiChat, None))
        .models()
        .await
        .unwrap_err();
    assert!(matches!(e, LlmError::Http(302)));

    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    let dead = url::Url::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let e = client(cfg(dead, Dialect::OpenaiChat, None))
        .extract::<Fixture>(&req())
        .await
        .unwrap_err();
    assert_eq!(e.kind(), "unavailable");
}

#[tokio::test]
async fn config_errors() {
    let mut c = cfg(
        "ftp://example.com".parse().unwrap(),
        Dialect::OpenaiChat,
        None,
    );
    assert_eq!(client(c.clone()).ping().await.unwrap_err().kind(), "config");
    c.base_url = "http://127.0.0.1:1".parse().unwrap();
    c.model = " ".into();
    assert_eq!(
        client(c)
            .extract::<Fixture>(&req())
            .await
            .unwrap_err()
            .kind(),
        "config"
    );
}

#[tokio::test]
async fn models_and_ping() {
    let app = Router::new()
        .route(
            "/v1/models",
            get(|| async { Json(json!({"data":[{"id":"qwen3:8b"},{"id":"other"}]})) }),
        )
        .route(
            "/api/tags",
            get(|| async { Json(json!({"models":[{"name":"qwen3:latest"}]})) }),
        );
    let base = serve(app).await;

    let c = client(cfg(base.join("/v1/").unwrap(), Dialect::OpenaiChat, None));
    assert_eq!(c.models().await.unwrap(), vec!["qwen3:8b", "other"]);
    assert!(c.ping().await.unwrap().model_found);

    let mut oc = cfg(base.clone(), Dialect::OllamaNative, None);
    assert!(!client(oc.clone()).ping().await.unwrap().model_found);
    oc.model = "qwen3".into();
    assert!(client(oc).ping().await.unwrap().model_found);

    let mut missing = cfg(base.join("/v1").unwrap(), Dialect::OpenaiChat, None);
    missing.model = "nope".into();
    assert!(!client(missing).ping().await.unwrap().model_found);
}

/// Thinking off asks the model not to reason (both dialects); on keeps its default and adds the
/// extra output budget so the reasoning cannot crowd out the answer.
#[tokio::test]
async fn thinking_switch_shapes_the_request() {
    let seen: Seen = Default::default();
    let app = Router::new()
        .route(
            "/v1/chat/completions",
            post(recorder(
                seen.clone(),
                openai_reply(r#"{"title":"x","n":3}"#),
            )),
        )
        .route(
            "/api/chat",
            post(recorder(
                seen.clone(),
                json!({"message":{"content":"{\"title\":\"y\",\"n\":1}"}}),
            )),
        )
        .with_state(());
    let base = serve(app).await;
    for thinking in [false, true] {
        let mut c = cfg(base.join("/v1").unwrap(), Dialect::OpenaiChat, None);
        c.thinking = thinking;
        client(c).extract::<Fixture>(&req()).await.unwrap();
        let mut c = cfg(base.clone(), Dialect::OllamaNative, None);
        c.thinking = thinking;
        client(c).extract::<Fixture>(&req()).await.unwrap();
    }
    let s = seen.lock().unwrap();
    let budget = 64 + fa_llm::THINKING_EXTRA_TOKENS;
    // Off: OpenAI-style, then Ollama.
    assert_eq!(s[0].1["reasoning_effort"], "none");
    assert_eq!(s[0].1["max_tokens"], 64);
    assert_eq!(s[1].1["think"], false);
    assert_eq!(s[1].1["options"]["num_predict"], 64);
    // On: no switch sent, a larger budget.
    assert!(s[2].1.get("reasoning_effort").is_none());
    assert_eq!(s[2].1["max_tokens"], budget);
    assert!(s[3].1.get("think").is_none());
    assert_eq!(s[3].1["options"]["num_predict"], budget);
}
