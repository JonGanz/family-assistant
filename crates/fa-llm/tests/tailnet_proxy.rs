//! D-127: requests for tailnet hosts go through the configured HTTP proxy, others stay direct.

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Answers every connection with a 200 JSON body and records each request line.
async fn fake(seen: Arc<Mutex<Vec<String>>>) -> url::Url {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = l.accept().await.unwrap();
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                if let Some(line) = head.lines().next() {
                    seen.lock().unwrap().push(line.to_owned());
                }
                let body = r#"{"ok":true}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            });
        }
    });
    url::Url::parse(&format!("http://{addr}")).unwrap()
}

#[tokio::test]
async fn tailnet_hosts_use_the_proxy_and_others_do_not() {
    let via_proxy = Arc::new(Mutex::new(Vec::new()));
    let direct = Arc::new(Mutex::new(Vec::new()));
    let proxy = fake(via_proxy.clone()).await;
    let target = fake(direct.clone()).await;
    let http = fa_llm::http_client(Some(&proxy)).unwrap();

    for url in [
        "http://100.64.0.7:11434/api/tags",
        "http://gpu.example.ts.net:11434/api/tags",
    ] {
        let r = http.get(url).send().await.unwrap();
        assert!(r.status().is_success(), "{url}");
        assert_eq!(r.text().await.unwrap(), r#"{"ok":true}"#);
    }
    assert_eq!(
        *via_proxy.lock().unwrap(),
        [
            "GET http://100.64.0.7:11434/api/tags HTTP/1.1",
            "GET http://gpu.example.ts.net:11434/api/tags HTTP/1.1",
        ]
    );

    let r = http
        .get(target.join("/api/tags").unwrap())
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());
    assert_eq!(
        via_proxy.lock().unwrap().len(),
        2,
        "direct call hit the proxy"
    );
    assert_eq!(*direct.lock().unwrap(), ["GET /api/tags HTTP/1.1"]);
}
