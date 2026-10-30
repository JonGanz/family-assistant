use chrono::{TimeZone, Utc};
use fa_connectors::{
    ConnectorError, EventTime, Http, Window,
    caldav::CaldavClient,
    google::{
        calendar::{self, EventQuery},
        oauth::GoogleOAuth,
    },
    ics::{self, IcsFetch},
    openmeteo,
};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path, query_param, query_param_is_missing},
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn http() -> Http {
    Http::new(true).unwrap()
}

fn window() -> Window {
    Window {
        from: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
        to: Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap(),
    }
}

// ---- http ----

#[tokio::test]
async fn http_rejected_unless_allowed() {
    let strict = Http::new(false).unwrap();
    let err = ics::fetch(&strict, "http://example.com/a.ics", None, None)
        .await
        .unwrap_err();
    assert_eq!(err, ConnectorError::InsecureUrl);
}

#[tokio::test]
async fn body_cap_enforced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(5000)))
        .mount(&server)
        .await;
    let small = Http::with_max_body(true, 1000).unwrap();
    let err = ics::fetch(&small, &server.uri(), None, None)
        .await
        .unwrap_err();
    assert_eq!(err, ConnectorError::TooLarge);
}

// ---- ics ----

#[tokio::test]
async fn ics_fetch_and_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .insert_header("Last-Modified", "Sat, 24 Oct 2026 10:00:00 GMT")
                .set_body_string(fixture("sample.ics")),
        )
        .mount(&server)
        .await;
    let IcsFetch::Fetched(f) = ics::fetch(&http(), &server.uri(), None, None)
        .await
        .unwrap()
    else {
        panic!("expected body");
    };
    assert_eq!(f.etag.as_deref(), Some("\"v1\""));
    assert!(f.last_modified.is_some());
    let again = ics::fetch(&http(), &server.uri(), f.etag.as_deref(), None)
        .await
        .unwrap();
    assert_eq!(again, IcsFetch::NotModified);
}

#[test]
fn webcal_normalised() {
    assert_eq!(
        ics::normalize_url("webcal://example.com/x.ics"),
        "https://example.com/x.ics"
    );
    assert_eq!(
        ics::normalize_url("https://example.com/x.ics"),
        "https://example.com/x.ics"
    );
}

#[test]
fn ics_parse_keeps_raw_fields() {
    let comps = ics::parse(&fixture("sample.ics")).unwrap();
    assert_eq!(comps.len(), 3);
    let weekly = &comps[0];
    assert_eq!(weekly.uid.as_deref(), Some("weekly-1@example.com"));
    assert_eq!(
        weekly.rrule.as_deref(),
        Some("FREQ=WEEKLY;BYDAY=MO;COUNT=10")
    );
    let start = weekly.dtstart.as_ref().unwrap();
    assert_eq!(start.value, "20261026T170000");
    assert_eq!(start.tzid.as_deref(), Some("Europe/Berlin"));
    assert!(!start.is_date);
    assert_eq!(weekly.exdate.len(), 2);
    assert_eq!(
        weekly.summary.as_deref(),
        Some("Football practice, pitch 2")
    );
    assert_eq!(weekly.class.as_deref(), Some("PRIVATE"));
    assert_eq!(weekly.status.as_deref(), Some("CONFIRMED"));
    assert_eq!(weekly.url.as_deref(), Some("https://example.com/practice"));
    assert!(
        weekly
            .description
            .as_deref()
            .unwrap()
            .contains("multiple physical lines")
    );

    let moved = &comps[1];
    assert_eq!(
        moved.recurrence_id.as_ref().unwrap().value,
        "20261116T170000"
    );
    assert_eq!(moved.duration.as_deref(), Some("PT1H"));

    let allday = &comps[2];
    assert!(allday.dtstart.as_ref().unwrap().is_date);
    assert_eq!(allday.rdate.len(), 1);
    assert!(allday.rdate[0].is_date);
}

// ---- google ----

fn oauth(server: &MockServer) -> GoogleOAuth {
    let mut o = GoogleOAuth::new("cid", "secret");
    o.token_url = format!("{}/token", server.uri());
    o
}

#[test]
fn authorize_url_has_pkce_and_offline() {
    let o = GoogleOAuth::new("cid", "secret");
    let url = url::Url::parse(
        &o.authorize_url(
            "https://x.example.com/cb",
            &["scope.a", "scope.b"],
            "st",
            "chal",
        )
        .unwrap(),
    )
    .unwrap();
    let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["code_challenge"], "chal");
    assert_eq!(q["access_type"], "offline");
    assert_eq!(q["prompt"], "consent");
    assert_eq!(q["scope"], "scope.a scope.b");
    assert_eq!(q["state"], "st");
    assert_eq!(url.host_str(), Some("accounts.google.com"));
}

