//! Needs a GreenMail server; see the doc comment of `fa_connectors::imap` for the exact commands.
//! Ignored unless `FA_TEST_GREENMAIL=host:imap_port:imaps_port` is set.

use std::sync::atomic::{AtomicU32, Ordering};

use chrono::NaiveDate;
use fa_connectors::{ImapClient, ImapConfig, ImapSecurity, SpecialUse};
use tokio::net::TcpStream;

struct Env {
    host: String,
    imap: u16,
    imaps: u16,
}

fn env() -> Env {
    let v = std::env::var("FA_TEST_GREENMAIL").expect("FA_TEST_GREENMAIL=host:imap:imaps");
    let mut p = v.split(':');
    Env {
        host: p.next().unwrap().to_owned(),
        imap: p.next().unwrap().parse().unwrap(),
        imaps: p.next().unwrap().parse().unwrap(),
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

fn cfg(e: &Env, user: &str, tls: bool) -> ImapConfig {
    ImapConfig {
        host: e.host.clone(),
        port: if tls { e.imaps } else { e.imap },
        security: if tls {
            ImapSecurity::Implicit
        } else {
            ImapSecurity::Plain
        },
        username: user.to_owned(),
        password: "pw".into(),
        insecure_tls: tls,
    }
}

fn mail(n: u32) -> Vec<u8> {
    format!(
        "From: Sender <sender@example.com>\r\nTo: {n}@example.com\r\nSubject: Test {n}\r\n\
         Message-ID: <{n}@example.com>\r\nDate: Sat, 24 Oct 2026 10:00:00 +0000\r\n\
         \r\nBody {n} \u{e4}\u{f6}\r\nsecond line\r\n"
    )
    .into_bytes()
}

/// Appends messages through a separate raw session.
async fn append(e: &Env, user: &str, folder: &str, msgs: &[Vec<u8>], date: &str) {
    let tcp = TcpStream::connect((e.host.as_str(), e.imap)).await.unwrap();
    let mut c = async_imap::Client::new(tcp);
    c.read_response().await.unwrap();
    let mut s = c.login(user, "pw").await.map_err(|(e, _)| e).unwrap();
    if folder != "INBOX" {
        let _ = s.create(folder).await;
    }
    for m in msgs {
        s.append(folder, Some("(\\Flagged)"), Some(&format!("\"{date}\"")), m)
            .await
            .unwrap();
    }
    s.logout().await.unwrap();
}

#[tokio::test]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn connect_plain_and_tls_capabilities_folders() {
    let e = env();
    for tls in [false, true] {
        let u = user();
        let mut c = ImapClient::connect(cfg(&e, &u, tls)).await.unwrap();
        let caps = c.capabilities().await.unwrap();
        assert!(!caps.gmail_ext);
        assert!(caps.uidplus);
        let folders = c.list_folders().await.unwrap();
        assert!(folders.iter().any(|f| f.name == "INBOX"));
        let st = c.select("INBOX").await.unwrap();
        assert!(st.uidvalidity > 0 && st.uidnext >= 1);
        c.logout().await.unwrap();
    }
    // Unknown special-use folders have none.
    let u = user();
    append(&e, &u, "INBOX", &[mail(0)], "01-Oct-2026 10:00:00 +0000").await;
    let mut c = ImapClient::connect(cfg(&e, &u, false)).await.unwrap();
    assert!(
        c.list_folders()
            .await
            .unwrap()
            .iter()
            .all(|f| f.special_use != Some(SpecialUse::Junk) || f.name != "INBOX")
    );
}

#[tokio::test]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn search_since_fetch_raw_and_incremental() {
    let e = env();
    let u = user();
    let old = mail(1);
    let new_a = mail(2);
    let new_b = mail(3);
    append(
        &e,
        &u,
        "INBOX",
        std::slice::from_ref(&old),
        "01-Sep-2026 10:00:00 +0000",
    )
    .await;
    append(
        &e,
        &u,
        "INBOX",
        &[new_a.clone(), new_b.clone()],
        "24-Oct-2026 10:00:00 +0000",
    )
    .await;

    let mut c = ImapClient::connect(cfg(&e, &u, true)).await.unwrap();
    let st = c.select("INBOX").await.unwrap();
    let since = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let uids = c.uid_search_since(since, None).await.unwrap();
    assert_eq!(uids.len(), 2);

    let msgs = c.uid_fetch(&uids, false).await.unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].raw, new_a);
    assert_eq!(msgs[1].raw, new_b);
    assert!(msgs[0].flags.iter().any(|f| f == "\\Flagged"));
    // BODY.PEEK: nothing was marked read.
    assert!(!msgs[0].flags.iter().any(|f| f == "\\Seen"));
    assert_eq!(
        msgs[0].internal_date.unwrap().date_naive().to_string(),
        "2026-10-24"
    );
    assert!(msgs[0].gm_msgid.is_none() && msgs[0].gm_labels.is_empty());

    // Everything incl. the old one, in one batch of fetches.
    let all = c.uid_fetch_range(1, false).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].raw, old);

    // Incremental by UIDNEXT: nothing new, then one new message.
    assert!(st.uidnext > 3);
    assert!(
        c.uid_fetch_range(st.uidnext, false)
            .await
            .unwrap()
            .is_empty()
    );
    let newest = mail(4);
    append(
        &e,
        &u,
        "INBOX",
        std::slice::from_ref(&newest),
        "25-Oct-2026 10:00:00 +0000",
    )
    .await;
    let got = c.uid_fetch_range(st.uidnext, false).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].raw, newest);
    assert_eq!(got[0].uid, st.uidnext);
    c.logout().await.unwrap();
}

