use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use fa_connectors::{ConnectorError, Http, google::gmail};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param},
};

fn http() -> Http {
    Http::new(true).unwrap()
}

#[tokio::test]
async fn profile() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/profile"))
        .and(header("authorization", "Bearer tok"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"emailAddress": "a@example.com", "historyId": "1234"})),
        )
        .mount(&s)
        .await;
    let p = gmail::get_profile(&http(), &s.uri(), "tok").await.unwrap();
    assert_eq!(
        (p.email.as_str(), p.history_id.as_str()),
        ("a@example.com", "1234")
    );
}

#[tokio::test]
async fn list_messages_paginates() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(query_param("q", "newer_than:30d"))
        .and(query_param("pageToken", "p2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"messages": [{"id": "m3", "threadId": "t3"}]})),
        )
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(query_param("q", "newer_than:30d"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "messages": [{"id": "m1", "threadId": "t1"}, {"id": "m2", "threadId": "t2"}],
            "nextPageToken": "p2"
        })))
        .mount(&s)
        .await;
    let (a, next) = gmail::list_messages(&http(), &s.uri(), "tok", "newer_than:30d", None)
        .await
        .unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(next.as_deref(), Some("p2"));
    let (b, next) =
        gmail::list_messages(&http(), &s.uri(), "tok", "newer_than:30d", next.as_deref())
            .await
            .unwrap();
    assert_eq!(b[0].id, "m3");
    assert_eq!(b[0].thread_id, "t3");
    assert!(next.is_none());
}

#[tokio::test]
async fn list_messages_empty() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"resultSizeEstimate": 0})))
        .mount(&s)
        .await;
    let (a, next) = gmail::list_messages(&http(), &s.uri(), "tok", "", None)
        .await
        .unwrap();
    assert!(a.is_empty() && next.is_none());
}

#[tokio::test]
async fn history_pages_and_types() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/history"))
        .and(query_param("startHistoryId", "100"))
        .and(query_param("historyTypes", "labelAdded"))
        .and(query_param("pageToken", "n2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"historyId": "130"})))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/history"))
        .and(query_param("startHistoryId", "100"))
        .and(query_param("historyTypes", "messageAdded"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "history": [
                {"id": "101", "messagesAdded": [{"message": {"id": "m1", "threadId": "t1"}}]},
                {"id": "102", "labelsAdded": [{"message": {"id": "m0", "threadId": "t0"}, "labelIds": ["SPAM"]}]},
                {"id": "103", "labelsRemoved": [{"message": {"id": "m0", "threadId": "t0"}, "labelIds": ["INBOX"]}],
                 "messagesDeleted": [{"message": {"id": "m9", "threadId": "t9"}}]}
            ],
            "nextPageToken": "n2",
            "historyId": "120"
        })))
        .mount(&s)
        .await;
    let a = gmail::list_history(&http(), &s.uri(), "tok", "100", None)
        .await
        .unwrap();
    assert_eq!(a.added.len(), 1);
    assert_eq!(a.labels_added, vec![("m0".into(), vec!["SPAM".into()])]);
    assert_eq!(a.labels_removed, vec![("m0".into(), vec!["INBOX".into()])]);
    assert_eq!(a.deleted, vec!["m9".to_string()]);
    assert_eq!(a.next_page_token.as_deref(), Some("n2"));
    let b = gmail::list_history(&http(), &s.uri(), "tok", "100", Some("n2"))
        .await
        .unwrap();
    assert!(b.added.is_empty() && b.next_page_token.is_none());
    assert_eq!(b.history_id.as_deref(), Some("130"));
}

#[tokio::test]
async fn history_404_is_expired() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/history"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": {"code": 404}})))
        .mount(&s)
        .await;
    let e = gmail::list_history(&http(), &s.uri(), "tok", "1", None)
        .await
        .unwrap_err();
    assert_eq!(e, ConnectorError::HistoryExpired);
}

#[tokio::test]
async fn get_raw_decodes_base64url() {
    let s = MockServer::start().await;
    let raw = b"From: a@example.com\r\nSubject: hi \xfb\xff?>\r\n\r\nbody\r\n";
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/m1"))
        .and(query_param("format", "raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "m1", "threadId": "t1", "labelIds": ["INBOX", "UNREAD"],
            "internalDate": "1792915200000",
            "raw": URL_SAFE_NO_PAD.encode(raw)
        })))
        .mount(&s)
        .await;
    let m = gmail::get_raw(&http(), &s.uri(), "tok", "m1")
        .await
        .unwrap();
    assert_eq!(m.raw, raw);
    assert_eq!(m.thread_id, "t1");
    assert_eq!(m.label_ids, vec!["INBOX", "UNREAD"]);
    assert_eq!(m.internal_date.timestamp(), 1_792_915_200);
}

#[tokio::test]
async fn attachment_and_modify_and_labels() {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/m1/attachments/a1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"size": 3, "data": URL_SAFE_NO_PAD.encode([0xfb, 0xff, 0x00])}),
            ),
        )
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/gmail/v1/users/me/messages/m1/modify"))
        .and(body_json(
            json!({"addLabelIds": ["Label_1"], "removeLabelIds": ["INBOX"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "m1"})))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/labels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"labels": [
            {"id": "INBOX", "name": "INBOX", "type": "system"}
        ]})))
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/gmail/v1/users/me/labels"))
        .and(body_json(json!({
            "name": "FA/Bills", "labelListVisibility": "labelShow", "messageListVisibility": "show"
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id": "Label_1", "name": "FA/Bills"})),
        )
        .mount(&s)
        .await;
    let h = http();
    assert_eq!(
        gmail::get_attachment(&h, &s.uri(), "t", "m1", "a1")
            .await
            .unwrap(),
        vec![0xfb, 0xff, 0x00]
    );
    gmail::modify(
        &h,
        &s.uri(),
        "t",
        "m1",
        &["Label_1".into()],
        &["INBOX".into()],
    )
    .await
    .unwrap();
    assert_eq!(
        gmail::list_labels(&h, &s.uri(), "t").await.unwrap()[0].id,
        "INBOX"
    );
    let l = gmail::create_label(&h, &s.uri(), "t", "FA/Bills")
        .await
        .unwrap();
    assert_eq!((l.id.as_str(), l.name.as_str()), ("Label_1", "FA/Bills"));
}

#[tokio::test]
async fn error_mapping() {
    let s = MockServer::start().await;
    let cases: [(u16, serde_json::Value, ConnectorError); 6] = [
        (401, json!({}), ConnectorError::Unauthorized),
        (
            403,
            json!({"error": {"errors": [{"reason": "insufficientPermissions"}], "message": "Insufficient Permission"}}),
            ConnectorError::InsufficientScope,
        ),
        (
            403,
            json!({"error": {"errors": [{"reason": "userRateLimitExceeded"}]}}),
            ConnectorError::Retryable(403),
        ),
        (429, json!({}), ConnectorError::Retryable(429)),
        (503, json!({}), ConnectorError::Retryable(503)),
        (400, json!({}), ConnectorError::Http(400)),
    ];
    for (status, body, want) in cases {
        s.reset().await;
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/profile"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(&s)
            .await;
        let e = gmail::get_profile(&http(), &s.uri(), "t")
            .await
            .unwrap_err();
        assert_eq!(e, want, "status {status}");
    }
}
