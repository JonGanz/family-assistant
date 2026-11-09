//! D-107 store pieces: the new AI settings, `message.llm_state`, the catch-up claim and the
//! guarded refinement of pending rule proposals.

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;

async fn seed(store: &Store) -> HouseholdId {
    let id = HouseholdId::new();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'A', 'UTC')")
        .bind(id.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

fn at(day: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, h, 0, 0).unwrap()
}

async fn source(store: &Store, h: HouseholdId) -> MailSourceId {
    let t = Utc::now();
    let s = MailSource {
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
        created_at: t,
        updated_at: t,
    };
    let mut tx = store.tx_for(h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &s, &[]).await.unwrap();
    tx.commit().await.unwrap();
    s.id
}

async fn message(
    store: &Store,
    h: HouseholdId,
    src: MailSourceId,
    remote: &str,
    when: DateTime<Utc>,
) -> MessageId {
    let m = NewMessage {
        household_id: h,
        source_id: src,
        remote_id: remote.into(),
        folder: Some("INBOX".into()),
        thread_id: None,
        rfc822_id: None,
        received_at: when,
        from_addr: "teacher@example.edu".into(),
        from_name: None,
        to_addrs: vec![],
        cc_addrs: vec![],
        subject: "Hello".into(),
        gmail_labels: vec![],
        has_attachments: false,
        body_text: Some("Body".into()),
        body_html: None,
        snippet: None,
        headers: None,
        attachments: None,
    };
    let mut tx = store.tx_for(h).await.unwrap();
    let id = MessageRepo::insert_parsed(&mut tx, &m)
        .await
        .unwrap()
        .unwrap();
    tx.commit().await.unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn settings_defaults_and_update_all(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let mut tx = store.tx_for(h).await.unwrap();
    let d = AiSettingsRepo::get_or_default(&mut tx, h).await.unwrap();
    assert_eq!(d.mail_mode, MailMode::LlmFirst);
    assert_eq!(d.catchup_days, 30);
    assert_eq!(d.extract_fallback_cutoff, 0.8);
    let u = AiSettingsRepo::update_all(
        &mut tx,
        h,
        &AiSettingsInput {
            mail_extract_mode: MailExtractMode::Always,
            max_input_chars: 5000,
            mail_mode: MailMode::RulesFirst,
            catchup_days: 7,
            extract_fallback_cutoff: 0.6,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        (u.mail_mode, u.catchup_days, u.extract_fallback_cutoff),
        (MailMode::RulesFirst, 7, 0.6)
    );
    // The two-field update leaves the D-107 columns alone.
    let u = AiSettingsRepo::update(&mut tx, h, MailExtractMode::Off, 6000)
        .await
        .unwrap();
    assert_eq!(
        (
            u.mail_extract_mode,
            u.max_input_chars,
            u.mail_mode,
            u.catchup_days
        ),
        (MailExtractMode::Off, 6000, MailMode::RulesFirst, 7)
    );
    assert!(
        AiSettingsRepo::update_all(
            &mut tx,
            h,
            &AiSettingsInput {
                mail_extract_mode: MailExtractMode::Off,
                max_input_chars: 6000,
                mail_mode: MailMode::LlmFirst,
                catchup_days: 400,
                extract_fallback_cutoff: 0.8,
            },
        )
        .await
        .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn llm_state_backlog_and_catchup_claim(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, other) = (seed(&store).await, seed(&store).await);
    let src = source(&store, h).await;
    let (old, mid, new) = (
        message(&store, h, src, "old", at(1, 9)).await,
        message(&store, h, src, "mid", at(10, 9)).await,
        message(&store, h, src, "new", at(20, 9)).await,
    );
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(
        LlmStateRepo::get(&mut tx, old).await.unwrap(),
        Some(LlmState::None)
    );
    for id in [old, mid, new] {
        assert!(
            LlmStateRepo::transition(&mut tx, id, &[LlmState::None], LlmState::Awaiting)
                .await
                .unwrap()
        );
    }
    // A message that is done stays done.
    LlmStateRepo::set(&mut tx, mid, LlmState::Done)
        .await
        .unwrap();
    assert!(
        !LlmStateRepo::transition(&mut tx, mid, &[LlmState::None], LlmState::Awaiting)
            .await
            .unwrap()
    );
    let b = LlmStateRepo::backlog(&mut tx, h).await.unwrap();
    assert_eq!((b.awaiting, b.queued), (2, 0));
    tx.commit().await.unwrap();

    // RLS: the other household sees nothing.
    let mut tx = store.tx_for(other).await.unwrap();
    assert_eq!(
        LlmStateRepo::backlog(&mut tx, other).await.unwrap(),
        LlmBacklog {
            awaiting: 0,
            queued: 0
        }
    );
    tx.rollback().await.unwrap();
    let mut sys = store.tx_system().await.unwrap();
    assert_eq!(
        LlmStateRepo::households_awaiting(&mut sys).await.unwrap(),
        vec![h]
    );
    sys.rollback().await.unwrap();

    // The window: only `new` is within it; `old` stays awaiting.
    let mut tx = store.tx_for(h).await.unwrap();
    let got = LlmStateRepo::claim_awaiting(&mut tx, h, at(5, 0), 10)
        .await
        .unwrap();
    assert_eq!(got, vec![new]);
    assert_eq!(
        LlmStateRepo::get(&mut tx, new).await.unwrap(),
        Some(LlmState::Queued)
    );
    assert_eq!(
        LlmStateRepo::get(&mut tx, old).await.unwrap(),
        Some(LlmState::Awaiting)
    );
    // Newest first, and the batch limit holds.
    LlmStateRepo::set(&mut tx, new, LlmState::Awaiting)
        .await
        .unwrap();
    let got = LlmStateRepo::claim_awaiting(&mut tx, h, at(1, 0), 2)
        .await
        .unwrap();
    assert_eq!(got, vec![new, old]);
    assert!(
        LlmStateRepo::claim_awaiting(&mut tx, h, at(1, 0), 2)
            .await
            .unwrap()
            .is_empty()
    );
    let b = LlmStateRepo::backlog(&mut tx, h).await.unwrap();
    assert_eq!((b.awaiting, b.queued), (0, 2));
}

#[sqlx::test(migrations = "./migrations")]
async fn running_attempt_reads_the_job_row(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let src = source(&store, h).await;
    let m = message(&store, h, src, "a", at(1, 9)).await;
    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(
        LlmStateRepo::running_attempt(&mut tx, m).await.unwrap(),
        None
    );
    let mut job = NewJob::new("mail.llm");
    job.household_id = Some(h);
    job.payload = serde_json::json!({"message_id": m, "household": h});
    job.max_attempts = 12;
    JobRepo::enqueue(&mut tx, &job).await.unwrap();
    let claimed = JobRepo::claim(&mut tx, "t", 60).await.unwrap().unwrap();
    assert_eq!(claimed.attempts, 1);
    assert_eq!(
        LlmStateRepo::running_attempt(&mut tx, m).await.unwrap(),
        Some((1, 12))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn refine_touches_only_untouched_rule_proposals(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let src = source(&store, h).await;
    let m = message(&store, h, src, "a", at(1, 9)).await;
    let mut tx = store.tx_for(h).await.unwrap();

    // Events: two proposals, one of them approved.
    let mut ids = Vec::new();
    for (i, title) in ["Conference", "Concert"].into_iter().enumerate() {
        let start = at(20 + i as u32, 15);
        ids.push(
            EventRepo::insert_extracted(
                &mut tx,
                &NewExtractedEvent {
                    household_id: h,
                    message_id: Some(m),
                    org_id: None,
                    title: title.into(),
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
                    dedupe_key: format!("{title}|k"),
                    remote_uid: None,
                    members: vec![],
                },
            )
            .await
            .unwrap(),
        );
    }
    EventRepo::set_status(&mut tx, ids[1], EventStatus::Approved, None)
        .await
        .unwrap();
    let pending = ProposalRefineRepo::events(&mut tx, m).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, ids[0].as_uuid());
    assert!(
        ProposalRefineRepo::refine_event(
            &mut tx,
            ids[0].as_uuid(),
            "Parent-teacher conference",
            Some("Room 12"),
            0.95,
            "ptc|k"
        )
        .await
        .unwrap()
    );
    let e = EventRepo::get_full(&mut tx, ids[0]).await.unwrap().unwrap();
    assert_eq!(e.event.title, "Parent-teacher conference");
    assert_eq!(e.event.location.as_deref(), Some("Room 12"));
    assert_eq!(e.extracted_by, ExtractOrigin::Llm);
    // Already the LLM's, and an approved one: both refuse.
    for id in [ids[0], ids[1]] {
        assert!(
            !ProposalRefineRepo::refine_event(&mut tx, id.as_uuid(), "X", None, 0.9, "x")
                .await
                .unwrap()
        );
    }
    assert!(
        ProposalRefineRepo::events(&mut tx, m)
            .await
            .unwrap()
            .is_empty()
    );

    // Bills: proposed and untouched; an edit (version bump) or an open bill refuses.
    let due = NaiveDate::from_ymd_opt(2026, 11, 3);
    let mk = |payee: &str, amount: i64, status: BillStatus| Bill {
        id: BillId::new(),
        household_id: h,
        message_id: Some(m),
        org_id: None,
        payee: payee.into(),
        amount_minor: amount,
        currency: "USD".into(),
        due_on: due,
        account_hint: None,
        statement_period: None,
        status,
        confidence: 0.6,
        task_id: None,
        source_ref: serde_json::json!({}),
        dedupe_key: bill_dedupe_key(payee, amount, due),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let (b1, b2) = (
        mk("Power co", 1000, BillStatus::Proposed),
        mk("Water co", 2000, BillStatus::Open),
    );
    for b in [&b1, &b2] {
        BillRepo::insert_if_new(&mut tx, b).await.unwrap().unwrap();
    }
    let pending = ProposalRefineRepo::bills(&mut tx, m).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].amount_minor, 1000);
    assert!(
        ProposalRefineRepo::refine_bill(
            &mut tx,
            b1.id.as_uuid(),
            "Example Power",
            None,
            0.9,
            &bill_dedupe_key("Example Power", 1000, due)
        )
        .await
        .unwrap()
    );
    assert!(
        !ProposalRefineRepo::refine_bill(&mut tx, b2.id.as_uuid(), "X", None, 0.9, "x")
            .await
            .unwrap()
    );

    // Tasks: suggested and untouched.
    let list = TaskListRepo::insert(
        &mut tx,
        &TaskList {
            id: TaskListId::new(),
            household_id: h,
            name: "Email".into(),
            kind: TaskListKind::Email,
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
    let task = |title: &str, status: TaskStatus| NewTaskRow {
        id: TaskId::new(),
        household_id: h,
        list_id: list.value.id,
        definition_id: None,
        weather_rule_id: None,
        title: title.into(),
        notes: None,
        due_at: None,
        visible_from: None,
        status,
        origin: TaskOrigin::Email,
        source_ref: Some(serde_json::json!({
            "kind": "email", "message_id": m.to_string(), "mail_task_kind": "action"
        })),
        priority_flag: PriorityFlag::Normal,
        calendar_display: CalendarDisplay::Hidden,
        created_by: None,
    };
    let (t1, t2) = (
        task("Return the slip", TaskStatus::Suggested),
        task("Pay the fee", TaskStatus::Open),
    );
    for t in [&t1, &t2] {
        TaskRepo::insert(&mut tx, t, &[], &[])
            .await
            .unwrap()
            .unwrap();
    }
    let pending = ProposalRefineRepo::tasks(&mut tx, m).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(
        (pending[0].id, pending[0].kind.as_str()),
        (t1.id.as_uuid(), "action")
    );
    let due_at = at(25, 23);
    assert!(
        ProposalRefineRepo::refine_task(
            &mut tx,
            t1.id.as_uuid(),
            "Sign and return the permission slip",
            Some(due_at),
            "sign"
        )
        .await
        .unwrap()
    );
    assert!(
        !ProposalRefineRepo::refine_task(&mut tx, t2.id.as_uuid(), "X", None, "other")
            .await
            .unwrap()
    );
    assert!(
        !ProposalRefineRepo::refine_task(&mut tx, t1.id.as_uuid(), "Y", None, "other")
            .await
            .unwrap()
    );
    let f = TaskFilter {
        ids: Some(vec![t1.id]),
        ..Default::default()
    };
    let row = &TaskRepo::get_view_rows(&mut tx, &f).await.unwrap()[0];
    assert_eq!(row.extracted_by, ExtractOrigin::Llm);
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_down_then_up(pool: PgPool) {
    let down = include_str!("../migrations/20261107000001_llm_first.down.sql");
    let up = include_str!("../migrations/20261107000001_llm_first.up.sql");
    for sql in [down, up] {
        sqlx::raw_sql(sql).execute(&pool).await.unwrap();
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM message WHERE llm_state <> 'none'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_reset_forgets_mail_and_keeps_written_events(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let other = seed(&store).await;
    let src = source(&store, h).await;
    let other_src = source(&store, other).await;
    let m1 = message(&store, h, src, "1", at(1, 9)).await;
    let _m2 = message(&store, h, src, "2", at(2, 9)).await;
    let keep_other = message(&store, other, other_src, "1", at(1, 9)).await;

    let mut tx = store.tx_system().await.unwrap();
    sqlx::query(
        "INSERT INTO event (id, household_id, title, start_at, end_at, origin, status, dedupe_key, message_id)
         SELECT gen_random_uuid(), $1, s, now(), now(), 'extracted', s, s, $2
           FROM unnest(ARRAY['proposed', 'rejected', 'written', 'approved']) s",
    )
    .bind(h.as_uuid())
    .bind(m1.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event (id, household_id, title, start_at, end_at, origin, status, dedupe_key)
         VALUES (gen_random_uuid(), $1, 'Manual', now(), now(), 'manual', 'written', 'm')",
    )
    .bind(h.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO bill (id, household_id, message_id, payee, amount_minor, currency, status, confidence, dedupe_key)
         VALUES (gen_random_uuid(), $1, $2, 'Water', 100, 'USD', 'open', 1, 'b')",
    )
    .bind(h.as_uuid())
    .bind(m1.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mail_folder_state (source_id, folder, household_id, uidvalidity, uidnext)
         VALUES ($1, 'INBOX', $2, 1, 10)",
    )
    .bind(src.as_uuid())
    .bind(h.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE mail_source SET history_id = '42', last_synced_at = now(), state = 'error' WHERE id = $1")
        .bind(src.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    for (kind, status, hh) in [
        ("mail.process", "queued", h),
        ("mail.llm", "dead", h),
        ("mail.sync", "queued", h),
        ("mail.process", "queued", other),
    ] {
        sqlx::query(
            "INSERT INTO job (id, household_id, kind, status) VALUES (gen_random_uuid(), $1, $2, $3)",
        )
        .bind(hh.as_uuid())
        .bind(kind)
        .bind(status)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    // A dry run (rolled back) reports the same counts and changes nothing.
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(MailResetRepo::running_jobs(&mut tx, h).await.unwrap(), 0);
    let dry = MailResetRepo::reset(&mut tx, h).await.unwrap();
    tx.rollback().await.unwrap();

    let mut tx = store.tx_for(h).await.unwrap();
    let c = MailResetRepo::reset(&mut tx, h).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(dry, c);
    assert_eq!(
        (
            c.messages,
            c.events_removed,
            c.events_kept,
            c.bills,
            c.jobs,
            c.folder_cursors,
            c.sources_rewound
        ),
        (2, 2, 2, 1, 2, 1, 1)
    );

    let mut tx = store.tx_system().await.unwrap();
    let q = |sql: &'static str| sqlx::query_scalar::<_, i64>(sql);
    let msgs: i64 = q("SELECT count(*) FROM message")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(msgs, 1, "only the other household's message is left");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM message WHERE id = $1")
        .bind(keep_other.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(left, 1);
    let events: Vec<(String, Option<String>, Option<uuid::Uuid>)> = sqlx::query_as(
        "SELECT origin, status, message_id FROM event WHERE household_id = $1 ORDER BY title",
    )
    .bind(h.as_uuid())
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        events,
        vec![
            ("extracted".into(), Some("approved".into()), None),
            ("manual".into(), Some("written".into()), None),
            ("extracted".into(), Some("written".into()), None),
        ]
    );
    let (hist, synced, state): (Option<String>, Option<DateTime<Utc>>, String) =
        sqlx::query_as("SELECT history_id, last_synced_at, state FROM mail_source WHERE id = $1")
            .bind(src.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!((hist, synced, state.as_str()), (None, None, "ok"));
    let jobs: Vec<(String, uuid::Uuid)> =
        sqlx::query_as("SELECT kind, household_id FROM job ORDER BY kind")
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    assert_eq!(
        jobs,
        vec![
            ("mail.process".into(), other.as_uuid()),
            ("mail.sync".into(), h.as_uuid()),
        ]
    );
}
