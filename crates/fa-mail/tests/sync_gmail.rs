//! Gmail API sync and actions against a wiremock server.

#[path = "sync_common.rs"]
mod common;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::*;
use fa_domain::mail_api::{MailActionInput, NewMailSource};
use fa_domain::*;
use fa_mail::MailError;
use fa_mail::actions::ActionService;
use fa_mail::sources::{GMAIL_MODIFY, GMAIL_READONLY, SourceService};
use fa_mail::sync::SyncService;
use fa_store::*;
use serde_json::json;
use sqlx::PgPool;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const API: &str = "/gmail/v1/users/me";

async fn account(e: &Env, scopes: &[&str]) -> OAuthAccountId {
    let scopes: Vec<String> = scopes.iter().map(|s| (*s).to_owned()).collect();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let a = OAuthAccountRepo::upsert(
        &mut tx,
        e.h,
        "google",
        e.adult,
        "me@example.com",
        &scopes,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    a.value.id
}

fn new_gmail(acc: OAuthAccountId, read_only: bool) -> NewMailSource {
    NewMailSource {
        provider: MailProvider::Gmail,
        name: "Mine".into(),
        address: None,
        owners: vec![],
        visibility: None,
        read_only: Some(read_only),
        backfill_days: None,
        query: None,
        poll_minutes: None,
        oauth_account: Some(acc),
        imap: None,
    }
}

async fn make_source(e: &Env, scopes: &[&str], read_only: bool) -> MailSourceId {
    let acc = account(e, scopes).await;
    SourceService::new(e.deps.clone())
        .create(e.h, new_gmail(acc, read_only), e.adult)
        .await
        .unwrap()
        .id
}

async fn mount_profile(s: &MockServer, history: &str) {
    Mock::given(method("GET"))
        .and(path(format!("{API}/profile")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"emailAddress": "me@example.com", "historyId": history})),
        )
        .mount(s)
        .await;
}

async fn mount_list(s: &MockServer, ids: &[&str]) {
    let msgs: Vec<_> = ids
        .iter()
        .map(|i| json!({"id": i, "threadId": format!("t{i}")}))
        .collect();
    Mock::given(method("GET"))
        .and(path(format!("{API}/messages")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"messages": msgs})))
        .mount(s)
        .await;
}

async fn mount_message(s: &MockServer, id: &str, n: u32, labels: &[&str]) {
    let raw = URL_SAFE_NO_PAD.encode(rfc822(n, &format!("Subject {id}")));
    Mock::given(method("GET"))
        .and(path(format!("{API}/messages/{id}")))
        .and(query_param("format", "raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": id, "threadId": format!("t{id}"), "labelIds": labels,
            "internalDate": "1793000000000", "raw": raw,
        })))
        .mount(s)
        .await;
}

