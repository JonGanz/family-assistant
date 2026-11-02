//! IMAP source hardening (M3.7 T-08): the private-address policy and the password rule on edits.
//! None of these need a mail server.

#[path = "sync_common.rs"]
mod common;

use std::sync::Arc;

use chrono::Utc;
use common::*;
use fa_domain::mail_api::{ImapSettings, ImapTestRequest, MailSourcePatch};
use fa_domain::*;
use fa_mail::MailError;
use fa_mail::sources::SourceService;
use fa_store::*;
use sqlx::PgPool;
use tokio::net::TcpListener;

async fn free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    l.local_addr().unwrap().port()
}

fn req(port: u16) -> ImapTestRequest {
    ImapTestRequest {
        host: "127.0.0.1".into(),
        port: i32::from(port),
        username: "me@example.com".into(),
        password: "pw".into(),
    }
}

/// C2-03: with the default policy a loopback server is refused, and the answer is the same as for
/// a closed port, so `imap/test` tells nothing about what is listening.
#[sqlx::test(migrations = "../fa-store/migrations")]
async fn imap_test_refuses_private_hosts_without_an_oracle(pool: PgPool) {
    let e = env(pool, "http://127.0.0.1:1", true).await;
    let mut deps = (*e.deps).clone();
    deps.cfg.allow_private_hosts = false;
    let svc = SourceService::new(Arc::new(deps));

    // A live listener that would accept the connection, and a closed port.
    let live = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let live_port = live.local_addr().unwrap().port();
    let a = svc.test_imap(req(live_port)).await;
    let b = svc.test_imap(req(free_port().await)).await;
    assert!(!a.ok && !b.ok);
    assert_eq!(a.error, b.error);
    let msg = a.error.unwrap();
    assert!(msg.contains("could not connect"), "{msg}");
    // The refused address never reached the listener.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(200), live.accept())
            .await
            .is_err()
    );
}

async fn imap_source(e: &Env) -> Versioned<MailSource> {
    let src = MailSource {
        id: MailSourceId::new(),
        household_id: e.h,
        provider: MailProvider::Imap,
        name: "Box".into(),
        address: "me@example.com".into(),
        oauth_account_id: None,
        imap_host: Some("imap.example.com".into()),
        imap_port: Some(993),
        imap_username: Some("me@example.com".into()),
        secret_id: None,
        folders: vec!["INBOX".into()],
        archive_folder: None,
        gmail_ext: false,
        read_only: true,
        visibility: MailVisibility::Owners,
        backfill_days: 30,
        query: None,
        poll_minutes: 15,
        history_id: None,
        state: MailSourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let v = MailSourceRepo::insert(&mut tx, &src, &[e.adult])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    v
}

fn patch(host: &str, port: i32, user: &str, password: Option<&str>) -> MailSourcePatch {
    MailSourcePatch {
        imap: Some(ImapSettings {
            host: host.into(),
            port,
            username: user.into(),
            password: password.map(str::to_owned),
            folders: None,
            archive_folder: None,
        }),
        ..Default::default()
    }
}

/// C2-07: the stored password only stays with an unchanged server and account.
#[sqlx::test(migrations = "../fa-store/migrations")]
async fn changing_host_port_or_username_requires_the_password(pool: PgPool) {
    let e = env(pool, "http://127.0.0.1:1", true).await;
    let svc = SourceService::new(e.deps.clone());
    let v = imap_source(&e).await;
    let id = v.value.id;

    let cases = [
        patch("evil.example.net", 993, "me@example.com", None),
        patch("imap.example.com", 143, "me@example.com", None),
        patch("imap.example.com", 993, "other@example.com", None),
        patch("evil.example.net", 993, "me@example.com", Some("")),
    ];
    for p in &cases {
        let r = svc.update(e.h, e.adult, id, p, v.version).await;
        assert!(matches!(r, Err(MailError::Invalid(_))), "{p:?}: {r:?}");
    }
    let s = source(&e, id).await;
    assert_eq!(s.imap_host.as_deref(), Some("imap.example.com"));

    // Unchanged connection settings keep the stored password (only the folders change).
    let mut keep = patch("imap.example.com", 993, "me@example.com", None);
    keep.imap.as_mut().unwrap().folders = Some(vec!["INBOX".into(), "Bills".into()]);
    let after = svc
        .update(e.h, e.adult, id, &keep, v.version)
        .await
        .unwrap();
    assert_eq!(source(&e, id).await.folders.len(), 2);

    // A new password with the new host is accepted.
    let ok = patch("imap.example.org", 993, "me@example.com", Some("new"));
    let version = after.version;
    svc.update(e.h, e.adult, id, &ok, version).await.unwrap();
    assert_eq!(
        source(&e, id).await.imap_host.as_deref(),
        Some("imap.example.org")
    );
}
