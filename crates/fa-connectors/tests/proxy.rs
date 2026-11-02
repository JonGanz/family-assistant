//! The strict client must ignore `HTTP(S)_PROXY`. Environment variables are process-wide and
//! `unsafe` to set, so the parent test starts the "proxy" and re-runs this binary as a child
//! with the variables in its environment.
use fa_connectors::Http;
use reqwest::Url;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::any};

fn target() -> Url {
    Url::parse("http://proxy-test.invalid/x").unwrap()
}

/// Runs only as the child of `strict_client_ignores_proxy_variables`.
#[tokio::test]
#[ignore = "child process of strict_client_ignores_proxy_variables"]
async fn child() {
    let open = Http::permissive(true).unwrap();
    let resp = open.send(open.get(&target()).unwrap()).await.unwrap();
    assert!(
        resp.status().is_success(),
        "permissive client uses the proxy"
    );
    let strict = Http::with_policy(true, false).unwrap();
    assert!(strict.send(strict.get(&target()).unwrap()).await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn strict_client_ignores_proxy_variables() {
    let proxy = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .mount(&proxy)
        .await;
    let uri = proxy.uri();
    let status = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args(["--ignored", "--exact", "child"])
            .env_remove("NO_PROXY")
            .env_remove("no_proxy");
        for k in ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"] {
            cmd.env(k, &uri);
        }
        cmd.output().unwrap()
    })
    .await
    .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stdout)
    );
    // Only the permissive client's request reached the proxy.
    assert_eq!(proxy.received_requests().await.unwrap().len(), 1);
}
