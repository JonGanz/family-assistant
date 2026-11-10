use chrono::{Duration, NaiveDate, Utc};
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;

async fn seed(store: &Store, name: &str) -> HouseholdId {
    let id = HouseholdId::new();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, $2, 'UTC')")
        .bind(id.as_uuid())
        .bind(name)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

fn input(model: &str) -> LlmEndpointInput {
    LlmEndpointInput {
        base_url: "http://llm.example.com:11434/v1".into(),
        model: model.into(),
        dialect: LlmDialect::OpenaiChat,
        timeout_ms: 60_000,
        thinking: false,
        enabled: true,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn endpoint_upsert_get_and_rls(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (a, b) = (seed(&store, "A").await, seed(&store, "B").await);
    let mut tx = store.tx_for(a).await.unwrap();
    assert!(LlmEndpointRepo::get(&mut tx, a).await.unwrap().is_none());
    let first = LlmEndpointRepo::upsert(&mut tx, a, &input("m1"))
        .await
        .unwrap();
    let second = LlmEndpointRepo::upsert(&mut tx, a, &input("m2"))
        .await
        .unwrap();
    assert_eq!(first.id, second.id);
    let got = LlmEndpointRepo::get(&mut tx, a).await.unwrap().unwrap();
    assert_eq!(got.model, "m2");
    assert_eq!(got.timeout_ms, 60_000);
    assert!(got.enabled);
    tx.commit().await.unwrap();

    let mut tx = store.tx_for(b).await.unwrap();
    assert!(LlmEndpointRepo::get(&mut tx, a).await.unwrap().is_none());
    assert!(LlmEndpointRepo::get(&mut tx, b).await.unwrap().is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn api_key_secret_accepts_llm_endpoint_owner(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let key = MasterKey::from_bytes(&[5u8; 32], 1).unwrap();
    let mut tx = store.tx_for(h).await.unwrap();
    let ep = LlmEndpointRepo::upsert(&mut tx, h, &input("m"))
        .await
        .unwrap();
    let owner = ep.id.as_uuid();
    SecretRepo::put(
        &mut tx,
        &key,
        h,
        "llm_endpoint",
        owner,
        SecretKind::ApiKey,
        b"k",
        None,
    )
    .await
    .unwrap();
    let got = SecretRepo::get(&mut tx, &key, h, "llm_endpoint", owner, SecretKind::ApiKey)
        .await
        .unwrap();
    assert_eq!(got.as_deref(), Some(&b"k"[..]));
}

#[sqlx::test(migrations = "./migrations")]
async fn ai_settings_default_then_update(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let d = AiSettingsRepo::get_or_default(&mut tx, h).await.unwrap();
    assert_eq!(d.mail_extract_mode, MailExtractMode::Fallback);
    assert_eq!(d.max_input_chars, 12_000);
    let u = AiSettingsRepo::update(&mut tx, h, MailExtractMode::Always, 5000)
        .await
        .unwrap();
    assert_eq!(u.mail_extract_mode, MailExtractMode::Always);
    let g = AiSettingsRepo::get_or_default(&mut tx, h).await.unwrap();
    assert_eq!(
        (g.mail_extract_mode, g.max_input_chars),
        (MailExtractMode::Always, 5000)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn llm_call_insert_stats_prune(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let now = Utc::now();
    let mut tx = store.tx_for(h).await.unwrap();
    for (i, (ok, ms, age_days)) in [
        (true, 100, 0),
        (true, 300, 0),
        (false, 200, 0),
        (true, 50, 10),
    ]
    .into_iter()
    .enumerate()
    {
        LlmCallRepo::insert(
            &mut tx,
            &NewLlmCall {
                household_id: h,
                purpose: "mail.llm".into(),
                model: "m".into(),
                started_at: now - Duration::days(age_days) + Duration::seconds(i as i64),
                latency_ms: ms,
                ok,
                error_kind: (!ok).then(|| "timeout".to_string()),
                prompt_tokens: Some(10),
                completion_tokens: None,
            },
        )
        .await
        .unwrap();
    }
    let stats = LlmCallRepo::stats_since(&mut tx, h, now - Duration::days(1))
        .await
        .unwrap();
    assert_eq!(stats.len(), 1);
    let s = &stats[0];
    assert_eq!((s.total, s.ok, s.p50_ms), (3, 2, Some(200)));
    assert!(s.p95_ms.unwrap() >= 200);
    let pruned = LlmCallRepo::prune_before(&mut tx, now - Duration::days(5))
        .await
        .unwrap();
    assert_eq!(pruned, 1);
    let all = LlmCallRepo::stats_since(&mut tx, h, now - Duration::days(30))
        .await
        .unwrap();
    assert_eq!(all[0].total, 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn stage_counts_empty_without_decisions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let c = DecisionRepo::stage_counts_since(&mut tx, h, Utc::now() - Duration::days(1))
        .await
        .unwrap();
    assert!(c.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn extracted_by_round_trips(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();

    let start = Utc::now() + Duration::days(3);
    let ev = EventRepo::insert_extracted(
        &mut tx,
        &NewExtractedEvent {
            household_id: h,
            message_id: None,
            org_id: None,
            title: "Concert".into(),
            description: None,
            location: None,
            start_at: start,
            end_at: start + Duration::hours(1),
            all_day: false,
            start_date: None,
            end_date: None,
            source_tz: None,
            cancelled: false,
            confidence: 0.9,
            source_ref: None,
            dedupe_key: "concert".into(),
            remote_uid: None,
            members: vec![],
        },
    )
    .await
    .unwrap();
    let full = EventRepo::get_full(&mut tx, ev).await.unwrap().unwrap();
    assert_eq!(full.extracted_by, ExtractOrigin::Rules);
    ExtractedByRepo::set(
        &mut tx,
        ExtractTarget::Event,
        ev.as_uuid(),
        ExtractOrigin::Llm,
    )
    .await
    .unwrap();
    let full = EventRepo::get_full(&mut tx, ev).await.unwrap().unwrap();
    assert_eq!(full.extracted_by, ExtractOrigin::Llm);

    let due = NaiveDate::from_ymd_opt(2026, 11, 3);
    let bill = Bill {
        id: BillId::new(),
        household_id: h,
        message_id: None,
        org_id: None,
        payee: "Utility".into(),
        amount_minor: 1234,
        currency: "USD".into(),
        due_on: due,
        account_hint: None,
        statement_period: None,
        status: BillStatus::Proposed,
        confidence: 0.9,
        task_id: None,
        source_ref: serde_json::json!({}),
        dedupe_key: bill_dedupe_key("Utility", 1234, due),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    BillRepo::insert_if_new(&mut tx, &bill)
        .await
        .unwrap()
        .unwrap();
    let id = bill.id.as_uuid();
    assert_eq!(
        ExtractedByRepo::get(&mut tx, ExtractTarget::Bill, id)
            .await
            .unwrap(),
        ExtractOrigin::Rules
    );
    ExtractedByRepo::set(&mut tx, ExtractTarget::Bill, id, ExtractOrigin::Llm)
        .await
        .unwrap();
    assert_eq!(
        ExtractedByRepo::get(&mut tx, ExtractTarget::Bill, id)
            .await
            .unwrap(),
        ExtractOrigin::Llm
    );

    let list = TaskListRepo::insert(
        &mut tx,
        &TaskList {
            id: TaskListId::new(),
            household_id: h,
            name: "x".into(),
            kind: TaskListKind::General,
            completion_policy: ListCompletionPolicy::Assignee,
            child_can_add: ChildCanAdd::None,
            dashboard_add: false,
            calendar_default: CalendarDisplay::UntilDone,
            claim_ttl_minutes: None,
            visible_on_dashboard: true,
            color: None,
            sort: 0,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    let task = NewTaskRow {
        id: TaskId::new(),
        household_id: h,
        list_id: list.value.id,
        definition_id: None,
        weather_rule_id: None,
        title: "Reply".into(),
        notes: None,
        due_at: None,
        visible_from: None,
        status: TaskStatus::Suggested,
        origin: TaskOrigin::Email,
        source_ref: None,
        priority_flag: PriorityFlag::Normal,
        calendar_display: CalendarDisplay::Hidden,
        created_by: None,
    };
    TaskRepo::insert(&mut tx, &task, &[], &[])
        .await
        .unwrap()
        .unwrap();
    let f = TaskFilter {
        ids: Some(vec![task.id]),
        ..Default::default()
    };
    assert_eq!(
        TaskRepo::get_view_rows(&mut tx, &f).await.unwrap()[0].extracted_by,
        ExtractOrigin::Rules
    );
    ExtractedByRepo::set(
        &mut tx,
        ExtractTarget::Task,
        task.id.as_uuid(),
        ExtractOrigin::Llm,
    )
    .await
    .unwrap();
    assert_eq!(
        TaskRepo::get_view_rows(&mut tx, &f).await.unwrap()[0].extracted_by,
        ExtractOrigin::Llm
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_down_then_up(pool: PgPool) {
    let down = include_str!("../migrations/20261104000001_m4_llm.down.sql");
    let up = include_str!("../migrations/20261104000001_m4_llm.up.sql");
    for sql in [down, up] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM llm_call")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
