//! The `llm` stage, the `mail.llm` job and `llm.prune` over a real Postgres and an axum stub LLM.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
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
        .route(
            "/v1/models",
            get(|| async { Json(json!({"data": [{"id": "test-model"}]})) }),
        )
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
    // The M4 tests below run rules_first; the D-107 tests switch with `Env::settings`.
    AiSettingsRepo::update_all(
        &mut tx,
        h,
        &AiSettingsInput {
            mail_extract_mode: MailExtractMode::Fallback,
            max_input_chars: 12000,
            mail_mode: MailMode::RulesFirst,
            catchup_days: 30,
            extract_fallback_cutoff: 0.8,
        },
    )
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
        let hs = fa_mail::process::handlers_with_clock(&self.deps, now);
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

// ------------------------------------------------------------------ D-107: llm_first

/// Rows of `event`: (title, status, extracted_by), oldest first.
async fn events(e: &Env) -> Vec<(String, String, String)> {
    sqlx::query_as("SELECT title, status, extracted_by FROM event ORDER BY created_at, id")
        .fetch_all(&e.pool)
        .await
        .unwrap()
}

impl Env {
    async fn settings(&self, mode: MailMode, catchup_days: u32, cutoff: f32) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        AiSettingsRepo::update_all(
            &mut tx,
            self.h,
            &AiSettingsInput {
                mail_extract_mode: MailExtractMode::Fallback,
                max_input_chars: 12000,
                mail_mode: mode,
                catchup_days,
                extract_fallback_cutoff: cutoff,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn llm_state(&self, id: MessageId) -> String {
        sqlx::query_scalar("SELECT llm_state FROM message WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// `(stage_kind, is_final)` of the decisions of a point, oldest first.
    async fn decisions(&self, id: MessageId, dp: &str) -> Vec<(String, bool)> {
        sqlx::query_as(
            "SELECT d.stage_kind, d.is_final FROM decision d
             JOIN decision_point p ON p.id = d.decision_point_id
             WHERE d.subject_id = $1 AND p.key = $2 ORDER BY d.created_at, d.id",
        )
        .bind(id.as_uuid())
        .bind(dp)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn backlog(&self) -> LlmBacklog {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        LlmStateRepo::backlog(&mut tx, self.h).await.unwrap()
    }

    async fn set_endpoint(&self, url: &str, enabled: bool) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        LlmEndpointRepo::upsert(
            &mut tx,
            self.h,
            &LlmEndpointInput {
                base_url: url.into(),
                model: "test-model".into(),
                dialect: LlmDialect::OpenaiChat,
                timeout_ms: 5000,
                enabled,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        self.deps.llm.as_ref().unwrap().invalidate(self.h);
    }

    /// Runs every queued `mail.llm` job payload (in queue order).
    async fn run_llm_jobs(&self) {
        let payloads: Vec<Value> = sqlx::query_scalar(
            "SELECT payload FROM job WHERE kind = 'mail.llm' AND status = 'queued'
             ORDER BY priority DESC, run_at, created_at",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();
        for p in payloads {
            self.run("mail.llm", p).await.unwrap();
        }
    }
}

fn conference_answer(category: &str) -> Value {
    json!({
        "category": {"value": category, "confidence": 0.9},
        "events": [{"title": "Conference with Ms Rivera", "start": "2026-11-06T23:00:00Z",
                    "end": "2026-11-07T00:00:00Z", "location": "Room 12", "confidence": 0.9}],
        "bills": [], "tasks": []
    })
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_queues_every_message_even_when_rules_are_sure(pool: PgPool) {
    let (url, _) = stub(json!({})).await;
    let e = env(pool, Some(url)).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    // A confident ICS invite, a plain mail and a promotion: all of them go to the model.
    let ics = e.insert(&fixture("teacher_ics.eml"), "ics").await;
    let plain = e.plain("p").await;
    let promo = e.insert(&fixture("promo.eml"), "promo").await;
    for id in [ics, plain, promo] {
        e.svc.process(e.h, id, None).await.unwrap();
        assert_eq!(e.llm_state(id).await, "queued");
    }
    assert_eq!(e.category(ics).await.as_deref(), Some("school"));
    assert_eq!(e.jobs("mail.llm").await.len(), 3);
    // Reprocessing keeps one job per message and the state.
    e.svc.process(e.h, ics, None).await.unwrap();
    assert_eq!(e.jobs("mail.llm").await.len(), 3);
    assert_eq!(
        e.backlog().await,
        LlmBacklog {
            awaiting: 0,
            queued: 3
        }
    );
    // The rules' result applied at once: one proposal from the rule extractor.
    assert_eq!(
        events(&e).await,
        vec![(
            "Parent-teacher conference".into(),
            "extracted".into(),
            "rules".into()
        )]
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_replaces_rules_decisions_and_pending_proposals(pool: PgPool) {
    let (url, state) = stub(conference_answer("appointments")).await;
    let e = env(pool, Some(url)).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    let id = e.insert(&fixture("teacher_ics.eml"), "ics").await;
    e.svc.process(e.h, id, None).await.unwrap();
    assert_eq!(e.category(id).await.as_deref(), Some("school"));
    assert_eq!(
        e.decisions(id, "email.category").await,
        vec![("rules".to_string(), true)]
    );

    let payload = e.jobs("mail.llm").await.remove(0);
    e.run("mail.llm", payload).await.unwrap();

    // All four points and the extraction were asked.
    let sys = state.lock().unwrap().requests[0]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(sys.contains(
        "Fields to fill: category, time_sensitive, is_bill, member_link, events, bills, tasks."
    ));
    // The model's category replaced the rules' decision: both are on record, the model's last.
    assert_eq!(e.category(id).await.as_deref(), Some("appointments"));
    assert_eq!(
        e.decisions(id, "email.category").await,
        vec![("rules".to_string(), true), ("llm".to_string(), true)]
    );
    // The pending rule proposal took the model's title and location; nothing was added.
    assert_eq!(
        events(&e).await,
        vec![(
            "Conference with Ms Rivera".into(),
            "extracted".into(),
            "llm".into()
        )]
    );
    let loc: Option<String> = sqlx::query_scalar("SELECT location FROM event")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(loc.as_deref(), Some("Room 12"));
    assert_eq!(e.llm_state(id).await, "done");
    // A second run (a duplicate job) asks nothing.
    e.run("mail.llm", json!({"message_id": id})).await.unwrap();
    assert_eq!(state.lock().unwrap().requests.len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_leaves_what_people_decided_alone(pool: PgPool) {
    let (url, state) = stub(conference_answer("school")).await;
    let e = env(pool, Some(url)).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    let labelled = e.insert(&fixture("teacher_ics.eml"), "a").await;
    let rejected = e.insert(&fixture("teacher_ics.eml"), "b").await;
    for id in [labelled, rejected] {
        e.svc.process(e.h, id, None).await.unwrap();
    }
    assert_eq!(events(&e).await.len(), 2);

    // A person labels the first message (an extract category) and approves its event; the
    // second message's event is rejected.
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let subject = Subject {
            kind: SubjectType::Message,
            id: labelled.as_uuid(),
        };
        Pipeline::record_human(
            &mut tx,
            e.h,
            DecisionPointKey::EmailCategory,
            subject,
            json!("appointments"),
            None,
        )
        .await
        .unwrap();
        MessageRepo::set_classification(&mut tx, labelled, Some("appointments"), None, None)
            .await
            .unwrap();
        let rows: Vec<(uuid::Uuid, uuid::Uuid)> =
            sqlx::query_as("SELECT id, message_id FROM event ORDER BY created_at, id")
                .fetch_all(&mut *tx)
                .await
                .unwrap();
        for (ev, msg) in rows {
            let status = if msg == labelled.as_uuid() {
                EventStatus::Approved
            } else {
                EventStatus::Rejected
            };
            EventRepo::set_status(&mut tx, ev.into(), status, None)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
    }

    e.run_llm_jobs().await;

    // The label stands; the model was not asked for a category on the labelled message.
    assert_eq!(e.category(labelled).await.as_deref(), Some("appointments"));
    assert_eq!(
        e.decisions(labelled, "email.category").await,
        vec![("rules".to_string(), true), ("human".to_string(), true)]
    );
    let sys = state.lock().unwrap().requests[0]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!sys.contains("category.value"));
    // Approved and rejected events keep their titles and status; the model added nothing.
    let rows = events(&e).await;
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|r| r.0 == "Parent-teacher conference" && r.2 == "rules")
    );
    let mut statuses: Vec<_> = rows.iter().map(|r| r.1.as_str()).collect();
    statuses.sort_unstable();
    assert_eq!(statuses, ["approved", "rejected"]);
    // The unlabelled message still got the model's category.
    assert_eq!(e.llm_state(labelled).await, "done");
    assert_eq!(e.llm_state(rejected).await, "done");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_without_a_client_waits_and_enabling_catches_up(pool: PgPool) {
    let e = env(pool, None).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    let dated = |day_month: (u32, u32)| ParsedMessage {
        from_addr: "noreply@example.com".into(),
        subject: SUBJECT.into(),
        body_text: "Please join us for a short chat about the crossing.".into(),
        date: Some(
            Utc.with_ymd_and_hms(2026, day_month.1, day_month.0, 9, 0, 0)
                .unwrap(),
        ),
        ..Default::default()
    };
    // The clock is 2026-10-28: yesterday, ten days ago and forty days ago.
    let ids = [
        e.insert(&dated((27, 10)), "recent").await,
        e.insert(&dated((18, 10)), "mid").await,
        e.insert(&dated((18, 9)), "old").await,
    ];
    for id in ids {
        e.svc.process(e.h, id, None).await.unwrap();
        assert_eq!(e.llm_state(id).await, "awaiting");
    }
    assert!(e.jobs("mail.llm").await.is_empty());
    assert_eq!(
        e.backlog().await,
        LlmBacklog {
            awaiting: 3,
            queued: 0
        }
    );

    // Without an endpoint the catch-up does nothing.
    e.run("mail.llm.catchup", json!({"household": e.h}))
        .await
        .unwrap();
    assert!(e.jobs("mail.llm").await.is_empty());

    // An endpoint that does not answer: probed (and logged), nothing queued.
    e.set_endpoint(&dead_url().await, true).await;
    e.run("mail.llm.catchup", json!({"household": e.h}))
        .await
        .unwrap();
    assert!(e.jobs("mail.llm").await.is_empty());
    assert_eq!(
        e.count("SELECT count(*) FROM llm_call WHERE purpose = 'endpoint.test' AND NOT ok")
            .await,
        1
    );

    // A window of 0 days switches the catch-up off.
    let (url, state) = stub(answer()).await;
    e.set_endpoint(&url, true).await;
    e.settings(MailMode::LlmFirst, 0, 0.8).await;
    e.run("mail.llm.catchup", json!({"household": e.h}))
        .await
        .unwrap();
    assert!(e.jobs("mail.llm").await.is_empty());

    // The endpoint answers and the window is 30 days: the two recent messages, newest first;
    // the old one stays rules-only.
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    e.run("mail.llm.catchup", json!({"household": e.h}))
        .await
        .unwrap();
    let jobs: Vec<Value> = sqlx::query_scalar(
        "SELECT payload FROM job WHERE kind = 'mail.llm' ORDER BY priority DESC, run_at",
    )
    .fetch_all(&e.pool)
    .await
    .unwrap();
    let queued: Vec<Value> = jobs.iter().map(|j| j["message_id"].clone()).collect();
    assert_eq!(queued, vec![json!(ids[0]), json!(ids[1])]);
    assert_eq!(
        e.count("SELECT count(*) FROM job WHERE kind = 'mail.llm' AND priority = -1")
            .await,
        2
    );
    assert_eq!(e.llm_state(ids[2]).await, "awaiting");
    assert_eq!(
        e.backlog().await,
        LlmBacklog {
            awaiting: 1,
            queued: 2
        }
    );
    assert_eq!(
        e.count("SELECT count(*) FROM llm_call WHERE purpose = 'endpoint.test' AND ok")
            .await,
        1
    );
    // Running the schedule's variant (no household) finds nothing more within the window.
    e.run("mail.llm.catchup", json!({})).await.unwrap();
    assert_eq!(e.jobs("mail.llm").await.len(), 2);

    // The jobs run; those messages are done.
    e.run_llm_jobs().await;
    assert_eq!(e.llm_state(ids[0]).await, "done");
    assert_eq!(e.llm_state(ids[1]).await, "done");
    assert_eq!(state.lock().unwrap().requests.len(), 2);
    assert_eq!(
        e.backlog().await,
        LlmBacklog {
            awaiting: 1,
            queued: 0
        }
    );
    assert!(fa_mail::default_schedules().contains(&("mail.llm.catchup", "*/15 * * * *")));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn disabling_the_endpoint_parks_queued_messages(pool: PgPool) {
    let (url, state) = stub(answer()).await;
    let e = env(pool, Some(url.clone())).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    let id = e.plain("m").await;
    e.svc.process(e.h, id, None).await.unwrap();
    assert_eq!(e.llm_state(id).await, "queued");
    e.set_endpoint(&url, false).await;
    // The job completes without a call and the message waits instead of being dropped.
    e.run_llm_jobs().await;
    assert_eq!(e.llm_state(id).await, "awaiting");
    assert!(state.lock().unwrap().requests.is_empty());
    // Enabling it again and catching up brings the message back.
    e.set_endpoint(&url, true).await;
    e.run("mail.llm.catchup", json!({"household": e.h}))
        .await
        .unwrap();
    assert_eq!(e.llm_state(id).await, "queued");
    e.run_llm_jobs().await;
    assert_eq!(e.llm_state(id).await, "done");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn an_unreachable_endpoint_parks_the_message_on_the_last_attempt(pool: PgPool) {
    let e = env(pool, Some(dead_url().await)).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    let id = e.plain("m").await;
    e.svc.process(e.h, id, None).await.unwrap();
    // Claim the job as the worker would (attempt 1 of 12).
    {
        let mut tx = e.store.tx_system().await.unwrap();
        let j = loop {
            let j = JobRepo::claim(&mut tx, "t", 60).await.unwrap().unwrap();
            if j.kind == "mail.llm" {
                break j;
            }
        };
        assert_eq!((j.attempts, j.max_attempts), (1, 12));
        tx.commit().await.unwrap();
    }
    let payload = e.jobs("mail.llm").await.remove(0);
    let err = e.run("mail.llm", payload.clone()).await.unwrap_err();
    assert_eq!(err.to_string(), "llm unavailable");
    // Still retrying: not parked.
    assert_eq!(e.llm_state(id).await, "queued");
    sqlx::query("UPDATE job SET attempts = max_attempts WHERE kind = 'mail.llm'")
        .execute(&e.pool)
        .await
        .unwrap();
    e.run("mail.llm", payload).await.unwrap_err();
    assert_eq!(e.llm_state(id).await, "awaiting");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rules_first_uses_the_configurable_cutoff(pool: PgPool) {
    let (url, _) = stub(json!({})).await;
    let e = env(pool, Some(url)).await;
    // The ICS invite scores 0.95: strong at the default cutoff, so no job.
    let ics = e.insert(&fixture("teacher_ics.eml"), "ics").await;
    e.svc.process(e.h, ics, None).await.unwrap();
    assert!(e.jobs("mail.llm").await.is_empty());
    assert_eq!(e.llm_state(ics).await, "none");
    // A cutoff above that score makes the same message ask the model.
    e.settings(MailMode::RulesFirst, 30, 0.99).await;
    e.svc.process(e.h, ics, None).await.unwrap();
    assert_eq!(e.jobs("mail.llm").await.len(), 1);
    assert_eq!(e.llm_state(ics).await, "queued");
    // rules_first never touches what the rules decided: a final rules decision is not asked.
    let (ask_cat, _) = (e.decisions(ics, "email.category").await, ());
    assert_eq!(ask_cat.last(), Some(&("rules".to_string(), true)));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rules_threshold_is_a_new_pipeline_version(pool: PgPool) {
    let e = env(pool, None).await;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let dp = DecisionPointKey::EmailIsBill;
    let v1 = Pipeline::set_stages(&mut tx, e.h, dp, false, 0.7, None)
        .await
        .unwrap();
    // Nothing changed: the active version stays.
    assert_eq!(
        Pipeline::set_stages(&mut tx, e.h, dp, false, 0.7, Some(0.8))
            .await
            .unwrap(),
        v1
    );
    let v2 = Pipeline::set_stages(&mut tx, e.h, dp, false, 0.7, Some(0.9))
        .await
        .unwrap();
    assert_ne!(v1, v2);
    let point = DecisionPointRepo::get_by_key(&mut tx, dp)
        .await
        .unwrap()
        .unwrap();
    let (active, stages) = PipelineRepo::active_version(&mut tx, point.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, v2);
    let rules = stages.iter().find(|s| s.kind == StageKind::Rules).unwrap();
    assert_eq!(rules.accept_threshold, 0.9);
    // No llm stage was invented for a point that has none.
    assert!(stages.iter().all(|s| s.kind != StageKind::Llm));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_first_refines_a_pending_bill_but_not_an_edited_one(pool: PgPool) {
    let answer = json!({
        "category": {"value": "bills", "confidence": 0.9},
        "is_bill": {"value": true, "confidence": 0.9},
        "bills": [{"payee": "Example Power Co", "amount_cents": 5000, "currency": "USD",
                   "due": "2026-11-20", "confidence": 0.9}]
    });
    let (url, state) = stub(answer.clone()).await;
    let e = env(pool, Some(url)).await;
    e.settings(MailMode::LlmFirst, 30, 0.8).await;
    {
        let t = Utc::now();
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        OrgRepo::insert(
            &mut tx,
            &Org {
                id: OrgId::new(),
                household_id: e.h,
                kind: OrgKind::Biller,
                name: "Power Co".into(),
                domains: vec!["power.example.com".into()],
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
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    let mail = |remote: &str, amount: &str| ParsedMessage {
        from_addr: "billing@power.example.com".into(),
        subject: format!("Statement {remote}"),
        body_text: format!("Your statement is ready. Amount due: ${amount}"),
        date: Some(now()),
        ..Default::default()
    };
    let (a, b) = (
        e.insert(&mail("a", "50.00"), "a").await,
        e.insert(&mail("b", "75.00"), "b").await,
    );
    for id in [a, b] {
        e.svc.process(e.h, id, None).await.unwrap();
    }
    let sql = "SELECT payee, status, extracted_by FROM bill WHERE message_id = $1";
    let row = |id: MessageId| {
        let pool = e.pool.clone();
        async move {
            sqlx::query_as::<_, (String, String, String)>(sql)
                .bind(id.as_uuid())
                .fetch_all(&pool)
                .await
                .unwrap()
        }
    };
    let (ra, rb) = (row(a).await, row(b).await);
    assert_eq!((ra.len(), rb.len()), (1, 1));
    assert!(ra[0].1 == "proposed" && ra[0].2 == "rules");
    // A person edits the second message's bill (a version bump) before the model answers.
    sqlx::query("UPDATE bill SET version = version + 1 WHERE message_id = $1")
        .bind(b.as_uuid())
        .execute(&e.pool)
        .await
        .unwrap();
    let jobs = e.jobs("mail.llm").await;
    e.run("mail.llm", jobs[0].clone()).await.unwrap();
    state.lock().unwrap().reply["bills"][0]["amount_cents"] = json!(7500);
    e.run("mail.llm", jobs[1].clone()).await.unwrap();
    // The first bill took the model's payee; the edited one is untouched, and nothing was added.
    assert_eq!(
        row(a).await,
        vec![("Example Power Co".into(), "proposed".into(), "llm".into())]
    );
    assert_eq!(row(b).await, rb);
    assert_eq!(e.count("SELECT count(*) FROM bill").await, 2);
}
