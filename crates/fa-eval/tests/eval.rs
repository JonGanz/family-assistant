//! The eval over a real Postgres, fictional messages and an axum stub LLM. Never run against the
//! dev database.

use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, routing::post};
use chrono::{NaiveTime, Utc};
use fa_domain::*;
use fa_eval::{EvalOpts, run};
use fa_pipeline::Pipeline;
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;

type Calls = Arc<Mutex<u32>>;

/// Answers by the subject line, so each message gets its own reply.
async fn chat(State(calls): State<Calls>, Json(body): Json<Value>) -> Json<Value> {
    *calls.lock().unwrap() += 1;
    let text = body.to_string();
    let reply = if text.contains("Zebra") || text.contains("Hello") {
        json!({"category": {"value": if text.contains("Zebra") { "school" } else { "other" }, "confidence": 0.9}})
    } else if text.contains("Pool") {
        json!({"category": {"value": "other", "confidence": 0.9}})
    } else {
        json!({"category": {"value": "school", "confidence": 0.3}})
    };
    Json(json!({
        "choices": [{"message": {"content": reply.to_string()}}],
        "usage": {"prompt_tokens": 11, "completion_tokens": 7}
    }))
}

async fn stub() -> (String, Calls) {
    let calls: Calls = Arc::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(chat))
        .with_state(calls.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, calls)
}

struct Env {
    store: Store,
    h: HouseholdId,
    src: MailSourceId,
    pat: MemberId,
    provider: Arc<fa_llm::Provider>,
}

