//! IMAP sync and actions against a throwaway GreenMail server. Ignored unless
//! `FA_TEST_GREENMAIL=host:imap_port:imaps_port` is set (see `fa_connectors::imap` for the
//! docker command).

#[path = "sync_common.rs"]
mod common;

use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Utc;
use common::*;
use fa_domain::mail_api::{ImapSettings, ImapTestRequest, MailActionInput, NewMailSource};
use fa_domain::*;
use fa_mail::actions::ActionService;
use fa_mail::sources::SourceService;
use fa_mail::sync::SyncService;
use sqlx::PgPool;
use tokio::net::TcpStream;

struct Gm {
    host: String,
    imap: u16,
}

fn gm() -> Gm {
    let v = std::env::var("FA_TEST_GREENMAIL").expect("FA_TEST_GREENMAIL=host:imap:imaps");
    let mut p = v.split(':');
    Gm {
        host: p.next().unwrap().to_owned(),
        imap: p.next().unwrap().parse().unwrap(),
    }
}

static N: AtomicU32 = AtomicU32::new(0);

fn user() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("u{t}-{}@example.com", N.fetch_add(1, Ordering::Relaxed))
}

fn today() -> String {
    Utc::now().format("%d-%b-%Y %H:%M:%S +0000").to_string()
}

async fn session(g: &Gm, user: &str) -> async_imap::Session<TcpStream> {
    let tcp = TcpStream::connect((g.host.as_str(), g.imap)).await.unwrap();
    let mut c = async_imap::Client::new(tcp);
    c.read_response().await.unwrap();
    c.login(user, "pw").await.map_err(|(e, _)| e).unwrap()
}

async fn append(g: &Gm, user: &str, folder: &str, msgs: &[Vec<u8>]) {
    let mut s = session(g, user).await;
    if folder != "INBOX" {
        let _ = s.create(folder).await;
    }
    for m in msgs {
        s.append(folder, None::<&str>, Some(&format!("\"{}\"", today())), m)
            .await
            .unwrap();
    }
    s.logout().await.unwrap();
}

async fn exists(g: &Gm, user: &str, folder: &str) -> u32 {
    let mut s = session(g, user).await;
    let n = s.select(folder).await.map(|m| m.exists).unwrap_or(0);
    s.logout().await.unwrap();
    n
}

fn imap_source(g: &Gm, user: &str, folders: &[&str], read_only: bool) -> NewMailSource {
    NewMailSource {
        provider: MailProvider::Imap,
        name: "Box".into(),
        address: None,
        owners: vec![],
        visibility: None,
        read_only: Some(read_only),
        backfill_days: Some(30),
        query: None,
        poll_minutes: None,
        oauth_account: None,
        imap: Some(ImapSettings {
            host: g.host.clone(),
            port: i32::from(g.imap),
            username: user.to_owned(),
            password: Some("pw".into()),
            folders: Some(folders.iter().map(|f| (*f).to_owned()).collect()),
            archive_folder: Some("Archive".into()),
        }),
    }
}

async fn setup(pool: PgPool, folders: &[&str], read_only: bool) -> (Env, Gm, String, MailSourceId) {
    let g = gm();
    let u = user();
    // Create the mailbox (and the extra folders) before the source tests its connection.
    append(&g, &u, "Archive", &[]).await;
    for f in folders.iter().filter(|f| **f != "INBOX") {
        append(&g, &u, f, &[]).await;
    }
    let e = env(pool, "http://127.0.0.1:1", true).await;
    let v = SourceService::new(e.deps.clone())
        .create(e.h, imap_source(&g, &u, folders, read_only), e.adult)
        .await
        .unwrap();
    (e, g, u, v.id)
}

