//! Shared helpers of the `sync_*` integration tests (included with `#[path]`).
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use fa_connectors::Http;
use fa_domain::*;
use fa_mail::{MailConfig, MailDeps};
use fa_store::*;
use sqlx::PgPool;

pub struct FakeTokens(pub Mutex<Result<String, TokenError>>);

#[async_trait]
impl AccessTokenProvider for FakeTokens {
    async fn access_token(&self, _: HouseholdId, _: OAuthAccountId) -> Result<String, TokenError> {
        self.0.lock().unwrap().clone()
    }
}

pub struct Env {
    pub store: Store,
    pub deps: Arc<MailDeps>,
    pub tokens: Arc<FakeTokens>,
    pub h: HouseholdId,
    pub adult: MemberId,
}

pub fn member(h: HouseholdId, name: &str, role: Role) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: name.into(),
        role,
        avatar: None,
        color: None,
        birthdate: None,
        grade: None,
        interests: vec![],
        hobbies: vec![],
        responsibilities: vec![],
        aliases: Vec::new(),
        gender: None,
        relevance_tags: Vec::new(),
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub async fn env(pool: PgPool, gmail_base: &str, allow_plain: bool) -> Env {
    let store = Store::from_pool(pool);
    let h = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
        tz: "UTC".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: Units::Metric,
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).expect("time"),
        quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
        currency: fa_domain::DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let adult = member(h.id, "Adult", Role::Adult);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    MemberRepo::insert(&mut tx, &adult, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let tokens = Arc::new(FakeTokens(Mutex::new(Ok("tok".into()))));
    let deps = Arc::new(MailDeps {
        store: store.clone(),
        http: Http::new(true).unwrap(),
        tokens: tokens.clone(),
        key: Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        cfg: MailConfig {
            gmail_api_base: gmail_base.to_owned(),
            allow_plain_imap: allow_plain,
            imap_tls_insecure: false,
            allow_private_hosts: true,
        },
    });
    Env {
        store,
        deps,
        tokens,
        h: h.id,
        adult: adult.id,
    }
}

pub async fn count(e: &Env, sql: &str) -> i64 {
    let mut tx = e.store.tx_system().await.unwrap();
    sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap()
}

pub async fn exec(e: &Env, sql: &str) {
    let mut tx = e.store.tx_system().await.unwrap();
    sqlx::query(sql).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

pub async fn source(e: &Env, id: MailSourceId) -> MailSource {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    MailSourceRepo::get(&mut tx, id)
        .await
        .unwrap()
        .unwrap()
        .value
}

pub async fn message_ids(e: &Env) -> Vec<MessageId> {
    let mut tx = e.store.tx_system().await.unwrap();
    let rows: Vec<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM message ORDER BY received_at, remote_id")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    rows.into_iter().map(Into::into).collect()
}

pub async fn message(e: &Env, remote_id: &str) -> Message {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM message WHERE remote_id = $1")
        .bind(remote_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    MessageRepo::get(&mut tx, id.into()).await.unwrap().unwrap()
}

/// A fictional RFC 822 message.
pub fn rfc822(n: u32, subject: &str) -> Vec<u8> {
    format!(
        "From: Sender <sender@example.com>\r\nTo: me@example.com\r\nSubject: {subject}\r\n\
         Message-ID: <msg{n}@example.com>\r\nDate: Sat, 24 Oct 2026 10:00:00 +0000\r\n\
         \r\nBody {n} of the test message.\r\n"
    )
    .into_bytes()
}
