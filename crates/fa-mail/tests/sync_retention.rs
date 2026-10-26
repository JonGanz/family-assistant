//! Retention policy (D-034).

#[path = "sync_common.rs"]
mod common;

use chrono::Utc;
use common::*;
use fa_domain::*;
use fa_mail::retention::{RetentionResult, run_household};
use fa_store::*;
use sqlx::PgPool;

struct Fixture {
    e: Env,
    source: MailSourceId,
}

async fn fixture(pool: PgPool, policy: &str, examples: i32) -> Fixture {
    let e = env(pool, "http://127.0.0.1:1", false).await;
    let source = MailSourceId::new();
    let now = Utc::now();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let acc =
        OAuthAccountRepo::upsert(&mut tx, e.h, "google", e.adult, "me@example.com", &[], None)
            .await
            .unwrap()
            .value
            .id;
    MailSourceRepo::insert(
        &mut tx,
        &MailSource {
            id: source,
            household_id: e.h,
            provider: MailProvider::Gmail,
            name: "Mine".into(),
            address: "me@example.com".into(),
            oauth_account_id: Some(acc),
            imap_host: None,
            imap_port: None,
            imap_username: None,
            secret_id: None,
            folders: vec!["INBOX".into()],
            archive_folder: None,
            gmail_ext: false,
            read_only: true,
            visibility: MailVisibility::Owners,
            backfill_days: 30,
            query: None,
            poll_minutes: 5,
            history_id: None,
            state: MailSourceState::Ok,
            last_synced_at: None,
            last_error: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
        },
        &[e.adult],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE household SET mail_content_policy = $1, mail_examples_per_category = $2")
        .bind(policy)
        .bind(examples)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    Fixture { e, source }
}

/// Inserts a message with content; `age_days` backdates `received_at` and `created_at`.
async fn add(f: &Fixture, id: &str, category: &str, state: &str, age_days: i64, flags: &str) {
    let mut tx = f.e.store.tx_for(f.e.h).await.unwrap();
    let mid = MessageRepo::insert_parsed(
        &mut tx,
        &NewMessage {
            household_id: f.e.h,
            source_id: f.source,
            remote_id: id.into(),
            folder: None,
            thread_id: None,
            rfc822_id: None,
            received_at: Utc::now() - chrono::Duration::days(age_days),
            from_addr: "sender@example.com".into(),
            from_name: None,
            to_addrs: vec![],
            cc_addrs: vec![],
            subject: "Hello".into(),
            gmail_labels: vec![],
            has_attachments: false,
            body_text: Some("body".into()),
            body_html: Some("<p>body</p>".into()),
            snippet: Some("body".into()),
            headers: Some(serde_json::json!({"list-id": "x"})),
            attachments: Some(serde_json::json!([{"part": "1", "mime": "text/calendar", "size": 3, "ics": "BEGIN"}])),
        },
    )
    .await
    .unwrap()
    .unwrap();
    sqlx::query(&format!(
        "UPDATE message SET category = $2, processing_state = $3,
                created_at = now() - make_interval(days => $4::int) {flags} WHERE id = $1"
    ))
    .bind(mid.as_uuid())
    .bind(category)
    .bind(state)
    .bind(age_days as i32)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn purged(f: &Fixture, id: &str) -> bool {
    count(
        &f.e,
        &format!("SELECT count(*) FROM message WHERE remote_id = '{id}' AND content_purged_at IS NOT NULL AND body_text IS NULL AND body_html IS NULL AND snippet IS NULL AND headers IS NULL AND NOT (attachments::text LIKE '%BEGIN%')"),
    )
    .await
        == 1
}

async fn state_of(f: &Fixture, id: &str) -> String {
    let mut tx = f.e.store.tx_system().await.unwrap();
    sqlx::query_scalar("SELECT processing_state FROM message WHERE remote_id = $1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn purge_after_processing_and_expiry(pool: PgPool) {
    let f = fixture(pool, "purge_after_processing", 200).await;
    add(&f, "done", "school", "processed", 1, "").await;
    add(&f, "fresh", "school", "pending", 1, "").await;
    add(&f, "stale", "school", "pending", 30, "").await;
    let r = run_household(&f.e.store, f.e.h, Utc::now()).await.unwrap();
    assert_eq!(
        r,
        RetentionResult {
            expired: 1,
            purged: 2
        }
    );
    assert!(purged(&f, "done").await);
    assert!(purged(&f, "stale").await);
    assert!(!purged(&f, "fresh").await);
    assert_eq!(state_of(&f, "stale").await, "expired");
    assert_eq!(state_of(&f, "fresh").await, "pending");
    // The metadata stays.
    assert_eq!(
        count(&f.e, "SELECT count(*) FROM message WHERE subject = 'Hello'").await,
        3
    );
    // Idempotent.
    let again = run_household(&f.e.store, f.e.h, Utc::now()).await.unwrap();
    assert_eq!(again, RetentionResult::default());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn keep_for_learning_keeps_examples_and_eval_sample(pool: PgPool) {
    let f = fixture(pool, "keep_for_learning", 2).await;
    let learn = ", is_learning_example = true";
    add(&f, "l1", "school", "processed", 10, learn).await;
    add(&f, "l2", "school", "processed", 5, learn).await;
    add(&f, "l3", "school", "processed", 1, learn).await;
    add(
        &f,
        "eval",
        "school",
        "processed",
        20,
        ", in_eval_sample = true",
    )
    .await;
    add(&f, "plain", "school", "processed", 3, "").await;
    add(&f, "other-cat", "bills", "processed", 3, learn).await;
    run_household(&f.e.store, f.e.h, Utc::now()).await.unwrap();
    // The two newest learning examples per category survive, the oldest does not.
    assert!(purged(&f, "l1").await);
    assert!(!purged(&f, "l2").await);
    assert!(!purged(&f, "l3").await);
    assert!(!purged(&f, "eval").await);
    assert!(purged(&f, "plain").await);
    assert!(!purged(&f, "other-cat").await);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn keep_all_only_expires(pool: PgPool) {
    let f = fixture(pool, "keep_all", 200).await;
    add(&f, "done", "school", "processed", 90, "").await;
    add(&f, "stale", "school", "pending", 30, "").await;
    let r = run_household(&f.e.store, f.e.h, Utc::now()).await.unwrap();
    assert_eq!(
        r,
        RetentionResult {
            expired: 1,
            purged: 0
        }
    );
    assert!(!purged(&f, "done").await);
    assert!(!purged(&f, "stale").await);
    assert_eq!(state_of(&f, "stale").await, "expired");
}
