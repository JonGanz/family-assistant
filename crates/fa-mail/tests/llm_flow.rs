//! The `llm` stage, the `mail.llm` job and `llm.prune` over a real Postgres and an axum stub LLM.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{Json, Router, extract::State, routing::post};
use chrono::{DateTime, Duration, NaiveTime, TimeZone, Utc};
use fa_domain::*;
use fa_jobs::JobContext;
use fa_mail::MailDeps;
use fa_mail::process::ProcessService;
use fa_pipeline::Pipeline;
use fa_rules::ParsedMessage;
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap()
}

const SUBJECT: &str = "Zebra crossing meeting";

#[derive(Default)]
struct StubState {
    reply: Value,
    requests: Vec<Value>,
}
type Stub = Arc<Mutex<StubState>>;

async fn chat(State(s): State<Stub>, Json(body): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.requests.push(body);
    Json(json!({
        "choices": [{"message": {"content": g.reply.to_string()}}],
        "usage": {"prompt_tokens": 11, "completion_tokens": 7}
    }))
}

async fn stub(reply: Value) -> (String, Stub) {
    let state: Stub = Arc::new(Mutex::new(StubState {
        reply,
        requests: vec![],
    }));
    let app = Router::new()
        .route("/v1/chat/completions", post(chat))
        .with_state(state.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, state)
}

async fn dead_url() -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", l.local_addr().unwrap());
    drop(l);
    url
}

struct NoTokens;

#[async_trait]
impl AccessTokenProvider for NoTokens {
    async fn access_token(&self, _: HouseholdId, _: OAuthAccountId) -> Result<String, TokenError> {
        Err(TokenError::NeedsReauth)
    }
}

struct Env {
    pool: PgPool,
    store: Store,
    h: HouseholdId,
    src: MailSourceId,
    deps: Arc<MailDeps>,
    svc: ProcessService,
}

async fn env(pool: PgPool, base_url: Option<String>) -> Env {
    let store = Store::from_pool(pool.clone());
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
                enabled: true,
            },
        )
        .await
        .unwrap();
    }
    Pipeline::ensure_defaults(&mut tx, h).await.unwrap();
    Pipeline::set_llm_stage(&mut tx, h, DecisionPointKey::EmailCategory, true, 0.7)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let provider = Arc::new(fa_llm::Provider::new(
        store.clone(),
        Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        fa_llm::default_http().unwrap(),
    ));
    let deps = Arc::new(MailDeps {
        store: store.clone(),
        http: fa_connectors::Http::new(false).unwrap(),
        tokens: Arc::new(NoTokens),
        key: Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        cfg: Default::default(),
        llm: Some(provider),
    });
    let svc = fa_mail::process::process_service(&deps).with_clock(now);
    Env {
        pool,
        store,
        h,
        src: src.id,
        deps,
        svc,
    }
}

