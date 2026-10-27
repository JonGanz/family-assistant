//! Starts the real `fa-server serve all` against a scratch database and checks the in-process
//! restart on a runtime-setting change (T-07). Needs `DATABASE_URL` pointing at a Postgres
//! server where the role may create databases.

use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection};

const PW: &str = "adult password 123";
const RESTART_LOG: &str = "settings changed, restarting";

struct Server {
    child: Child,
    base: String,
    log: std::path::PathBuf,
    admin_url: String,
    db_name: String,
    _dir: tempfile::TempDir,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn restarts(&self) -> usize {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .matches(RESTART_LOG)
            .count()
    }
}

async fn start() -> Server {
    let admin_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let db_name = format!("fa_t07_{}", uuid::Uuid::new_v4().simple());
    let mut admin = PgConnection::connect_with(&PgConnectOptions::from_str(&admin_url).unwrap())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {db_name}"))
        .execute(&mut admin)
        .await
        .unwrap();
    let (prefix, _) = admin_url
        .split('?')
        .next()
        .unwrap()
        .rsplit_once('/')
        .unwrap();
    let db_url = format!("{prefix}/{db_name}");

    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("master_key");
    std::fs::write(&key, [7u8; 32]).unwrap();
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let log = dir.path().join("server.log");
    let child = Command::new(env!("CARGO_BIN_EXE_fa-server"))
        .arg("serve")
        .arg("all")
        .current_dir(dir.path())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("DATABASE_URL", &db_url)
        .env("FA_BIND_ADDR", format!("127.0.0.1:{port}"))
        .env("FA_MASTER_KEY_PATH", &key)
        .env("FA_LOG_FORMAT", "json")
        .env("FA_WEB_DIST_DIR", dir.path().join("none"))
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&log).unwrap())
        .spawn()
        .unwrap();
    let s = Server {
        child,
        base: format!("http://127.0.0.1:{port}"),
        log,
        admin_url,
        db_name,
        _dir: dir,
    };
    wait_ready(&s.base).await;
    s
}

async fn wait_ready(base: &str) {
    let http = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(r) = http.get(format!("{base}/readyz")).send().await
            && r.status().is_success()
        {
            return;
        }
        assert!(Instant::now() < deadline, "server not ready");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn drop_db(s: &Server) {
    let mut admin = PgConnection::connect_with(&PgConnectOptions::from_str(&s.admin_url).unwrap())
        .await
        .unwrap();
    let _ = sqlx::query(&format!(
        "DROP DATABASE IF EXISTS {} WITH (FORCE)",
        s.db_name
    ))
    .execute(&mut admin)
    .await;
}

async fn send(
    http: &reqwest::Client,
    base: &str,
    method: reqwest::Method,
    path: &str,
    cookie: &str,
    body: Option<Value>,
) -> reqwest::Response {
    let mut r = http
        .request(method, format!("{base}{path}"))
        .header("X-FA-Request", "1")
        .header("Cookie", cookie);
    if let Some(b) = body {
        r = r.json(&b);
    }
    r.send().await.unwrap()
}

async fn setup(http: &reqwest::Client, base: &str) -> String {
    let r = send(
        http,
        base,
        reqwest::Method::POST,
        "/api/v1/setup/household",
        "",
        Some(
            json!({"name": "Example", "tz": "UTC", "location": null, "units": "metric",
                    "currency": "USD", "base_url": "http://fa.example.com"}),
        ),
    )
    .await;
    assert_eq!(r.status(), 201);
    let r = send(
        http,
        base,
        reqwest::Method::POST,
        "/api/v1/setup/owner",
        "",
        Some(
            json!({"display_name": "Alex", "color": "#3366cc", "username": "alex",
                    "password": PW, "pin": "1234"}),
        ),
    )
    .await;
    assert_eq!(r.status(), 201);
    let set = r
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    set.split(';').next().unwrap().to_owned()
}

async fn setting(http: &reqwest::Client, base: &str, cookie: &str, key: &str) -> Option<Value> {
    let r = send(
        http,
        base,
        reqwest::Method::GET,
        "/api/v1/system/settings",
        cookie,
        None,
    )
    .await;
    let j: Value = r.json().await.ok()?;
    j["settings"]
        .as_array()?
        .iter()
        .find(|s| s["key"] == key)
        .cloned()
}

#[tokio::test]
async fn runtime_setting_change_restarts_and_backup_confirm_does_not() {
    let s = start().await;
    let http = reqwest::Client::new();
    let cookie = setup(&http, &s.base).await;

    let before = setting(&http, &s.base, &cookie, "allow_private_hosts")
        .await
        .unwrap();
    assert_eq!(before["source"], "default");

    let r = send(
        &http,
        &s.base,
        reqwest::Method::PATCH,
        "/api/v1/system/settings",
        &cookie,
        Some(json!({"values": {"allow_private_hosts": true}})),
    )
    .await;
    assert_eq!(r.status(), 200, "{}", r.text().await.unwrap());

    // Restart after the 500 ms debounce, then the api comes back.
    let deadline = Instant::now() + Duration::from_secs(30);
    while s.restarts() == 0 {
        assert!(Instant::now() < deadline, "no restart");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Let any restart caused by the setup's own stored URL settle.
    tokio::time::sleep(Duration::from_millis(2000)).await;
    wait_ready(&s.base).await;
    let v = setting(&http, &s.base, &cookie, "allow_private_hosts")
        .await
        .unwrap();
    assert_eq!(
        (v["source"].clone(), v["value"].clone()),
        (json!("db"), json!(true))
    );
    // Setup stored public_base_url too; any restart for it is over by now.
    let n = s.restarts();
    assert!(n >= 1);

    // Confirming the key backup writes a non-runtime key and must not restart.
    let r = send(
        &http,
        &s.base,
        reqwest::Method::POST,
        "/api/v1/system/master-key/confirm",
        &cookie,
        None,
    )
    .await;
    assert_eq!(r.status(), 204);
    tokio::time::sleep(Duration::from_millis(2000)).await;
    assert_eq!(s.restarts(), n);
    wait_ready(&s.base).await;

    drop_db(&s).await;
}