#[sqlx::test(migrations = "../fa-store/migrations")]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn first_sync_then_incremental_by_uidnext(pool: PgPool) {
    let (e, g, u, src) = setup(pool, &["INBOX"], true).await;
    append(
        &g,
        &u,
        "INBOX",
        &[rfc822(1, "One"), rfc822(2, "Two"), rfc822(3, "Three")],
    )
    .await;
    let svc = SyncService::new(e.deps.clone());
    svc.sync_source(e.h, src).await.unwrap();

    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM message WHERE processing_state = 'pending'"
        )
        .await,
        3
    );
    let m = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM message WHERE subject = 'One'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        fa_store::MessageRepo::get(&mut tx, id.into())
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(m.folder.as_deref(), Some("INBOX"));
    assert!(
        m.remote_id.contains(':'),
        "uidvalidity:uid, got {}",
        m.remote_id
    );
    assert_eq!(m.rfc822_id.as_deref(), Some("msg1@example.com"));
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM mail_folder_state WHERE folder = 'INBOX'"
        )
        .await,
        1
    );
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.process'").await,
        3
    );
    assert!(source(&e, src).await.last_synced_at.is_some());
    // The password is stored as a secret, not on the source row.
    assert_eq!(
        count(
            &e,
            "SELECT count(*) FROM secret WHERE owner_type = 'mail_source'"
        )
        .await,
        1
    );

    // Nothing new: nothing added.
    svc.sync_source(e.h, src).await.unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 3);

    append(&g, &u, "INBOX", &[rfc822(4, "Four"), rfc822(5, "Five")]).await;
    svc.sync_source(e.h, src).await.unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 5);
    assert_eq!(
        count(&e, "SELECT count(*) FROM job WHERE kind = 'mail.process'").await,
        5
    );
    let uidnext: i64 = count(
        &e,
        "SELECT uidnext FROM mail_folder_state WHERE folder = 'INBOX'",
    )
    .await;
    assert_eq!(uidnext, 6);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn uidvalidity_reset_refetches_without_duplicates(pool: PgPool) {
    let (e, g, u, src) = setup(pool, &["INBOX", "Work"], true).await;
    append(&g, &u, "Work", &[rfc822(1, "One"), rfc822(2, "Two")]).await;
    let svc = SyncService::new(e.deps.clone());
    svc.sync_source(e.h, src).await.unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 2);
    let before = count(
        &e,
        "SELECT uidvalidity FROM mail_folder_state WHERE folder = 'Work'",
    )
    .await;

    // Replace the folder: the server hands out a new UIDVALIDITY. (GreenMail drops the
    // connection on DELETE of a folder other sessions had selected, so the old folder is renamed
    // away instead; for the client the effect is the same.)
    {
        let mut s = session(&g, &u).await;
        s.run_command_and_check_ok("RENAME \"Work\" \"Old\"")
            .await
            .unwrap();
        s.logout().await.unwrap();
    }
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    append(
        &g,
        &u,
        "Work",
        &[rfc822(1, "One"), rfc822(2, "Two"), rfc822(3, "Three")],
    )
    .await;
    svc.sync_source(e.h, src).await.unwrap();

    let after = count(
        &e,
        "SELECT uidvalidity FROM mail_folder_state WHERE folder = 'Work'",
    )
    .await;
    assert_ne!(
        before, after,
        "the server must have issued a new UIDVALIDITY"
    );
    // The two known messages are recognised by Message-ID; only the new one is added.
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 3);
    assert_eq!(
        count(&e, "SELECT count(DISTINCT rfc822_id) FROM message").await,
        3
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn move_action_and_its_undo(pool: PgPool) {
    let (e, g, u, src) = setup(pool, &["INBOX"], false).await;
    append(&g, &u, "INBOX", &[rfc822(1, "One"), rfc822(2, "Two")]).await;
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();
    let id = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM message WHERE subject = 'One'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        MessageId::from(id)
    };
    let actions = ActionService::new(e.deps.clone());

    let a = actions
        .apply(
            e.h,
            id,
            MailActionInput {
                action: MailActionKind::Archive,
                label: None,
            },
            Some(e.adult),
            false,
        )
        .await
        .unwrap();
    assert_eq!(exists(&g, &u, "INBOX").await, 1);
    assert_eq!(exists(&g, &u, "Archive").await, 1);
    let moved = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        fa_store::MessageRepo::get(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(moved.folder.as_deref(), Some("Archive"));

    // The next sync does not resurrect or duplicate the archived message.
    SyncService::new(e.deps.clone())
        .sync_source(e.h, src)
        .await
        .unwrap();
    assert_eq!(count(&e, "SELECT count(*) FROM message").await, 2);

    let undone = actions.undo(e.h, a.id, Some(e.adult)).await.unwrap();
    assert!(undone.undone_at.is_some());
    assert_eq!(exists(&g, &u, "INBOX").await, 2);
    assert_eq!(exists(&g, &u, "Archive").await, 0);
    let back = {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        fa_store::MessageRepo::get(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(back.folder.as_deref(), Some("INBOX"));

    // Mark read and a keyword label work on the message at its new location.
    actions
        .apply(
            e.h,
            id,
            MailActionInput {
                action: MailActionKind::MarkRead,
                label: None,
            },
            None,
            false,
        )
        .await
        .unwrap();
    actions
        .apply(
            e.h,
            id,
            MailActionInput {
                action: MailActionKind::Label,
                label: Some("School".into()),
            },
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        count(&e, "SELECT count(*) FROM mail_action WHERE error IS NULL").await,
        3
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn plain_imap_is_refused_unless_allowed(pool: PgPool) {
    let g = gm();
    let u = user();
    append(&g, &u, "INBOX", &[]).await;
    let req = || ImapTestRequest {
        host: g.host.clone(),
        port: i32::from(g.imap),
        username: u.clone(),
        password: "pw".into(),
    };
    let strict = env(pool.clone(), "http://127.0.0.1:1", false).await;
    let r = SourceService::new(strict.deps.clone())
        .test_imap(req())
        .await;
    assert!(!r.ok, "plain IMAP must be refused by default");
    let lax = env(pool, "http://127.0.0.1:1", true).await;
    let r = SourceService::new(lax.deps.clone()).test_imap(req()).await;
    assert!(r.ok, "{:?}", r.error);
    assert!(r.folders.iter().any(|f| f == "INBOX"));
    assert!(!r.gmail_ext);
}