fn fixture(name: &str) -> ParsedMessage {
    let raw = std::fs::read(format!(
        "{}/../fa-rules/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    fa_rules::parse(&raw).unwrap()
}

impl Env {
    async fn insert(&self, p: &ParsedMessage, remote: &str) -> MessageId {
        let attachments: Vec<_> = p
            .attachments
            .iter()
            .map(|a| {
                json!({"part": a.part, "filename": a.filename, "mime": a.mime,
                       "size": a.size, "ics": a.ics})
            })
            .collect();
        let m = NewMessage {
            household_id: self.h,
            source_id: self.src,
            remote_id: remote.into(),
            folder: Some("INBOX".into()),
            thread_id: None,
            rfc822_id: p.message_id.clone(),
            received_at: p.date.unwrap_or_else(now),
            from_addr: p.from_addr.clone(),
            from_name: p.from_name.clone(),
            to_addrs: p.to.clone(),
            cc_addrs: p.cc.clone(),
            subject: p.subject.clone(),
            gmail_labels: vec![],
            has_attachments: !attachments.is_empty(),
            body_text: Some(p.body_text.clone()),
            body_html: p.body_html.clone(),
            snippet: Some(p.body_text.chars().take(200).collect()),
            headers: Some(json!(p.headers)),
            attachments: Some(json!(attachments)),
        };
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        let id = MessageRepo::insert_parsed(&mut tx, &m)
            .await
            .unwrap()
            .expect("new message");
        tx.commit().await.unwrap();
        id
    }

    async fn plain(&self, remote: &str) -> MessageId {
        let p = ParsedMessage {
            from_addr: "noreply@example.com".into(),
            subject: SUBJECT.into(),
            body_text: "Please join us for a short chat about the crossing.".into(),
            date: Some(now()),
            ..Default::default()
        };
        self.insert(&p, remote).await
    }

    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await.unwrap()
    }

    async fn jobs(&self, kind: &str) -> Vec<Value> {
        sqlx::query_scalar("SELECT payload FROM job WHERE kind = $1 ORDER BY created_at")
            .bind(kind)
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }

    async fn run(&self, kind: &str, payload: Value) -> Result<(), fa_jobs::JobError> {
        let ctx = JobContext {
            store: self.store.clone(),
            backup_dir: None,
        };
        let hs = fa_mail::handlers(&self.deps);
        hs.iter()
            .find(|h| h.kind() == kind)
            .unwrap()
            .run(&ctx, payload)
            .await
    }

    async fn category(&self, id: MessageId) -> Option<String> {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        MessageRepo::get(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
            .category
    }

    async fn set_mode(&self, mode: MailExtractMode) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        AiSettingsRepo::update(&mut tx, self.h, mode, 12000)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
}

fn answer() -> Value {
    json!({
        "category": {"value": "school", "confidence": 0.9},
        "events": [{"title": "Zebra crossing chat", "start": "2026-11-05T15:00:00Z",
                    "end": "2026-11-05T16:00:00Z", "confidence": 0.85}],
        "bills": [], "tasks": []
    })
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pending_message_queues_one_job(pool: PgPool) {
    let (url, _) = stub(answer()).await;
    let e = env(pool, Some(url)).await;
    let id = e.plain("m1").await;
    e.svc.process(e.h, id, None).await.unwrap();
    let jobs = e.jobs("mail.llm").await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["message_id"], json!(id));
    e.svc.process(e.h, id, None).await.unwrap();
    assert_eq!(e.jobs("mail.llm").await.len(), 1);
    // Reprocessing keeps the default decision of the pending point.
    assert_eq!(
        e.count("SELECT count(*) FROM job WHERE kind = 'mail.llm' AND max_attempts = 12")
            .await,
        1
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn no_endpoint_no_job(pool: PgPool) {
    let e = env(pool, None).await;
    let id = e.plain("m1").await;
    e.svc.process(e.h, id, None).await.unwrap();
    assert!(e.jobs("mail.llm").await.is_empty());
    // A job that still runs completes without doing anything.
    e.run("mail.llm", json!({"message_id": id})).await.unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn job_records_decision_and_proposes_event(pool: PgPool) {
    let (url, state) = stub(answer()).await;
    let e = env(pool, Some(url)).await;
    let id = e.plain("m1").await;
    e.svc.process(e.h, id, None).await.unwrap();
    assert_ne!(e.category(id).await.as_deref(), Some("school"));
    let payload = e.jobs("mail.llm").await.remove(0);
    e.run("mail.llm", payload).await.unwrap();

    assert_eq!(e.category(id).await.as_deref(), Some("school"));
    assert_eq!(
        e.count(
            "SELECT count(*) FROM decision d JOIN decision_point p ON p.id = d.decision_point_id
             WHERE d.stage_kind = 'llm' AND p.key = 'email.category' AND d.is_final"
        )
        .await,
        1
    );
    // The behaviour step re-ran: auto actions queued under their own key.
    assert_eq!(e.jobs("mail.auto_actions").await.len(), 2);
    // One proposal, marked as LLM-made, on no calendar.
    assert_eq!(
        e.count("SELECT count(*) FROM event WHERE status = 'extracted' AND calendar_id IS NULL AND extracted_by = 'llm'")
            .await,
        1
    );
    assert_eq!(e.count("SELECT count(*) FROM event").await, 1);
    // One call, metrics only.
    assert_eq!(
        e.count("SELECT count(*) FROM llm_call WHERE ok AND purpose = 'mail.llm' AND model = 'test-model' AND prompt_tokens = 11")
            .await,
        1
    );
    let leaked: i64 = sqlx::query_scalar("SELECT count(*) FROM llm_call c WHERE c::text ILIKE $1")
        .bind(format!("%{SUBJECT}%"))
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(leaked, 0);
    {
        let g = state.lock().unwrap();
        assert_eq!(g.requests.len(), 1);
        let user = g.requests[0]["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains(SUBJECT));
    }
    // Running again asks nothing: the category is final and the rules were strong.
    e.run("mail.llm", json!({"message_id": id, "rules_strong": true}))
        .await
        .unwrap();
    assert_eq!(state.lock().unwrap().requests.len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn low_confidence_keeps_the_default(pool: PgPool) {
    let (url, _) = stub(json!({
        "category": {"value": "school", "confidence": 0.4},
        "events": [{"title": "Zebra crossing chat", "start": "2026-11-05T15:00:00Z", "confidence": 0.4}]
    }))
    .await;
    let e = env(pool, Some(url)).await;
    let id = e.plain("m1").await;
    e.svc.process(e.h, id, None).await.unwrap();
    let before = e.category(id).await;
    e.run("mail.llm", json!({"message_id": id})).await.unwrap();
    assert_eq!(e.category(id).await, before);
    assert_eq!(
        e.count("SELECT count(*) FROM decision WHERE stage_kind = 'llm'")
            .await,
        0
    );
    assert_eq!(e.count("SELECT count(*) FROM event").await, 0);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn unavailable_endpoint_fails_the_job_and_logs_the_call(pool: PgPool) {
    let e = env(pool, Some(dead_url().await)).await;
    let id = e.plain("m1").await;
    e.svc.process(e.h, id, None).await.unwrap();
    let r = e.run("mail.llm", json!({"message_id": id})).await;
    let err = r.unwrap_err().to_string();
    assert_eq!(err, "llm unavailable");
    assert_eq!(
        e.count("SELECT count(*) FROM llm_call WHERE NOT ok AND error_kind = 'unavailable'")
            .await,
        1
    );
    // The job runner re-queues with a later run_at (30 s x 2^attempts).
    let job = e.jobs("mail.llm").await.len();
    assert_eq!(job, 1);
    let mut tx = e.store.tx_system().await.unwrap();
    let j = loop {
        let j = JobRepo::claim(&mut tx, "t", 60).await.unwrap().unwrap();
        if j.kind == "mail.llm" {
            break j;
        }
    };
    assert_eq!(j.max_attempts, 12);
    JobRepo::fail(&mut tx, j.id, "llm unavailable")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        e.count("SELECT count(*) FROM job WHERE kind = 'mail.llm' AND status = 'queued' AND run_at > now()")
            .await,
        1
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn extract_modes(pool: PgPool) {
    let (url, state) = stub(json!({})).await;
    let e = env(pool, Some(url)).await;

    // Fallback: a rule extractor scored 0.95 (ICS) and the category is accepted: no job.
    let ics = e.insert(&fixture("teacher_ics.eml"), "ics1").await;
    e.svc.process(e.h, ics, None).await.unwrap();
    assert_eq!(e.category(ics).await.as_deref(), Some("school"));
    assert!(e.jobs("mail.llm").await.is_empty());

    // Always: the same message queues a job that asks for extraction.
    e.set_mode(MailExtractMode::Always).await;
    e.svc.process(e.h, ics, None).await.unwrap();
    let jobs = e.jobs("mail.llm").await;
    assert_eq!(jobs.len(), 1);
    e.run("mail.llm", jobs[0].clone()).await.unwrap();
    let sys = state.lock().unwrap().requests[0]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(sys.contains("Fields to fill: events, bills, tasks."));
    assert!(!sys.contains("category.value"));

    // Off: a pending message still gets its decision part, without extraction.
    e.set_mode(MailExtractMode::Off).await;
    let id = e.plain("m2").await;
    e.svc.process(e.h, id, None).await.unwrap();
    let jobs = e.jobs("mail.llm").await;
    assert_eq!(jobs.len(), 2);
    e.run("mail.llm", jobs[1].clone()).await.unwrap();
    let sys = state.lock().unwrap().requests[1]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(sys.contains("Fields to fill: category."));
    assert!(!sys.contains("events"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn prune_deletes_old_calls(pool: PgPool) {
    let e = env(pool, None).await;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    for days in [100, 91, 10, 0] {
        LlmCallRepo::insert(
            &mut tx,
            &NewLlmCall {
                household_id: e.h,
                purpose: "mail.llm".into(),
                model: "m".into(),
                started_at: Utc::now() - Duration::days(days),
                latency_ms: 5,
                ok: true,
                error_kind: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    e.run("llm.prune", json!({})).await.unwrap();
    assert_eq!(e.count("SELECT count(*) FROM llm_call").await, 2);
    assert!(fa_mail::default_schedules().contains(&("llm.prune", "0 4 * * *")));
}