#[tokio::test]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn fetch_more_than_one_batch() {
    let e = env();
    let u = user();
    let msgs: Vec<_> = (0..55).map(mail).collect();
    append(&e, &u, "INBOX", &msgs, "24-Oct-2026 10:00:00 +0000").await;
    let mut c = ImapClient::connect(cfg(&e, &u, false)).await.unwrap();
    c.select("INBOX").await.unwrap();
    let uids = c
        .uid_search_since(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), None)
        .await
        .unwrap();
    let got = c.uid_fetch(&uids, false).await.unwrap();
    assert_eq!(got.len(), 55);
    assert_eq!(got[54].raw, msgs[54]);
}

#[tokio::test]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn move_and_flags() {
    let e = env();
    let u = user();
    append(
        &e,
        &u,
        "INBOX",
        &[mail(1), mail(2)],
        "24-Oct-2026 10:00:00 +0000",
    )
    .await;
    append(&e, &u, "Archive", &[mail(9)], "24-Oct-2026 10:00:00 +0000").await;
    let mut c = ImapClient::connect(cfg(&e, &u, false)).await.unwrap();
    c.select("INBOX").await.unwrap();
    let uids = c.uid_fetch_range(1, false).await.unwrap();
    let (a, b) = (uids[0].uid, uids[1].uid);

    c.uid_store_flags(a, true, &["\\Seen"]).await.unwrap();
    let m = c.uid_fetch(&[a], false).await.unwrap();
    assert!(m[0].flags.iter().any(|f| f == "\\Seen"));
    c.uid_store_flags(a, false, &["\\Seen"]).await.unwrap();
    let m = c.uid_fetch(&[a], false).await.unwrap();
    assert!(!m[0].flags.iter().any(|f| f == "\\Seen"));
    assert!(
        c.uid_store_flags(a, true, &["bad flag\r\nx"])
            .await
            .is_err()
    );

    c.uid_move(a, "Archive").await.unwrap();
    let left = c.uid_fetch_range(1, false).await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].uid, b);
    c.select("Archive").await.unwrap();
    let arch = c.uid_fetch_range(1, false).await.unwrap();
    assert_eq!(arch.len(), 2);
    assert!(arch.iter().any(|m| m.raw == mail(1)));
}

#[tokio::test]
#[ignore = "needs FA_TEST_GREENMAIL"]
async fn connect_refused_is_error() {
    let e = env();
    let mut c = cfg(&e, "x@example.com", false);
    c.port = 1;
    assert!(ImapClient::connect(c).await.is_err());
}