#[tokio::test]
async fn oauth_exchange_and_refresh() {
    let server = MockServer::start().await;
    // payload: {"email":"alex@example.com"}
    let id_token = "e30.eyJlbWFpbCI6ImFsZXhAZXhhbXBsZS5jb20ifQ.sig";
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code_verifier=ver"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at", "refresh_token": "rt", "expires_in": 3599,
            "scope": "a b", "id_token": id_token
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at2", "expires_in": 3599, "scope": "a b"
        })))
        .mount(&server)
        .await;
    let o = oauth(&server);
    let t = o
        .exchange_code(&http(), "code", "https://x/cb", "ver")
        .await
        .unwrap();
    assert_eq!(t.access, "at");
    assert_eq!(t.refresh.as_deref(), Some("rt"));
    assert_eq!(t.scopes, vec!["a", "b"]);
    assert_eq!(t.id_token_email.as_deref(), Some("alex@example.com"));
    assert!(t.expires_at > Utc::now());
    let r = o.refresh(&http(), "rt").await.unwrap();
    assert_eq!(r.access, "at2");
    assert!(r.refresh.is_none());
}

#[tokio::test]
async fn oauth_invalid_grant_is_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "invalid_grant"})))
        .mount(&server)
        .await;
    let err = oauth(&server).refresh(&http(), "rt").await.unwrap_err();
    assert_eq!(err, ConnectorError::Unauthorized);
}

#[tokio::test]
async fn google_calendar_list_paginates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(query_param_is_missing("pageToken"))
        .and(header("authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"id": "a@example.com", "summary": "Alex"}],
            "nextPageToken": "p2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(query_param("pageToken", "p2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"id": "fam@group.example.com", "summary": "Family", "summaryOverride": "Our family"}]
        })))
        .mount(&server)
        .await;
    let cals = calendar::list_calendars(&http(), &server.uri(), "tok")
        .await
        .unwrap();
    assert_eq!(cals.len(), 2);
    assert_eq!(cals[1].name, "Our family");
}

#[tokio::test]
async fn google_events_window_pagination_and_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/a@example.com/events"))
        .and(query_param("singleEvents", "true"))
        .and(query_param("timeMin", "2026-10-01T00:00:00Z"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
              {"id": "e1", "status": "confirmed", "summary": "Lunch", "location": "Cafe",
               "htmlLink": "https://calendar.example.com/e1", "etag": "\"1\"",
               "visibility": "private",
               "start": {"dateTime": "2026-10-25T12:00:00+02:00"}, "end": {"dateTime": "2026-10-25T13:00:00+02:00"}},
              {"id": "e2", "summary": "Holiday", "start": {"date": "2026-12-24"}, "end": {"date": "2026-12-27"}}
            ],
            "nextPageToken": "n2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(query_param("pageToken", "n2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [
              {"id": "r1_20261026T150000Z", "recurringEventId": "r1", "summary": "Practice",
               "originalStartTime": {"dateTime": "2026-10-26T17:00:00+02:00"},
               "start": {"dateTime": "2026-10-26T17:30:00+02:00"}, "end": {"dateTime": "2026-10-26T18:30:00+02:00"}}
            ],
            "nextSyncToken": "sync1"
        })))
        .mount(&server)
        .await;
    let page = calendar::list_events(
        &http(),
        &server.uri(),
        "tok",
        "a@example.com",
        &EventQuery::Window(window()),
    )
    .await
    .unwrap();
    assert_eq!(page.next_sync_token.as_deref(), Some("sync1"));
    assert_eq!(page.items.len(), 3);
    let e1 = &page.items[0];
    assert!(e1.private && !e1.cancelled);
    assert_eq!(
        e1.start,
        EventTime::DateTime(Utc.with_ymd_and_hms(2026, 10, 25, 10, 0, 0).unwrap())
    );
    assert_eq!(
        e1.external_url.as_deref(),
        Some("https://calendar.example.com/e1")
    );
    assert_eq!(e1.instance_key, "");
    assert!(matches!(page.items[1].start, EventTime::Date(_)));
    let r = &page.items[2];
    assert_eq!(r.uid, "r1");
    assert_eq!(r.instance_key, "2026-10-26T15:00:00Z");
}

