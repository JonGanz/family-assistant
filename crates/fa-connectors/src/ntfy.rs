use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{
    Method,
    header::{AUTHORIZATION, HeaderValue},
};
use serde_json::Value;

use crate::{ConnectorError, Http, error::Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtfyMessage {
    pub title: String,
    pub body: String,
    /// 1..=5; out-of-range values are clamped.
    pub priority: u8,
    pub tags: Vec<String>,
    pub click: Option<String>,
}

/// Publishes one message and returns its ntfy `id`.
///
/// The `Http` policy applies to `base_url`: plain `http://` is rejected unless the client was
/// built with `Http::new(true)`. `FA_ALLOW_HTTP_FEEDS` is not the switch for ntfy; callers (fa-notify)
/// build a dedicated `Http` with `allow_http = true` for ntfy, since a tailnet or localhost ntfy over
/// http is common. Neither the topic nor the token is logged or put into errors.
pub async fn publish(
    http: &Http,
    base_url: &str,
    topic: &str,
    token: Option<&str>,
    msg: &NtfyMessage,
) -> Result<String> {
    let mut url = http.parse_url(base_url)?;
    url.path_segments_mut()
        .map_err(|_| ConnectorError::Parse("bad base url".into()))?
        .pop_if_empty()
        .push(topic);
    let mut rb = http
        .request(Method::POST, &url)?
        .header("Title", header_text(&msg.title)?)
        .header("Priority", msg.priority.clamp(1, 5).to_string())
        .body(msg.body.clone());
    if !msg.tags.is_empty() {
        rb = rb.header("Tags", header_text(&msg.tags.join(","))?);
    }
    if let Some(click) = &msg.click {
        rb = rb.header("Click", header_text(click)?);
    }
    if let Some(token) = token {
        let mut v = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| ConnectorError::Parse("bad token".into()))?;
        v.set_sensitive(true);
        rb = rb.header(AUTHORIZATION, v);
    }
    let resp = http.send(rb).await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => ConnectorError::Unauthorized,
            s => ConnectorError::Http(s),
        });
    }
    let bytes = http.body(resp).await?;
    let json: Value =
        serde_json::from_slice(&bytes).map_err(|e| ConnectorError::Parse(e.to_string()))?;
    json.get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ConnectorError::Parse("missing `id`".into()))
}

/// ASCII passes through; anything else is RFC 2047 base64-encoded (ntfy decodes it).
fn header_text(s: &str) -> Result<HeaderValue> {
    let s = s.replace(['\r', '\n'], " ");
    let v = if s.is_ascii() {
        s
    } else {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(s))
    };
    HeaderValue::from_str(&v).map_err(|_| ConnectorError::Parse("bad header value".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, header_exists, method, path},
    };

    fn msg() -> NtfyMessage {
        NtfyMessage {
            title: "Hello".into(),
            body: "Body text".into(),
            priority: 4,
            tags: vec![],
            click: None,
        }
    }

    fn ok(id: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": id, "event": "message"}))
    }

    #[tokio::test]
    async fn minimal_headers_and_id() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/topic1"))
            .and(header("Title", "Hello"))
            .and(header("Priority", "4"))
            .respond_with(ok("abc"))
            .expect(1)
            .mount(&s)
            .await;
        let http = Http::new(true).unwrap();
        let id = publish(&http, &s.uri(), "topic1", None, &msg())
            .await
            .unwrap();
        assert_eq!(id, "abc");
        let req = &s.received_requests().await.unwrap()[0];
        assert!(req.headers.get("Tags").is_none());
        assert!(req.headers.get("Click").is_none());
        assert!(req.headers.get("Authorization").is_none());
        assert_eq!(req.body, b"Body text");
    }

    #[tokio::test]
    async fn optional_headers_token_and_encoding() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header_exists("Tags"))
            .and(header("Click", "https://example.com/tasks"))
            .and(header("Authorization", "Bearer tk_secret"))
            .and(header_exists("Title"))
            .respond_with(ok("x1"))
            .expect(1)
            .mount(&s)
            .await;
        let m = NtfyMessage {
            title: "Café 🎉".into(),
            tags: vec!["a".into(), "b".into()],
            click: Some("https://example.com/tasks".into()),
            priority: 9,
            ..msg()
        };
        let http = Http::new(true).unwrap();
        publish(&http, &s.uri(), "t", Some("tk_secret"), &m)
            .await
            .unwrap();
        let req = &s.received_requests().await.unwrap()[0];
        let title = req.headers.get("Title").unwrap().to_str().unwrap();
        assert!(title.starts_with("=?UTF-8?B?") && title.ends_with("?="));
        assert_eq!(req.headers.get("Priority").unwrap(), "5");
        assert_eq!(req.headers.get("Tags").unwrap(), "a,b");
    }

    #[tokio::test]
    async fn error_mapping() {
        let s = MockServer::start().await;
        for (st, _) in [(401, 0), (403, 0), (500, 0), (429, 0)] {
            Mock::given(method("POST"))
                .and(path(format!("/t{st}")))
                .respond_with(ResponseTemplate::new(st).set_body_string("nope"))
                .mount(&s)
                .await;
        }
        let http = Http::new(true).unwrap();
        let e = |t: &'static str| {
            let (http, uri) = (http.clone(), s.uri());
            async move { publish(&http, &uri, t, None, &msg()).await.unwrap_err() }
        };
        assert_eq!(e("t401").await, ConnectorError::Unauthorized);
        assert_eq!(e("t403").await, ConnectorError::Unauthorized);
        assert_eq!(e("t500").await, ConnectorError::Http(500));
        assert_eq!(e("t429").await, ConnectorError::Http(429));
    }

    #[tokio::test]
    async fn bad_json_and_http_policy() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&s)
            .await;
        let http = Http::new(true).unwrap();
        assert!(matches!(
            publish(&http, &s.uri(), "t", None, &msg()).await,
            Err(ConnectorError::Parse(_))
        ));
        let strict = Http::new(false).unwrap();
        assert_eq!(
            publish(&strict, &s.uri(), "t", None, &msg())
                .await
                .unwrap_err(),
            ConnectorError::InsecureUrl
        );
    }

    #[tokio::test]
    #[ignore = "needs FA_TEST_NTFY_URL, e.g. http://127.0.0.1:55402"]
    async fn real_ntfy() {
        let Ok(base) = std::env::var("FA_TEST_NTFY_URL") else {
            return;
        };
        let http = Http::new(true).unwrap();
        let topic = format!("fa-test-{}", std::process::id());
        for title in ["Plain title", "Café 🎉"] {
            let m = NtfyMessage {
                title: title.into(),
                tags: vec!["tada".into()],
                ..msg()
            };
            let id = publish(&http, &base, &topic, None, &m).await.unwrap();
            let url = format!("{base}/{topic}/json?poll=1&since=all");
            let text = reqwest::get(url).await.unwrap().text().await.unwrap();
            let found = text.lines().any(|l| {
                let v: Value = serde_json::from_str(l).unwrap();
                v["id"] == id && v["title"] == title
            });
            assert!(found, "title {title} not read back");
        }
    }
}