async fn first_sync(e: &Env, s: &MockServer, src: MailSourceId) {
    mount_profile(s, "100").await;
    mount_list(s, &["m1", "m2"]).await;
    mount_message(s, "m1", 1, &["INBOX", "UNREAD"]).await;
    mount_message(s, "m2", 2, &["INBOX"]).await;
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn first_sync_stores_pending_messages(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_READONLY], true).await;
    first_sync(&e, &server, src).await;

    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM message WHERE processing_state = 'pending'"
        )
        .await,
        2
    );
    let m = message(&e, "m1").await;
    assert_eq!(m.subject, "Subject m1");
    assert_eq!(m.from_addr, "sender@example.com");
    assert_eq!(
        m.gmail_labels,
        vec!["INBOX".to_owned(), "UNREAD".to_owned()]
    );
    assert_eq!(m.thread_id.as_deref(), Some("tm1"));
    assert_eq!(m.rfc822_id.as_deref(), Some("msg1@example.com"));
    assert!(m.snippet.unwrap().starts_with("Body 1"));
    let s = source(&e, src).await;
    assert_eq!(s.history_id.as_deref(), Some("100"));
    assert_eq!(s.state, MailSourceState::Ok);
    assert!(s.last_synced_at.is_some());
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.process'").await,
        2
    );

    // A second run without a history change adds nothing.
    Mock::given(method("GET"))
        .and(path(format!("{API}/history")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"historyId": "100"})))
        .mount(&server)
        .await;
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 2);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn incremental_adds_messages_and_applies_label_changes(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_READONLY], true).await;
    first_sync(&e, &server, src).await;

    mount_message(&server, "m3", 3, &["INBOX"]).await;
    Mock::given(method("GET"))
        .and(path(format!("{API}/history")))
        .and(query_param("startHistoryId", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "history": [
                {"messagesAdded": [{"message": {"id": "m3", "threadId": "tm3"}}]},
                {"labelsAdded": [{"message": {"id": "m1", "threadId": "tm1"}, "labelIds": ["SPAM"]}]},
                {"labelsRemoved": [{"message": {"id": "m2", "threadId": "tm2"}, "labelIds": ["INBOX"]}]},
            ],
            "historyId": "110",
        })))
        .mount(&server)
        .await;
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();

    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 3);
    assert!(
        message(&e, "m1")
            .await
            .gmail_labels
            .contains(&"SPAM".to_owned())
    );
    assert!(message(&e, "m2").await.gmail_labels.is_empty());
    assert_eq!(source(&e, src).await.history_id.as_deref(), Some("110"));
    // The move to Spam re-runs only the category decision.
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.process' AND payload->>'only' = 'email.category'").await,
        1
    );
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.process'").await,
        4
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn expired_history_falls_back_to_a_full_backfill(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_READONLY], true).await;
    first_sync(&e, &server, src).await;

    server.reset().await;
    mount_profile(&server, "200").await;
    mount_list(&server, &["m1", "m2", "m4"]).await;
    mount_message(&server, "m4", 4, &["INBOX"]).await;
    Mock::given(method("GET"))
        .and(path(format!("{API}/history")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();

    // Only the new message was fetched (m1 and m2 are stored); no duplicates.
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 3);
    let reqs = server.received_requests().await.unwrap();
    let fetched: Vec<_> = reqs
        .iter()
        .filter(|r| r.url.path().contains("/messages/"))
        .map(|r| r.url.path().to_owned())
        .collect();
    assert_eq!(fetched, vec![format!("{API}/messages/m4")]);
    assert_eq!(source(&e, src).await.history_id.as_deref(), Some("200"));
    assert_eq!(source(&e, src).await.state, MailSourceState::Ok);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn invalid_grant_pauses_the_source_and_notifies(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_READONLY], true).await;
    *e.tokens.0.lock().unwrap() = Err(TokenError::NeedsReauth);

    let svc = SyncService::new(e.deps.clone());
    svc.sync_source(e.h, src).await.unwrap();
    let s = source(&e, src).await;
    assert_eq!(s.state, MailSourceState::NeedsReauth);
    assert!(s.last_error.is_some());
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 0);
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM notification WHERE trigger = 'source_reauth'"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM oauth_account WHERE state = 'needs_reauth'"
        )
        .await,
        1
    );

    // Paused sources are skipped: no second notification.
    svc.sync_source(e.h, src).await.unwrap();
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM notification WHERE trigger = 'source_reauth'"
        )
        .await,
        1
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn transient_errors_retry_and_keep_the_source_active(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_READONLY], true).await;
    mount_profile(&server, "100").await;
    Mock::given(method("GET"))
        .and(path(format!("{API}/messages")))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let r = SyncService::new(e.deps.clone()).sync_source(e.h, src).await;
    assert!(matches!(r, Err(MailError::Connector(_))));
    let s = source(&e, src).await;
    assert_eq!(s.state, MailSourceState::Ok);
    assert!(s.last_error.is_some());
    assert!(s.history_id.is_none());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn source_creation_checks_scopes(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let svc = SourceService::new(e.deps.clone());

    let none = account(&e, &["openid"]).await;
    let r = svc.create(e.h, new_gmail(none, true), e.adult).await;
    assert!(matches!(r, Err(MailError::Invalid(m)) if m.contains("reconnect Google")));

    let ro = account(&e, &[GMAIL_READONLY]).await;
    let r = svc.create(e.h, new_gmail(ro, false), e.adult).await;
    assert!(
        matches!(r, Err(MailError::Invalid(_))),
        "writable needs gmail.modify"
    );
    let ok = svc.create(e.h, new_gmail(ro, true), e.adult).await.unwrap();
    assert_eq!(ok.address, "me@example.com");
    assert!(ok.can_manage && ok.read_only);

    // Another adult cannot attach the account.
    let other = member(e.h, "Other", Role::Adult);
    let mut tx = e.store.tx_system().await.unwrap();
    MemberRepo::insert(&mut tx, &other, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let r = svc.create(e.h, new_gmail(ro, true), other.id).await;
    assert!(matches!(r, Err(MailError::Forbidden)));

    // Lists show the source to its owner, and to nobody else unless household-visible.
    assert_eq!(svc.list(e.h, e.adult).await.unwrap().len(), 1);
    assert_eq!(svc.list(e.h, other.id).await.unwrap().len(), 0);
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.sync'").await,
        1
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn actions_label_archive_undo_and_auto_actions(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    let src = make_source(&e, &[GMAIL_MODIFY], false).await;
    first_sync(&e, &server, src).await;
    let msg = message(&e, "m1").await;
    let actions = ActionService::new(e.deps.clone());

    Mock::given(method("GET"))
        .and(path(format!("{API}/labels")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"labels": [{"id": "Label_1", "name": "Other"}]})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/labels")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id": "Label_9", "name": "FA/School"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/messages/m1/modify")))
        .and(body_json(
            json!({"addLabelIds": ["Label_9"], "removeLabelIds": []}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/messages/m1/modify")))
        .and(body_json(
            json!({"addLabelIds": [], "removeLabelIds": ["Label_9"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/messages/m1/modify")))
        .and(body_json(
            json!({"addLabelIds": [], "removeLabelIds": ["INBOX"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let a = actions
        .apply(
            e.h,
            msg.id,
            MailActionInput {
                action: MailActionKind::Label,
                label: Some("School".into()),
            },
            Some(e.adult),
            false,
        )
        .await
        .unwrap();
    assert_eq!(a.label.as_deref(), Some("FA/School"));
    assert!(a.error.is_none() && !a.automatic);
    let undone = actions.undo(e.h, a.id, Some(e.adult)).await.unwrap();
    assert!(undone.undone_at.is_some());
    assert!(matches!(
        actions.undo(e.h, a.id, Some(e.adult)).await,
        Err(MailError::Conflict)
    ));

    let arch = actions
        .apply(
            e.h,
            msg.id,
            MailActionInput {
                action: MailActionKind::Archive,
                label: None,
            },
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(arch.action, MailActionKind::Archive);
    assert_eq!(count(&e, "SELECT count(*) FROM mail_action").await, 2);

    // The 30-day window.
    exec(
        &e,
        "UPDATE mail_action SET done_at = now() - interval '31 days' WHERE action = 'archive'",
    )
    .await;
    assert!(matches!(
        actions.undo(e.h, arch.id, None).await,
        Err(MailError::Invalid(_))
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn auto_actions_label_once_and_read_only_is_refused(pool: PgPool) {
    let server = MockServer::start().await;
    let e = env(pool, &server.uri(), false).await;
    // Read-only source: manual actions are refused and auto actions do nothing.
    let ro = make_source(&e, &[GMAIL_MODIFY], true).await;
    first_sync(&e, &server, ro).await;
    let msg = message(&e, "m1").await;
    let actions = ActionService::new(e.deps.clone());
    let r = actions
        .apply(
            e.h,
            msg.id,
            MailActionInput {
                action: MailActionKind::MarkRead,
                label: None,
            },
            None,
            false,
        )
        .await;
    assert!(matches!(r, Err(MailError::ReadOnly)));
    actions.auto_actions(e.h, msg.id).await.unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM mail_action").await, 0);

    // Make the source writable, classify the message and link a member.
    exec(&e, "UPDATE mail_source SET read_only = false").await;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    mail_defaults::ensure(&mut tx, e.h).await.unwrap();
    MessageRepo::set_classification(&mut tx, msg.id, Some("school"), None, None)
        .await
        .unwrap();
    MessageRepo::set_members(&mut tx, msg.id, &[(e.adult, DecidedBy::Rule, 0.9)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    Mock::given(method("GET"))
        .and(path(format!("{API}/labels")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"labels": []})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/labels")))
        .respond_with(|r: &wiremock::Request| {
            let b: serde_json::Value = serde_json::from_slice(&r.body).unwrap();
            let name = b["name"].as_str().unwrap().to_owned();
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": format!("id-{name}"), "name": name}))
        })
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{API}/messages/m1/modify")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    actions.auto_actions(e.h, msg.id).await.unwrap();
    assert_eq!(
        count(&e, "SELECT count(*) FROM mail_action WHERE automatic AND action = 'label' AND label IN ('FA/School', 'FA/Adult') AND error IS NULL").await,
        2
    );
    let modifies = |reqs: Vec<wiremock::Request>| {
        reqs.iter()
            .filter(|r| r.url.path().ends_with("/modify"))
            .count()
    };
    assert_eq!(modifies(server.received_requests().await.unwrap()), 2);
    // A second run is a no-op.
    actions.auto_actions(e.h, msg.id).await.unwrap();
    assert_eq!(modifies(server.received_requests().await.unwrap()), 2);
    assert_eq!(count(&e, "SELECT count(*) FROM mail_action").await, 2);
}