#[tokio::test]
async fn google_incremental_sync_cancelled_and_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/ok/events"))
        .and(query_param("syncToken", "s0"))
        .and(query_param_is_missing("timeMin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [{"id": "gone1", "status": "cancelled"}],
            "nextSyncToken": "s1"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/stale/events"))
        .respond_with(ResponseTemplate::new(410))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/denied/events"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let q = EventQuery::SyncToken("s0".into());
    let page = calendar::list_events(&http(), &server.uri(), "t", "ok", &q)
        .await
        .unwrap();
    assert!(page.items[0].cancelled);
    assert_eq!(page.items[0].uid, "gone1");
    let err = calendar::list_events(&http(), &server.uri(), "t", "stale", &q)
        .await
        .unwrap_err();
    assert_eq!(err, ConnectorError::Gone);
    let err = calendar::list_events(&http(), &server.uri(), "t", "denied", &q)
        .await
        .unwrap_err();
    assert_eq!(err, ConnectorError::Unauthorized);
}

// ---- caldav ----

async fn caldav_server() -> MockServer {
    let server = MockServer::start().await;
    let dav = |body: &'static str, file: &'static str| {
        let _ = body;
        ResponseTemplate::new(207)
            .insert_header("Content-Type", "application/xml; charset=utf-8")
            .set_body_string(fixture(file))
    };
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/"))
        .and(body_string_contains("current-user-principal"))
        .and(header("authorization", "Basic YWxleDpwdw=="))
        .respond_with(dav("", "caldav_principal.xml"))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/alex/"))
        .and(body_string_contains("calendar-home-set"))
        .respond_with(dav("", "caldav_home.xml"))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/alex/"))
        .and(body_string_contains("resourcetype"))
        .and(header("depth", "1"))
        .respond_with(dav("", "caldav_collections.xml"))
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path("/alex/family/"))
        .and(body_string_contains(
            "time-range start=\"20261001T000000Z\" end=\"20261201T000000Z\"",
        ))
        .respond_with(dav("", "caldav_report.xml"))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/alex/family/"))
        .and(body_string_contains("getctag"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<?xml version="1.0"?><multistatus xmlns="DAV:" xmlns:CS="http://calendarserver.org/ns/"><response><href>/alex/family/</href><propstat><prop><CS:getctag>"ctag-43"</CS:getctag></prop><status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#,
        ))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn caldav_discover_fetch_ctag() {
    let server = caldav_server().await;
    let client = CaldavClient::new(http(), &server.uri(), "alex", "pw").unwrap();
    let cals = client.discover().await.unwrap();
    assert_eq!(cals.len(), 2);
    assert_eq!(cals[0].remote_id, "/alex/family/");
    assert_eq!(cals[0].name, "Family & Friends");
    assert_eq!(cals[0].ctag.as_deref(), Some("\"ctag-42\""));
    assert_eq!(cals[1].name, "Chores");

    let blobs = client.fetch(&cals[0], &window()).await.unwrap();
    assert_eq!(blobs.len(), 2);
    let parsed: Vec<_> = blobs.iter().flat_map(|b| ics::parse(b).unwrap()).collect();
    assert_eq!(parsed[0].summary.as_deref(), Some("Dentist & checkup"));
    assert_eq!(
        parsed[0].dtstart.as_ref().unwrap().tzid.as_deref(),
        Some("Europe/Berlin")
    );
    assert!(parsed[1].dtstart.as_ref().unwrap().is_date);

    assert_eq!(
        client.ctag(&cals[0]).await.unwrap().as_deref(),
        Some("\"ctag-43\"")
    );
}

#[tokio::test]
async fn caldav_bad_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let client = CaldavClient::new(http(), &server.uri(), "alex", "bad").unwrap();
    assert_eq!(
        client.discover().await.unwrap_err(),
        ConnectorError::Unauthorized
    );
}

#[test]
fn caldav_client_rejects_http_when_strict() {
    let err = CaldavClient::new(Http::new(false).unwrap(), "http://example.com", "a", "b").err();
    assert_eq!(err, Some(ConnectorError::InsecureUrl));
}

// ---- open-meteo ----

#[tokio::test]
async fn openmeteo_forecast_parses_fixture() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/forecast"))
        .and(query_param("latitude", "52.52"))
        .and(query_param("timezone", "Europe/Berlin"))
        .and(query_param("forecast_hours", "48"))
        .and(query_param("forecast_days", "7"))
        .and(query_param("hourly", openmeteo::HOURLY))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("openmeteo.json")))
        .mount(&server)
        .await;
    let f = openmeteo::forecast(&http(), &server.uri(), 52.52, 13.42, "Europe/Berlin")
        .await
        .unwrap();
    assert_eq!(f.current_f64("temperature_2m"), Some(12.4));
    assert!(f.hourly.get("uv_index").is_some());
    let temps = f.hourly_series("temperature_2m");
    assert_eq!(temps.len(), 3);
    // 14:00 Berlin (CEST, +2) = 12:00 UTC
    assert_eq!(
        temps[0].0,
        Utc.with_ymd_and_hms(2026, 10, 24, 12, 0, 0).unwrap()
    );
    assert_eq!(temps[0].1, Some(12.4));
    assert_eq!(temps[2].1, None);
    assert_eq!(f.daily_series("precipitation_sum").len(), 2);
    let sunrise = f.daily_times("sunrise");
    assert_eq!(
        sunrise[0].1,
        Some(Utc.with_ymd_and_hms(2026, 10, 24, 5, 18, 0).unwrap())
    );
    assert!(f.hourly_series("nope").is_empty());
}