async fn env(pool: PgPool, base_url: Option<String>) -> Env {
    let store = Store::from_pool(pool);
    let h = HouseholdId::new();
    let hh = Household {
        id: h,
        name: "Test".into(),
        tz: "UTC".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: Units::Metric,
        week_start: Default::default(),
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        quiet_until: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        currency: DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let pat = Member {
        id: MemberId::new(),
        household_id: h,
        display_name: "Pat".into(),
        role: Role::Adult,
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
    };
    let t = Utc::now();
    let src = MailSource {
        id: MailSourceId::new(),
        household_id: h,
        provider: MailProvider::Imap,
        name: "Parent mail".into(),
        address: "parent@example.com".into(),
        oauth_account_id: None,
        imap_host: Some("imap.example.com".into()),
        imap_port: Some(993),
        imap_username: Some("parent@example.com".into()),
        secret_id: None,
        folders: vec!["INBOX".into()],
        archive_folder: None,
        gmail_ext: false,
        read_only: false,
        visibility: MailVisibility::Owners,
        backfill_days: 30,
        query: None,
        poll_minutes: 5,
        history_id: None,
        state: MailSourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: t,
        updated_at: t,
    };
    let school = Org {
        id: OrgId::new(),
        household_id: h,
        kind: OrgKind::School,
        name: "Example Elementary".into(),
        domains: vec!["school.example.edu".into()],
        senders: vec![],
        ics_calendar_id: None,
        default_calendar_id: None,
        contact_mode: ContactMode::Manual,
        bill_amount_regex: None,
        bill_due_regex: None,
        bill_account_regex: None,
        archived_at: None,
        created_at: t,
        updated_at: t,
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    MemberRepo::insert(&mut tx, &pat, None, None).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &src, &[pat.id])
        .await
        .unwrap();
    OrgRepo::insert(&mut tx, &school).await.unwrap();
    if let Some(url) = base_url {
        LlmEndpointRepo::upsert(
            &mut tx,
            h,
            &LlmEndpointInput {
                base_url: url,
                model: "test-model".into(),
                dialect: LlmDialect::OpenaiChat,
                timeout_ms: 5000,
                thinking: false,
                enabled: true,
            },
        )
        .await
        .unwrap();
    }
    Pipeline::ensure_defaults(&mut tx, h).await.unwrap();
    tx.commit().await.unwrap();
    let provider = Arc::new(fa_llm::Provider::new(
        store.clone(),
        Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        fa_llm::default_http().unwrap(),
    ));
    Env {
        store,
        h,
        src: src.id,
        pat: pat.id,
        provider,
    }
}

impl Env {
    /// Inserts a message; `truth` labels it and `sample` puts it in the eval sample.
    async fn msg(&self, from: &str, subject: &str, truth: Option<&str>, sample: bool) -> MessageId {
        let m = NewMessage {
            household_id: self.h,
            source_id: self.src,
            remote_id: format!("r-{subject}-{from}"),
            folder: Some("INBOX".into()),
            thread_id: None,
            rfc822_id: None,
            received_at: Utc::now(),
            from_addr: from.into(),
            from_name: None,
            to_addrs: vec![],
            cc_addrs: vec![],
            subject: subject.into(),
            gmail_labels: vec![],
            has_attachments: false,
            body_text: Some(format!("{subject}: a fictional note.")),
            body_html: None,
            snippet: None,
            headers: Some(json!({})),
            attachments: Some(json!([])),
        };
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        let id = MessageRepo::insert_parsed(&mut tx, &m)
            .await
            .unwrap()
            .expect("new message");
        if sample {
            MessageRepo::set_eval_sample(&mut tx, id, true)
                .await
                .unwrap();
        }
        if let Some(t) = truth {
            Pipeline::record_human(
                &mut tx,
                self.h,
                DecisionPointKey::EmailCategory,
                Subject {
                    kind: SubjectType::Message,
                    id: id.as_uuid(),
                },
                json!(t),
                Some(self.pat),
            )
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();
        id
    }

    async fn fill(&self) {
        let school = "office@school.example.edu";
        let other = "noreply@example.com";
        // Rules decide these two: one right, one wrong.
        self.msg(school, "Newsletter", Some("school"), true).await;
        self.msg(school, "Lunch menu", Some("other"), true).await;
        // The rules leave these to the model.
        self.msg(other, "Zebra crossing meeting", Some("school"), true)
            .await; // right
        self.msg(other, "Hello neighbour", Some("other"), true)
            .await; // right
        self.msg(other, "Pool schedule", Some("school"), true).await; // wrong
        self.msg(other, "Mystery", Some("school"), true).await; // low confidence
        // Ignored: no human label, and not in the sample.
        self.msg(other, "Unlabelled", None, true).await;
        self.msg(other, "Outside the sample", Some("other"), false)
            .await;
    }

    async fn mode(&self, mode: MailMode) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        AiSettingsRepo::update_all(
            &mut tx,
            self.h,
            &AiSettingsInput {
                mail_extract_mode: MailExtractMode::Fallback,
                max_input_chars: 12000,
                mail_mode: mode,
                catchup_days: 30,
                extract_fallback_cutoff: 0.8,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        sqlx::query_scalar(sql).fetch_one(&mut *tx).await.unwrap()
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rules_only_then_with_llm(pool: PgPool) {
    let (url, calls) = stub().await;
    let e = env(pool, Some(url)).await;
    e.fill().await;
    e.mode(MailMode::RulesFirst).await;
    let decisions = e.count("SELECT count(*) FROM decision").await;
    let jobs = e.count("SELECT count(*) FROM job").await;

    let dp = DecisionPointKey::EmailCategory;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let r = run(&mut tx, e.h, dp, EvalOpts::default(), None)
        .await
        .unwrap();
    assert_eq!(r.items, 6);
    assert_eq!(r.per_stage.len(), 1);
    assert_eq!((r.per_stage[0].decided, r.per_stage[0].correct), (2, 1));
    assert_eq!(r.undecided, 4);
    assert!((r.accuracy - 0.5).abs() < 1e-9);
    assert!((r.coverage - 2.0 / 6.0).abs() < 1e-9);
    assert_eq!(*calls.lock().unwrap(), 0);

    let opts = EvalOpts {
        with_llm: true,
        limit: None,
    };
    let r = run(&mut tx, e.h, dp, opts, Some(&e.provider))
        .await
        .unwrap();
    assert_eq!(r.items, 6);
    assert_eq!(r.per_stage.len(), 2);
    assert_eq!((r.per_stage[0].decided, r.per_stage[0].correct), (2, 1));
    assert_eq!((r.per_stage[1].decided, r.per_stage[1].correct), (3, 2));
    assert_eq!((r.undecided, r.llm_errors), (1, 0));
    assert!((r.accuracy - 3.0 / 5.0).abs() < 1e-9);
    assert!((r.coverage - 5.0 / 6.0).abs() < 1e-9);
    assert_eq!(*calls.lock().unwrap(), 4);
    assert!(r.per_stage[1].p95_ms >= r.per_stage[1].p50_ms);

    let limited = EvalOpts {
        with_llm: false,
        limit: Some(2),
    };
    assert_eq!(run(&mut tx, e.h, dp, limited, None).await.unwrap().items, 2);
    drop(tx);

    assert_eq!(e.count("SELECT count(*) FROM decision").await, decisions);
    assert_eq!(e.count("SELECT count(*) FROM job").await, jobs);
    assert_eq!(
        e.count("SELECT count(*) FROM llm_call WHERE purpose = 'eval'")
            .await,
        4
    );
    assert_eq!(e.count("SELECT count(*) FROM llm_call").await, 4);
}

/// D-107: in llm_first the model answers every item and the rules decide only what it leaves
/// open.
#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_asks_every_item_and_falls_back_to_the_rules(pool: PgPool) {
    let (url, calls) = stub().await;
    let e = env(pool, Some(url)).await;
    e.fill().await;
    e.mode(MailMode::LlmFirst).await;
    let dp = DecisionPointKey::EmailCategory;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let opts = EvalOpts {
        with_llm: true,
        limit: None,
    };
    let r = run(&mut tx, e.h, dp, opts, Some(&e.provider))
        .await
        .unwrap();
    assert_eq!(r.items, 6);
    // The model is unsure about the two items the rules decide (0.3 is below its threshold), so
    // the rules keep them; it decides the three clear ones; one item stays open.
    assert_eq!((r.per_stage[0].decided, r.per_stage[0].correct), (2, 1));
    assert_eq!((r.per_stage[1].decided, r.per_stage[1].correct), (3, 2));
    assert_eq!((r.undecided, r.llm_errors), (1, 0));
    assert_eq!(*calls.lock().unwrap(), 6);
    // Without --with-llm the mode changes nothing.
    let r = run(&mut tx, e.h, dp, EvalOpts::default(), None)
        .await
        .unwrap();
    assert_eq!((r.per_stage.len(), r.per_stage[0].decided), (1, 2));
    assert_eq!(*calls.lock().unwrap(), 6);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn with_llm_needs_an_enabled_endpoint(pool: PgPool) {
    let e = env(pool, None).await;
    e.fill().await;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let opts = EvalOpts {
        with_llm: true,
        limit: None,
    };
    let dp = DecisionPointKey::EmailCategory;
    let err = run(&mut tx, e.h, dp, opts, Some(&e.provider))
        .await
        .unwrap_err();
    assert!(matches!(err, fa_eval::EvalError::NotConfigured(_)), "{err}");
    let err = run(&mut tx, e.h, dp, opts, None).await.unwrap_err();
    assert!(matches!(err, fa_eval::EvalError::NotConfigured(_)));
}
