use chrono::{TimeZone, Utc};
use fa_connectors::{
    ConnectorError, EventWrite, Http, caldav::CaldavClient, google::calendar, ics,
};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, body_string_contains, header, method, path},
};

fn http() -> Http {
    Http::new(true).unwrap()
}

fn timed() -> EventWrite {
    EventWrite {
        summary: "Concert".into(),
        description: Some("Bring snacks".into()),
        location: Some("School hall".into()),
        start: Utc.with_ymd_and_hms(2026, 11, 3, 18, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 11, 3, 19, 30, 0).unwrap(),
        all_day: false,
        tz: "Europe/Berlin".into(),
    }
}

fn all_day() -> EventWrite {
    EventWrite {
        summary: "Trip".into(),
        description: None,
        location: None,
        // Local midnight 4 Nov in Berlin (UTC+1) .. same day: end is forced to start + 1 day.
        start: Utc.with_ymd_and_hms(2026, 11, 3, 23, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 11, 3, 23, 0, 0).unwrap(),
        all_day: true,
        tz: "Europe/Berlin".into(),
    }
}

#[tokio::test]
async fn google_insert_timed() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/cal@example.com/events"))
        .and(header("authorization", "Bearer tok"))
        .and(body_json(json!({
            "summary": "Concert", "description": "Bring snacks", "location": "School hall",
            "start": {"dateTime": "2026-11-03T18:00:00Z", "timeZone": "Europe/Berlin"},
            "end": {"dateTime": "2026-11-03T19:30:00Z", "timeZone": "Europe/Berlin"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ev1", "iCalUID": "ev1@google.com", "etag": "\"e1\"",
            "htmlLink": "https://www.google.com/calendar/event?eid=x"
        })))
        .mount(&s)
        .await;
    let w = calendar::insert_event(&http(), &s.uri(), "tok", "cal@example.com", &timed())
        .await
        .unwrap();
    assert_eq!(w.id, "ev1");
    assert_eq!(w.ical_uid.as_deref(), Some("ev1@google.com"));
    assert_eq!(w.etag.as_deref(), Some("\"e1\""));
    assert!(w.html_link.is_some());
}

#[tokio::test]
async fn google_insert_all_day_and_scope_error() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/c/events"))
        .and(body_json(json!({
            "summary": "Trip", "start": {"date": "2026-11-04"}, "end": {"date": "2026-11-05"}
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id": "ev2", "iCalUID": "u"})),
        )
        .mount(&s)
        .await;
    calendar::insert_event(&http(), &s.uri(), "t", "c", &all_day())
        .await
        .unwrap();

    s.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403).set_body_json(
            json!({"error": {"message": "Request had insufficient authentication scopes."}}),
        ))
        .mount(&s)
        .await;
    let e = calendar::insert_event(&http(), &s.uri(), "t", "c", &timed())
        .await
        .unwrap_err();
    assert_eq!(e, ConnectorError::InsufficientScope);
}

#[tokio::test]
async fn google_patch_and_delete() {
    let s = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/calendar/v3/calendars/c/events/ev1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id": "ev1", "etag": "\"e2\""})),
        )
        .mount(&s)
        .await;
    let w = calendar::patch_event(&http(), &s.uri(), "t", "c", "ev1", &timed())
        .await
        .unwrap();
    assert_eq!(w.etag.as_deref(), Some("\"e2\""));

    Mock::given(method("DELETE"))
        .and(path("/calendar/v3/calendars/c/events/ev1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&s)
        .await;
    calendar::delete_event(&http(), &s.uri(), "t", "c", "ev1")
        .await
        .unwrap();
    for status in [404, 410] {
        s.reset().await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&s)
            .await;
        calendar::delete_event(&http(), &s.uri(), "t", "c", "gone")
            .await
            .unwrap();
    }
    s.reset().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&s)
        .await;
    assert_eq!(
        calendar::delete_event(&http(), &s.uri(), "t", "c", "x")
            .await
            .unwrap_err(),
        ConnectorError::Retryable(500)
    );
}

#[test]
fn vevent_timed_and_all_day() {
    let t = ics::build_vevent("fa-1@family-assistant", &timed()).unwrap();
    assert!(t.contains("BEGIN:VEVENT"));
    assert!(t.contains("UID:fa-1@family-assistant"));
    assert!(t.contains("SUMMARY:Concert"));
    assert!(t.contains("DTSTART:20261103T180000Z"));
    assert!(t.contains("DTEND:20261103T193000Z"));
    assert!(t.contains("LOCATION:School hall"));
    let parsed = ics::parse(&t).unwrap();
    assert_eq!(parsed[0].uid.as_deref(), Some("fa-1@family-assistant"));

    let a = ics::build_vevent("fa-2@family-assistant", &all_day()).unwrap();
    assert!(a.contains("DTSTART;VALUE=DATE:20261104"), "{a}");
    assert!(a.contains("DTEND;VALUE=DATE:20261105"), "{a}");
}

fn client(s: &MockServer) -> CaldavClient {
    CaldavClient::new(http(), &s.uri(), "u", "p").unwrap()
}

#[tokio::test]
async fn caldav_put_and_delete() {
    let s = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/cal/home/fa-1@family-assistant.ics"))
        .and(header("if-none-match", "*"))
        .and(header("content-type", "text/calendar; charset=utf-8"))
        .and(body_string_contains("BEGIN:VCALENDAR"))
        .respond_with(ResponseTemplate::new(201).insert_header("ETag", "\"abc\""))
        .mount(&s)
        .await;
    let ics_text = ics::build_vevent("fa-1@family-assistant", &timed()).unwrap();
    let c = client(&s);
    let etag = c
        .put_event("/cal/home", "fa-1@family-assistant", &ics_text, true, None)
        .await
        .unwrap();
    assert_eq!(etag.as_deref(), Some("\"abc\""));

    Mock::given(method("PUT"))
        .and(path("/cal/home/fa-1@family-assistant.ics"))
        .and(header("if-match", "\"abc\""))
        .respond_with(ResponseTemplate::new(204))
        .mount(&s)
        .await;
    assert_eq!(
        c.put_event(
            "/cal/home/",
            "fa-1@family-assistant",
            &ics_text,
            false,
            Some("\"abc\"")
        )
        .await
        .unwrap(),
        None
    );

    Mock::given(method("DELETE"))
        .and(path("/cal/home/fa-1@family-assistant.ics"))
        .and(header("if-match", "\"abc\""))
        .respond_with(ResponseTemplate::new(204))
        .mount(&s)
        .await;
    c.delete_event("/cal/home/fa-1@family-assistant.ics", Some("\"abc\""))
        .await
        .unwrap();
}

#[tokio::test]
async fn caldav_precondition_and_missing() {
    let s = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(412))
        .mount(&s)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&s)
        .await;
    let c = client(&s);
    assert_eq!(
        c.put_event(
            "/cal/home/",
            "x",
            "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n",
            true,
            None
        )
        .await
        .unwrap_err(),
        ConnectorError::Http(412)
    );
    c.delete_event("/cal/home/x.ics", None).await.unwrap();
}
