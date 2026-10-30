use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use fa_domain::api::HouseholdPatch;
use fa_domain::*;
use fa_store::*;
use sqlx::{Executor, PgPool};

fn at(day: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, h, 0, 0).unwrap()
}

fn tm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn household(name: &str) -> Household {
    Household {
        id: HouseholdId::new(),
        name: name.into(),
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
        quiet_from: tm(21, 0),
        quiet_until: tm(7, 0),
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
    }
}

fn member(h: HouseholdId, name: &str, role: Role) -> Member {
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

struct Seed {
    h: HouseholdId,
    adult: MemberId,
    adult2: MemberId,
    child: MemberId,
}

async fn seed(store: &Store, name: &str) -> Seed {
    let hh = household(name);
    let adult = member(hh.id, "Ada Example", Role::Adult);
    let adult2 = member(hh.id, "Bo Example", Role::Adult);
    let child = member(hh.id, "Kit Example", Role::Child);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    for m in [&adult, &adult2, &child] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
    }
    tx.commit().await.unwrap();
    Seed {
        h: hh.id,
        adult: adult.id,
        adult2: adult2.id,
        child: child.id,
    }
}

fn source(h: HouseholdId, name: &str) -> MailSource {
    MailSource {
        id: MailSourceId::new(),
        household_id: h,
        provider: MailProvider::Imap,
        name: name.into(),
        address: "Parent@Example.com".into(),
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn new_msg(
    h: HouseholdId,
    src: MailSourceId,
    remote: &str,
    subject: &str,
    when: DateTime<Utc>,
) -> NewMessage {
    NewMessage {
        household_id: h,
        source_id: src,
        remote_id: remote.into(),
        folder: Some("INBOX".into()),
        thread_id: None,
        rfc822_id: Some(format!("<{remote}@example.com>")),
        received_at: when,
        from_addr: "teacher@example.edu".into(),
        from_name: Some("Teacher".into()),
        to_addrs: vec!["parent@example.com".into()],
        cc_addrs: vec![],
        subject: subject.into(),
        gmail_labels: vec![],
        has_attachments: false,
        body_text: Some("Body text".into()),
        body_html: Some("<p>Body</p>".into()),
        snippet: Some("Body".into()),
        headers: Some(serde_json::json!({"list-id": "x"})),
        attachments: Some(
            serde_json::json!([{"part": "2", "filename": "a.ics", "mime": "text/calendar",
                                              "size": 10, "ics": "BEGIN:VCALENDAR"}]),
        ),
    }
}

async fn add_source(store: &Store, h: HouseholdId, owners: &[MemberId]) -> MailSourceId {
    let s = source(h, "Parent mail");
    let mut tx = store.tx_for(h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &s, owners).await.unwrap();
    tx.commit().await.unwrap();
    s.id
}

async fn add_msg(
    store: &Store,
    h: HouseholdId,
    src: MailSourceId,
    remote: &str,
    category: Option<&str>,
    when: DateTime<Utc>,
) -> MessageId {
    let mut tx = store.tx_for(h).await.unwrap();
    let id = MessageRepo::insert_parsed(&mut tx, &new_msg(h, src, remote, remote, when))
        .await
        .unwrap()
        .unwrap();
    MessageRepo::set_classification(&mut tx, id, category, None, None)
        .await
        .unwrap();
    MessageRepo::set_state(&mut tx, id, ProcessingState::Processed)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

fn org(h: HouseholdId, name: &str) -> Org {
    Org {
        id: OrgId::new(),
        household_id: h,
        kind: OrgKind::School,
        name: name.into(),
        domains: vec!["example.edu".into()],
        senders: vec![],
        ics_calendar_id: None,
        default_calendar_id: None,
        contact_mode: ContactMode::Suggest,
        bill_amount_regex: None,
        bill_due_regex: None,
        bill_account_regex: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn bill(h: HouseholdId, payee: &str, amount: i64, status: BillStatus) -> Bill {
    let due = NaiveDate::from_ymd_opt(2026, 11, 3);
    Bill {
        id: BillId::new(),
        household_id: h,
        message_id: None,
        org_id: None,
        payee: payee.into(),
        amount_minor: amount,
        currency: "USD".into(),
        due_on: due,
        account_hint: None,
        statement_period: None,
        status,
        confidence: 0.9,
        task_id: None,
        source_ref: serde_json::json!({}),
        dedupe_key: bill_dedupe_key(payee, amount, due),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn extracted(h: HouseholdId, title: &str, start: DateTime<Utc>) -> NewExtractedEvent {
    NewExtractedEvent {
        household_id: h,
        message_id: None,
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
        confidence: 0.95,
        source_ref: Some(serde_json::json!({"kind": "email"})),
        dedupe_key: format!("{title}|{start}"),
        remote_uid: Some(format!("uid-{title}")),
        members: vec![],
    }
}

fn email_task_row(
    h: HouseholdId,
    list: TaskListId,
    message: MessageId,
    status: TaskStatus,
) -> NewTaskRow {
    NewTaskRow {
        id: TaskId::new(),
        household_id: h,
        list_id: list,
        definition_id: None,
        weather_rule_id: None,
        title: "Reply to the teacher".into(),
        notes: None,
        due_at: None,
        visible_from: None,
        status,
        origin: TaskOrigin::Email,
        source_ref: Some(serde_json::json!({"kind": "email", "message_id": message.to_string()})),
        priority_flag: PriorityFlag::Normal,
        calendar_display: CalendarDisplay::Hidden,
        created_by: None,
    }
}

fn list_defaults(h: HouseholdId) -> TaskList {
    TaskList {
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
    }
}

async fn count(tx: &mut Tx, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

const TABLES: &[&str] = &[
    "org",
    "org_contact",
    "enrollment",
    "enrollment_contact",
    "mail_source",
    "mail_source_owner",
    "mail_folder_state",
    "message",
    "message_member",
    "contact_suggestion",
    "decision_point",
    "pipeline_version",
    "pipeline_stage",
    "decision",
    "mail_rule",
    "category",
    "bill",
    "mail_action",
    "event_message",
];

#[sqlx::test(migrations = "./migrations")]
async fn migration_reverts_reapplies_and_seeds_existing_households(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let down = include_str!("../migrations/20261027000001_m3_ingest.down.sql");
    let up = include_str!("../migrations/20261027000001_m3_ingest.up.sql");
    let n = |pool: PgPool| async move {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public'
             AND table_name IN ('org','message','bill','decision','mail_source','category','event_message')",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    assert_eq!(n(pool.clone()).await, 7);
    // Rows that use the new values are removed by the down migration.
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Concert", at(30, 18)))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    pool.execute(down).await.unwrap();
    assert_eq!(n(pool.clone()).await, 0);
    let cols: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns
         WHERE table_name = 'household' AND column_name IN ('currency', 'bill_lead_days')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cols, 0);
    // A household that exists before the migration is seeded by it.
    let old = HouseholdId::new();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'Old', 'UTC')")
        .bind(old.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    pool.execute(up).await.unwrap();
    assert_eq!(n(pool.clone()).await, 7);
    let mut tx = store.tx_for(old).await.unwrap();
    assert_eq!(count(&mut tx, "category").await, 12);
    assert_eq!(count(&mut tx, "decision_point").await, 5);
    assert_eq!(count(&mut tx, "pipeline_version").await, 5);
    assert_eq!(count(&mut tx, "pipeline_stage").await, 10);
    // ensure() after the migration changes nothing.
    mail_defaults::ensure(&mut tx, old).await.unwrap();
    assert_eq!(count(&mut tx, "category").await, 12);
    assert_eq!(count(&mut tx, "pipeline_stage").await, 10);
}

#[sqlx::test(migrations = "./migrations")]
async fn seed_is_idempotent_and_complete(pool: PgPool) {
    let store = Store::from_pool(pool);
    let a = seed(&store, "A").await;
    let b = seed(&store, "B").await;
    let mut tx = store.tx_for(a.h).await.unwrap();
    assert!(CategoryRepo::list(&mut tx, true).await.unwrap().is_empty());
    for _ in 0..2 {
        mail_defaults::ensure(&mut tx, a.h).await.unwrap();
    }
    let cats = CategoryRepo::list(&mut tx, false).await.unwrap();
    assert_eq!(cats.len(), 12);
    let keys: Vec<_> = cats.iter().map(|c| c.value.key.as_str()).collect();
    assert_eq!(keys[0], "school");
    assert_eq!(keys[11], "other");
    assert_eq!(cats[0].value.behaviour, CategoryBehaviour::Extract);
    assert!(cats[0].value.notify_time_sensitive);
    assert_eq!(cats[0].value.auto_actions, vec![AutoAction::Label]);
    assert_eq!(cats[0].value.email_tasks, EmailTasksMode::Suggest);
    assert_eq!(cats[8].value.behaviour, CategoryBehaviour::Ignore);
    let dps = DecisionPointRepo::list(&mut tx).await.unwrap();
    assert_eq!(dps.len(), 5);
    for dp in &dps {
        assert_eq!(dp.default_value, dp.key.default_value());
        let (v, stages) = PipelineRepo::active_version(&mut tx, dp.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((v.version, v.name.as_str()), (1, "Rules only"));
        assert_eq!(stages.len(), 2);
        assert_eq!(
            (stages[0].kind, stages[0].accept_threshold),
            (StageKind::Rules, 0.8)
        );
        assert_eq!(
            (stages[1].kind, stages[1].accept_threshold),
            (StageKind::Human, 0.0)
        );
        assert_eq!(
            PipelineRepo::versions(&mut tx, dp.id).await.unwrap().len(),
            1
        );
    }
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert_eq!(count(&mut tx, "category").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn rls_isolates_every_m3_table(pool: PgPool) {
    let store = Store::from_pool(pool);
    let a = seed(&store, "A").await;
    let b = seed(&store, "B").await;
    let src = add_source(&store, a.h, &[a.adult]).await;
    let msg = add_msg(&store, a.h, src, "m1", Some("school"), at(20, 9)).await;

    let mut tx = store.tx_for(a.h).await.unwrap();
    mail_defaults::ensure(&mut tx, a.h).await.unwrap();
    let o = OrgRepo::insert(&mut tx, &org(a.h, "Example School"))
        .await
        .unwrap();
    let c = OrgContactRepo::insert(
        &mut tx,
        &OrgContact {
            id: OrgContactId::new(),
            household_id: a.h,
            org_id: o.value.id,
            name: "Teacher".into(),
            email: "Teacher@Example.edu".into(),
            role: ContactRole::Teacher,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    assert_eq!(c.value.email, "teacher@example.edu");
    let e = EnrollmentRepo::insert(
        &mut tx,
        &Enrollment {
            id: EnrollmentId::new(),
            household_id: a.h,
            member_id: a.child,
            org_id: o.value.id,
            grade: Some("3".into()),
            class_name: None,
            from_date: None,
            until_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    EnrollmentRepo::set_contacts(&mut tx, e.value.id, &[c.value.id])
        .await
        .unwrap();
    MailFolderStateRepo::upsert(
        &mut tx,
        &MailFolderState {
            source_id: src,
            folder: "INBOX".into(),
            uidvalidity: 7,
            uidnext: 10,
            highestmodseq: None,
        },
    )
    .await
    .unwrap();
    MessageRepo::set_members(&mut tx, msg, &[(a.child, DecidedBy::Rule, 0.9)])
        .await
        .unwrap();
    assert!(
        ContactSuggestionRepo::insert_if_new(
            &mut tx,
            &ContactSuggestion {
                id: ContactSuggestionId::new(),
                household_id: a.h,
                org_id: o.value.id,
                email: "coach@example.edu".into(),
                name: None,
                member_id: None,
                message_id: Some(msg),
                status: SuggestionStatus::Proposed,
                decided_by: None,
                decided_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        )
        .await
        .unwrap()
    );
    let dp = DecisionPointRepo::get_by_key(&mut tx, DecisionPointKey::EmailCategory)
        .await
        .unwrap()
        .unwrap();
    DecisionRepo::insert(
        &mut tx,
        &NewDecision {
            household_id: a.h,
            decision_point_id: dp.id,
            subject: Subject::message(msg),
            pipeline_version_id: None,
            stage_kind: StageKind::Rules,
            value: serde_json::json!("school"),
            confidence: 0.9,
            is_final: true,
            source: Some("org_domain".into()),
            decided_by: None,
        },
    )
    .await
    .unwrap();
    MailRuleRepo::insert(
        &mut tx,
        &MailRule {
            id: MailRuleId::new(),
            household_id: a.h,
            decision_point: DecisionPointKey::EmailCategory,
            name: "PTA".into(),
            priority: 10,
            conditions: vec![RuleCondition {
                field: RuleField::FromDomain,
                op: RuleOp::Is,
                header: None,
                values: vec!["example.edu".into()],
            }],
            value: serde_json::json!("school"),
            confidence: 0.9,
            enabled: true,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    BillRepo::insert_if_new(&mut tx, &bill(a.h, "Example Power", 1000, BillStatus::Open))
        .await
        .unwrap()
        .unwrap();
    MailActionRepo::insert(
        &mut tx,
        &NewMailAction {
            household_id: a.h,
            message_id: msg,
            action: MailActionKind::Label,
            label: Some("FA/School".into()),
            automatic: true,
            done_by: None,
            error: None,
        },
    )
    .await
    .unwrap();
    let ev = EventRepo::insert_extracted(&mut tx, &extracted(a.h, "Concert", at(30, 18)))
        .await
        .unwrap();
    EventRepo::link_message(&mut tx, ev, msg).await.unwrap();
    for t in TABLES {
        assert!(count(&mut tx, t).await > 0, "{t} should have rows for A");
    }
    tx.commit().await.unwrap();

    let mut tx = store.tx_for(b.h).await.unwrap();
    for t in TABLES {
        assert_eq!(count(&mut tx, t).await, 0, "{t} leaks across households");
    }
    assert!(OrgRepo::list(&mut tx, true).await.unwrap().is_empty());
    assert!(
        MailSourceRepo::list(&mut tx, true)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(EventRepo::proposals(&mut tx).await.unwrap().is_empty());
    tx.commit().await.unwrap();

    // The tenant policy also blocks writing another household's rows.
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert!(OrgRepo::insert(&mut tx, &org(a.h, "Sneaky")).await.is_err());
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert!(
        MessageRepo::insert_parsed(&mut tx, &new_msg(a.h, src, "evil", "x", at(1, 1)))
            .await
            .is_err()
    );
    // decision rows cannot be updated by the app role
    let mut tx = store.tx_for(a.h).await.unwrap();
    let r = sqlx::query("UPDATE decision SET confidence = 0")
        .execute(&mut *tx)
        .await;
    assert!(r.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn household_member_and_pref_columns(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let h = HouseholdRepo::get(&mut tx, s.h).await.unwrap().unwrap();
    assert_eq!(h.value.currency, "USD");
    assert_eq!(h.value.gender_terms, GenderTerms::default());
    assert_eq!(h.value.events, EventsSettings::default());
    assert_eq!(h.value.bill_lead_days, 3);
    assert!(h.value.disabled_signals.is_empty());
    let patch: HouseholdPatch = serde_json::from_value(serde_json::json!({
        "currency": "EUR", "bill_lead_days": 5,
        "events": {"auto_write": true, "extract_threshold": 0.8, "route_threshold": 0.7},
        "inbox": {"task_expiry_days": 30, "contact_expiry_days": 10},
        "retention_mail": {"content_policy": "keep_all", "examples_per_category": 50, "max_pending_days": 3},
        "gender_terms": {"female": ["girls"], "male": ["boys"]}
    }))
    .unwrap();
    let v = HouseholdRepo::update_settings(&mut tx, s.h, &patch, h.version)
        .await
        .unwrap();
    assert_eq!(v, h.version + 1);
    HouseholdRepo::set_disabled_signals(&mut tx, s.h, &["gmail_spam".to_string()])
        .await
        .unwrap();
    assert_eq!(
        HouseholdRepo::disabled_signals(&mut tx, s.h).await.unwrap(),
        vec!["gmail_spam"]
    );
    let h2 = HouseholdRepo::get(&mut tx, s.h).await.unwrap().unwrap();
    assert_eq!(h2.value.currency, "EUR");
    assert_eq!(h2.value.bill_lead_days, 5);
    assert!(h2.value.events.auto_write && h2.value.events.extract_threshold == 0.8);
    assert_eq!(h2.value.inbox.task_expiry_days, 30);
    assert_eq!(
        h2.value.retention_mail.content_policy,
        ContentPolicy::KeepAll
    );
    assert_eq!(h2.value.gender_terms.female, vec!["girls"]);
    assert_eq!(h2.value.disabled_signals, vec!["gmail_spam"]);
    // the full update writes the new columns too
    let mut full = h2.value.clone();
    full.currency = "GBP".into();
    full.bill_lead_days = 7;
    let v = HouseholdRepo::update(&mut tx, &full, h2.version)
        .await
        .unwrap();
    assert!(v > h2.version);
    let h3 = HouseholdRepo::get(&mut tx, s.h).await.unwrap().unwrap();
    assert_eq!(
        (h3.value.currency.as_str(), h3.value.bill_lead_days),
        ("GBP", 7)
    );
    assert_eq!(h3.value.disabled_signals, vec!["gmail_spam"]);

    let m = MemberRepo::get(&mut tx, s.child).await.unwrap().unwrap();
    let mut edited = m.value.clone();
    edited.aliases = vec!["Kitty".into()];
    edited.gender = Some(Gender::Female);
    edited.relevance_tags = vec!["soccer".into()];
    MemberRepo::update(&mut tx, &edited, m.version)
        .await
        .unwrap();
    let m2 = MemberRepo::get(&mut tx, s.child).await.unwrap().unwrap();
    assert_eq!(m2.value.aliases, vec!["Kitty"]);
    assert_eq!(m2.value.gender, Some(Gender::Female));
    assert_eq!(m2.value.relevance_tags, vec!["soccer"]);
    let mut fresh = member(s.h, "Cy Example", Role::Child);
    fresh.gender = Some(Gender::Male);
    fresh.aliases = vec!["C".into()];
    MemberRepo::insert(&mut tx, &fresh, None, None)
        .await
        .unwrap();
    let f = MemberRepo::get(&mut tx, fresh.id).await.unwrap().unwrap();
    assert_eq!(
        (f.value.gender, f.value.aliases),
        (Some(Gender::Male), vec!["C".to_string()])
    );

    let pref = NotifyPref {
        id: uuid::Uuid::now_v7(),
        household_id: s.h,
        member_id: s.adult,
        channel: "ntfy".into(),
        topic: "topic-example".into(),
        triggers: vec!["approval_needed".into()],
        due_soon_minutes: 120,
        nudge_minutes: 10,
        quiet_from: None,
        quiet_until: None,
        copy_to_member_id: None,
        approval_mode: ApprovalMode::Digest,
        archived_at: None,
    };
    let p = NotifyPrefRepo::insert(&mut tx, &pref).await.unwrap();
    assert_eq!(p.value.approval_mode, ApprovalMode::Digest);
    let mut changed = p.value.clone();
    changed.approval_mode = ApprovalMode::Immediate;
    NotifyPrefRepo::update(&mut tx, &changed, p.version)
        .await
        .unwrap();
    let got = NotifyPrefRepo::get_for_member(&mut tx, s.adult)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.value.approval_mode, ApprovalMode::Immediate);
    changed.approval_mode = ApprovalMode::Off;
    let up = NotifyPrefRepo::upsert_for_member(&mut tx, &changed, None)
        .await
        .unwrap();
    assert_eq!(up.value.approval_mode, ApprovalMode::Off);
}

#[sqlx::test(migrations = "./migrations")]
async fn message_dedupe_listing_and_content(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let other = add_source(&store, s.h, &[s.adult2]).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let first =
        MessageRepo::insert_parsed(&mut tx, &new_msg(s.h, src, "r1", "Field trip", at(20, 9)))
            .await
            .unwrap();
    let id = first.unwrap();
    assert!(
        MessageRepo::insert_parsed(&mut tx, &new_msg(s.h, src, "r1", "Field trip", at(20, 9)))
            .await
            .unwrap()
            .is_none(),
        "same (source, remote_id) is ignored"
    );
    // the same remote id in another source is a different message
    assert!(
        MessageRepo::insert_parsed(&mut tx, &new_msg(s.h, other, "r1", "Field trip", at(20, 9)))
            .await
            .unwrap()
            .is_some()
    );
    let m = MessageRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(m.processing_state, ProcessingState::Pending);
    assert!(
        m.body_text.is_none() && m.headers.is_none(),
        "get has no content"
    );
    let full = MessageRepo::get_with_content(&mut tx, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(full.body_text.as_deref(), Some("Body text"));
    assert!(full.headers.is_some());
    assert!(
        MessageRepo::get_by_remote(&mut tx, src, "r1")
            .await
            .unwrap()
            .is_some()
    );

    // list: filters
    let m2 = MessageRepo::insert_parsed(
        &mut tx,
        &new_msg(s.h, src, "r2", "Weekly 100% newsletter", at(21, 9)),
    )
    .await
    .unwrap()
    .unwrap();
    let m3 = MessageRepo::insert_parsed(
        &mut tx,
        &new_msg(s.h, src, "r3", "Pizza coupons", at(22, 9)),
    )
    .await
    .unwrap()
    .unwrap();
    MessageRepo::set_classification(&mut tx, id, Some("school"), Some(true), Some(false))
        .await
        .unwrap();
    MessageRepo::set_classification(&mut tx, m2, Some("newsletters"), None, None)
        .await
        .unwrap();
    MessageRepo::set_classification(&mut tx, m3, Some("promotions"), None, None)
        .await
        .unwrap();
    MessageRepo::set_members(
        &mut tx,
        id,
        &[
            (s.child, DecidedBy::Rule, 0.9),
            (s.adult, DecidedBy::User, 1.0),
        ],
    )
    .await
    .unwrap();
    let f = |sources: &'static [MailSourceId]| MessageFilter {
        sources,
        limit: 50,
        ..Default::default()
    };
    let _ = f;
    let sources = [src];
    let all = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        all.iter().map(|m| m.remote_id.as_str()).collect::<Vec<_>>(),
        ["r3", "r2", "r1"]
    );
    assert!(all[0].body_text.is_none());
    let hide = vec!["promotions".to_string()];
    let visible = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            include_ignored_keys: Some(&hide),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(visible.len(), 2);
    let by_cat = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            category: Some("school"),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_cat.len(), 1);
    let by_member = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            member: Some(s.child),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_member.len(), 1);
    let by_q = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            q: Some("100%"),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_q.len(), 1, "wildcards in q are literal");
    let by_q = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            q: Some("TEACHER@"),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(by_q.len(), 3);
    let page = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            before: Some(at(22, 9)),
            limit: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].remote_id, "r2");
    let pending = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &sources,
            state: Some(ProcessingState::Processed),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(pending.is_empty());
    let two = [src, other];
    let both = MessageRepo::list(
        &mut tx,
        &MessageFilter {
            sources: &two,
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(both.len(), 4);
    assert_eq!(MessageRepo::members(&mut tx, id).await.unwrap().len(), 2);
    let mo = MessageRepo::members_of(&mut tx, &[id, m2]).await.unwrap();
    assert_eq!(mo[&id].len(), 2);
    assert!(!mo.contains_key(&m2));
    MessageRepo::set_members(&mut tx, id, &[(s.child, DecidedBy::User, 1.0)])
        .await
        .unwrap();
    assert_eq!(
        MessageRepo::members(&mut tx, id).await.unwrap(),
        vec![(s.child, DecidedBy::User, 1.0)]
    );
    let counts = MailSourceRepo::message_counts(&mut tx).await.unwrap();
    assert_eq!((counts[&src], counts[&other]), (3, 1));

    // state, labels, flags
    MessageRepo::set_state(&mut tx, id, ProcessingState::Processed)
        .await
        .unwrap();
    let m = MessageRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert!(m.processed_at.is_some());
    assert_eq!(
        (m.category.as_deref(), m.time_sensitive, m.is_bill),
        (Some("school"), Some(true), Some(false))
    );
    MessageRepo::set_labels(&mut tx, id, &["INBOX".into(), "Label_1".into()])
        .await
        .unwrap();
    MessageRepo::set_eval_sample(&mut tx, id, true)
        .await
        .unwrap();
    MessageRepo::set_learning_example(&mut tx, id, true)
        .await
        .unwrap();
    let m = MessageRepo::get(&mut tx, id).await.unwrap().unwrap();
    assert!(m.in_eval_sample && m.is_learning_example);
    assert_eq!(m.gmail_labels.len(), 2);
    MessageRepo::attachments_update(&mut tx, id, &serde_json::json!([{"part": "1", "mime": "application/pdf", "size": 5, "pdf_text": "Total"}])).await.unwrap();

    // purge keeps metadata, drops content and attachment text
    assert_eq!(
        MessageRepo::purge_content(&mut tx, &[id, m2])
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        MessageRepo::purge_content(&mut tx, &[id]).await.unwrap(),
        0,
        "idempotent"
    );
    let p = MessageRepo::get_with_content(&mut tx, id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        p.body_text.is_none()
            && p.body_html.is_none()
            && p.headers.is_none()
            && p.snippet.is_none()
    );
    assert!(p.content_purged_at.is_some());
    assert_eq!(p.subject, "Field trip");
    let att = p.attachments.unwrap();
    assert_eq!(att[0]["mime"], "application/pdf");
    assert!(att[0].get("pdf_text").is_none());

    // pending queries
    let pend = MessageRepo::pending(&mut tx, 10).await.unwrap();
    assert!(pend.contains(&m3) && !pend.contains(&id));
    let old = MessageRepo::pending_older_than(&mut tx, Utc::now() + Duration::days(1))
        .await
        .unwrap();
    assert!(old.contains(&m3));
    assert!(
        MessageRepo::pending_older_than(&mut tx, Utc::now() - Duration::days(1))
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn retention_candidates_follow_the_policy(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let a = add_msg(&store, s.h, src, "a", Some("school"), at(20, 9)).await;
    let b = add_msg(&store, s.h, src, "b", Some("school"), at(21, 9)).await;
    let c = add_msg(&store, s.h, src, "c", Some("school"), at(22, 9)).await;
    let d = add_msg(&store, s.h, src, "d", Some("school"), at(23, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    // a, b: learning examples; c: eval sample; d: plain
    for id in [a, b] {
        MessageRepo::set_learning_example(&mut tx, id, true)
            .await
            .unwrap();
    }
    MessageRepo::set_eval_sample(&mut tx, c, true)
        .await
        .unwrap();
    let all =
        MessageRepo::retention_candidates(&mut tx, ContentPolicy::PurgeAfterProcessing, 1, 100)
            .await
            .unwrap();
    assert_eq!(all.len(), 4);
    assert!(
        MessageRepo::retention_candidates(&mut tx, ContentPolicy::KeepAll, 1, 100)
            .await
            .unwrap()
            .is_empty()
    );
    // keep the newest learning example (b) and the eval sample (c); purge a (over the cap) and d
    let learn = MessageRepo::retention_candidates(&mut tx, ContentPolicy::KeepForLearning, 1, 100)
        .await
        .unwrap();
    assert_eq!(learn, vec![a, d]);
    let learn = MessageRepo::retention_candidates(&mut tx, ContentPolicy::KeepForLearning, 5, 100)
        .await
        .unwrap();
    assert_eq!(learn, vec![d]);
    // pending messages are never candidates
    let p = MessageRepo::insert_parsed(&mut tx, &new_msg(s.h, src, "p", "p", at(24, 9)))
        .await
        .unwrap()
        .unwrap();
    let all =
        MessageRepo::retention_candidates(&mut tx, ContentPolicy::PurgeAfterProcessing, 1, 100)
            .await
            .unwrap();
    assert!(!all.contains(&p));
    MessageRepo::purge_content(&mut tx, &[d]).await.unwrap();
    let learn = MessageRepo::retention_candidates(&mut tx, ContentPolicy::KeepForLearning, 5, 100)
        .await
        .unwrap();
    assert!(learn.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn due_for_sync_and_sync_state(pool: PgPool) {
    let store = Store::from_pool(pool);
    let a = seed(&store, "A").await;
    let b = seed(&store, "B").await;
    let now = at(26, 12);
    let mk = |h, name: &str, poll: i32| {
        let mut s = source(h, name);
        s.poll_minutes = poll;
        s
    };
    let (never, recent, stale, paused, archived, other_hh) = (
        mk(a.h, "never", 5),
        mk(a.h, "recent", 5),
        mk(a.h, "stale", 5),
        mk(a.h, "paused", 5),
        mk(a.h, "archived", 5),
        mk(b.h, "other", 60),
    );
    for (h, srcs) in [
        (a.h, vec![&never, &recent, &stale, &paused, &archived]),
        (b.h, vec![&other_hh]),
    ] {
        let mut tx = store.tx_for(h).await.unwrap();
        for s in srcs {
            MailSourceRepo::insert(&mut tx, s, &[]).await.unwrap();
        }
        tx.commit().await.unwrap();
    }
    let mut tx = store.tx_system().await.unwrap();
    MailSourceRepo::set_sync_state(
        &mut tx,
        recent.id,
        Some("100"),
        MailSourceState::Ok,
        None,
        Some(now - Duration::minutes(4)),
    )
    .await
    .unwrap();
    MailSourceRepo::set_sync_state(
        &mut tx,
        stale.id,
        None,
        MailSourceState::Ok,
        None,
        Some(now - Duration::minutes(5)),
    )
    .await
    .unwrap();
    MailSourceRepo::set_sync_state(
        &mut tx,
        paused.id,
        None,
        MailSourceState::Paused,
        None,
        None,
    )
    .await
    .unwrap();
    MailSourceRepo::archive(&mut tx, archived.id).await.unwrap();
    MailSourceRepo::set_sync_state(
        &mut tx,
        other_hh.id,
        None,
        MailSourceState::Ok,
        None,
        Some(now - Duration::minutes(59)),
    )
    .await
    .unwrap();
    let due = MailSourceRepo::due_for_sync(&mut tx, now).await.unwrap();
    let ids: Vec<_> = due.iter().map(|d| d.1).collect();
    assert_eq!(
        ids,
        vec![never.id, stale.id],
        "never-synced first; boundary is inclusive"
    );
    assert!(due.iter().all(|d| d.0 == a.h));
    let due = MailSourceRepo::due_for_sync(&mut tx, now + Duration::minutes(2))
        .await
        .unwrap();
    assert!(due.iter().any(|d| d.1 == recent.id));
    let due = MailSourceRepo::due_for_sync(&mut tx, now + Duration::minutes(1))
        .await
        .unwrap();
    assert!(due.iter().any(|d| d.1 == other_hh.id));

    // history id and error handling
    let got = MailSourceRepo::get(&mut tx, recent.id)
        .await
        .unwrap()
        .unwrap()
        .value;
    assert_eq!(got.history_id.as_deref(), Some("100"));
    assert_eq!(got.address, "parent@example.com");
    MailSourceRepo::set_sync_state(
        &mut tx,
        recent.id,
        None,
        MailSourceState::Error,
        Some("boom"),
        None,
    )
    .await
    .unwrap();
    let got = MailSourceRepo::get(&mut tx, recent.id)
        .await
        .unwrap()
        .unwrap()
        .value;
    assert_eq!(
        (
            got.state,
            got.last_error.as_deref(),
            got.history_id.as_deref()
        ),
        (MailSourceState::Error, Some("boom"), Some("100"))
    );
    assert!(got.last_synced_at.is_some());
    MailSourceRepo::set_sync_state(&mut tx, recent.id, None, MailSourceState::Ok, None, None)
        .await
        .unwrap();
    MailSourceRepo::clear_history_id(&mut tx, recent.id)
        .await
        .unwrap();
    MailSourceRepo::set_gmail_ext(&mut tx, recent.id, true)
        .await
        .unwrap();
    let got = MailSourceRepo::get(&mut tx, recent.id)
        .await
        .unwrap()
        .unwrap()
        .value;
    assert!(got.history_id.is_none() && got.last_error.is_none() && got.gmail_ext);
    // an errored source is not due until resumed
    MailSourceRepo::set_sync_state(
        &mut tx,
        stale.id,
        None,
        MailSourceState::NeedsReauth,
        Some("x"),
        None,
    )
    .await
    .unwrap();
    let due = MailSourceRepo::due_for_sync(&mut tx, now + Duration::days(1))
        .await
        .unwrap();
    assert!(due.iter().all(|d| d.1 != stale.id));
}

#[sqlx::test(migrations = "./migrations")]
async fn sources_owners_visibility_and_folders(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let owned = source(s.h, "mine");
    let mut shared = source(s.h, "shared");
    shared.visibility = MailVisibility::Household;
    let theirs = source(s.h, "theirs");
    MailSourceRepo::insert(&mut tx, &owned, &[s.adult])
        .await
        .unwrap();
    MailSourceRepo::insert(&mut tx, &shared, &[s.adult2])
        .await
        .unwrap();
    let t = MailSourceRepo::insert(&mut tx, &theirs, &[s.adult2])
        .await
        .unwrap();
    let readable = MailSourceRepo::readable_by(&mut tx, s.adult).await.unwrap();
    assert_eq!(readable, vec![owned.id, shared.id]);
    let readable2 = MailSourceRepo::readable_by(&mut tx, s.adult2)
        .await
        .unwrap();
    assert_eq!(readable2.len(), 2);
    let listed = MailSourceRepo::list(&mut tx, false).await.unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].1, vec![s.adult]);
    MailSourceRepo::set_owners(&mut tx, theirs.id, &[s.adult, s.adult2])
        .await
        .unwrap();
    assert_eq!(
        MailSourceRepo::owners(&mut tx, theirs.id)
            .await
            .unwrap()
            .len(),
        2
    );
    // update + version conflict
    let mut edited = t.value.clone();
    edited.name = "renamed".into();
    edited.state = MailSourceState::Paused;
    let v = MailSourceRepo::update(&mut tx, &edited, t.version)
        .await
        .unwrap();
    assert_eq!(v, t.version + 1);
    assert!(matches!(
        MailSourceRepo::update(&mut tx, &edited, t.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        MailSourceRepo::update(&mut tx, &source(s.h, "ghost"), 1).await,
        Err(StoreError::NotFound)
    ));
    MailSourceRepo::archive(&mut tx, owned.id).await.unwrap();
    assert_eq!(
        MailSourceRepo::readable_by(&mut tx, s.adult).await.unwrap(),
        vec![shared.id, theirs.id]
    );
    // a gmail source needs an oauth account; an imap source needs host and user
    let mut bad = source(s.h, "bad");
    bad.provider = MailProvider::Gmail;
    assert!(MailSourceRepo::insert(&mut tx, &bad, &[]).await.is_err());
    tx.rollback().await.unwrap();

    // folder state
    let mut tx = store.tx_for(s.h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &owned, &[s.adult])
        .await
        .unwrap();
    let st = MailFolderState {
        source_id: owned.id,
        folder: "INBOX".into(),
        uidvalidity: 1,
        uidnext: 5,
        highestmodseq: Some(9),
    };
    MailFolderStateRepo::upsert(&mut tx, &st).await.unwrap();
    assert_eq!(
        MailFolderStateRepo::get(&mut tx, owned.id, "INBOX")
            .await
            .unwrap(),
        Some(st.clone())
    );
    let st2 = MailFolderState {
        uidnext: 8,
        highestmodseq: None,
        ..st.clone()
    };
    MailFolderStateRepo::upsert(&mut tx, &st2).await.unwrap();
    assert_eq!(
        MailFolderStateRepo::list(&mut tx, owned.id).await.unwrap(),
        vec![st2]
    );
    MailFolderStateRepo::reset(&mut tx, owned.id, "INBOX")
        .await
        .unwrap();
    assert!(
        MailFolderStateRepo::get(&mut tx, owned.id, "INBOX")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn label_queue_is_stratified(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let other_src = add_source(&store, s.h, &[s.adult2]).await;
    let mut ids = std::collections::HashMap::new();
    for (cat, n) in [("school", 4), ("bills", 2), ("social", 1)] {
        for i in 0..n {
            let remote = format!("{cat}{i}");
            let id = add_msg(&store, s.h, src, &remote, Some(cat), at(10 + i, 9)).await;
            ids.insert(remote, id);
        }
    }
    let hidden = add_msg(&store, s.h, other_src, "hidden", Some("school"), at(25, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    mail_defaults::ensure(&mut tx, s.h).await.unwrap();
    let dp = DecisionPointRepo::get_by_key(&mut tx, DecisionPointKey::EmailCategory)
        .await
        .unwrap()
        .unwrap();
    let decide = |msg, is_final, by| NewDecision {
        household_id: s.h,
        decision_point_id: dp.id,
        subject: Subject::message(msg),
        pipeline_version_id: None,
        stage_kind: if is_final {
            StageKind::Rules
        } else {
            StageKind::Human
        },
        value: serde_json::json!("school"),
        confidence: 0.5,
        is_final,
        source: None,
        decided_by: by,
    };
    // school0 is the oldest but its latest decision is not final: it comes first among school
    for k in [
        "school1", "school2", "school3", "bills0", "bills1", "social0",
    ] {
        DecisionRepo::insert(&mut tx, &decide(ids[k], true, None))
            .await
            .unwrap();
    }
    DecisionRepo::insert(&mut tx, &decide(ids["school0"], false, None))
        .await
        .unwrap();
    // bills1 was already labelled by a member: excluded
    DecisionRepo::insert(&mut tx, &decide(ids["bills1"], true, Some(s.adult)))
        .await
        .unwrap();
    let readable = [src];
    let q = MessageRepo::label_queue(&mut tx, s.h, &readable, &[], 10)
        .await
        .unwrap();
    let cat_of = |id: &MessageId| ids.iter().find(|(_, v)| *v == id).unwrap().0.clone();
    let order: Vec<String> = q.iter().map(cat_of).collect();
    assert_eq!(
        q.len(),
        6,
        "bills1 labelled, hidden source skipped: {order:?}"
    );
    assert!(!q.contains(&hidden) && !q.contains(&ids["bills1"]));
    // round one: one of each category, round two continues
    let first_round: std::collections::HashSet<_> = order[..3]
        .iter()
        .map(|k| k.trim_end_matches(char::is_numeric).to_string())
        .collect();
    assert_eq!(first_round.len(), 3, "{order:?}");
    assert_eq!(
        order[0].trim_end_matches(char::is_numeric),
        "bills",
        "categories in key order within a round: {order:?}"
    );
    let school_first = order.iter().find(|k| k.starts_with("school")).unwrap();
    assert_eq!(
        school_first, "school0",
        "non-final decisions are preferred: {order:?}"
    );
    // limit and exclude
    let q2 = MessageRepo::label_queue(&mut tx, s.h, &readable, &[q[0]], 2)
        .await
        .unwrap();
    assert_eq!(q2.len(), 2);
    assert!(!q2.contains(&q[0]));
    assert!(
        MessageRepo::label_queue(&mut tx, s.h, &[], &[], 10)
            .await
            .unwrap()
            .is_empty()
    );
    // eval progress
    MessageRepo::set_eval_sample(&mut tx, ids["bills1"], true)
        .await
        .unwrap();
    MessageRepo::set_eval_sample(&mut tx, ids["social0"], true)
        .await
        .unwrap();
    assert_eq!(
        MessageRepo::eval_progress(&mut tx, s.h).await.unwrap(),
        (2, 1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn decisions_and_pipeline_versions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    mail_defaults::ensure(&mut tx, s.h).await.unwrap();
    let dp = DecisionPointRepo::get_by_key(&mut tx, DecisionPointKey::EmailMemberLink)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dp.key, DecisionPointKey::EmailMemberLink);
    let msg = MessageId::new();
    let subject = Subject::message(msg);
    assert!(
        DecisionRepo::latest(&mut tx, dp.id, subject)
            .await
            .unwrap()
            .is_none()
    );
    let new = |value: serde_json::Value, is_final| NewDecision {
        household_id: s.h,
        decision_point_id: dp.id,
        subject,
        pipeline_version_id: None,
        stage_kind: StageKind::Rules,
        value,
        confidence: 0.9,
        is_final,
        source: Some("member_name".into()),
        decided_by: None,
    };
    DecisionRepo::insert(&mut tx, &new(serde_json::json!("first"), false))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    // a later transaction writes a two-valued decision
    let mut tx = store.tx_for(s.h).await.unwrap();
    DecisionRepo::insert(&mut tx, &new(serde_json::json!(s.adult.to_string()), true))
        .await
        .unwrap();
    DecisionRepo::insert(&mut tx, &new(serde_json::json!(s.child.to_string()), true))
        .await
        .unwrap();
    let latest = DecisionRepo::latest(&mut tx, dp.id, subject)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.value, serde_json::json!(s.child.to_string()));
    assert!(
        DecisionRepo::latest_final(&mut tx, dp.id, subject)
            .await
            .unwrap()
            .unwrap()
            .is_final
    );
    assert_eq!(
        DecisionRepo::latest_batch(&mut tx, dp.id, subject)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        DecisionRepo::for_subject(&mut tx, subject)
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        DecisionRepo::delete_for_subjects(&mut tx, SubjectType::Message, &[msg.as_uuid()])
            .await
            .unwrap(),
        3
    );
    assert!(
        DecisionRepo::for_subject(&mut tx, subject)
            .await
            .unwrap()
            .is_empty()
    );

    // a new active version replaces the old one
    let v2 = PipelineVersion {
        id: PipelineVersionId::new(),
        household_id: s.h,
        decision_point_id: dp.id,
        version: 2,
        active: true,
        name: "Experiment".into(),
        created_by: Some(s.adult),
    };
    let stage = PipelineStage {
        id: PipelineStageId::new(),
        household_id: s.h,
        pipeline_version_id: v2.id,
        ord: 0,
        kind: StageKind::Rules,
        enabled: true,
        accept_threshold: 0.9,
        on_unavailable: OnUnavailable::Skip,
        config: serde_json::json!({}),
    };
    PipelineRepo::insert_version(&mut tx, &v2, &[stage])
        .await
        .unwrap();
    let (active, stages) = PipelineRepo::active_version(&mut tx, dp.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((active.version, stages.len()), (2, 1));
    let versions = PipelineRepo::versions(&mut tx, dp.id).await.unwrap();
    assert_eq!(
        versions
            .iter()
            .map(|v| (v.version, v.active))
            .collect::<Vec<_>>(),
        vec![(2, true), (1, false)]
    );
    PipelineRepo::activate(&mut tx, versions[1].id)
        .await
        .unwrap();
    assert_eq!(
        PipelineRepo::active_version(&mut tx, dp.id)
            .await
            .unwrap()
            .unwrap()
            .0
            .version,
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn orgs_contacts_enrollments_and_suggestions(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let _o = OrgRepo::insert(&mut tx, &org(s.h, "Example School"))
        .await
        .unwrap();
    assert!(
        OrgRepo::insert(&mut tx, &org(s.h, "example school"))
            .await
            .is_err(),
        "name is unique per household"
    );
    tx.rollback().await.unwrap();
    let mut tx = store.tx_for(s.h).await.unwrap();
    let o = OrgRepo::insert(&mut tx, &org(s.h, "Example School"))
        .await
        .unwrap();
    assert_eq!(
        OrgRepo::get_by_name(&mut tx, "EXAMPLE school")
            .await
            .unwrap()
            .unwrap()
            .value
            .id,
        o.value.id
    );
    let mut changed = o.value.clone();
    changed.senders = vec!["office@example.edu".into()];
    changed.contact_mode = ContactMode::Auto;
    let v = OrgRepo::update(&mut tx, &changed, o.version).await.unwrap();
    assert!(matches!(
        OrgRepo::update(&mut tx, &changed, o.version).await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        OrgRepo::get(&mut tx, o.value.id)
            .await
            .unwrap()
            .unwrap()
            .version,
        v
    );
    let o2 = OrgRepo::insert(&mut tx, &org(s.h, "Example Club"))
        .await
        .unwrap();
    let contact = |name: &str, email: &str, org: OrgId| OrgContact {
        id: OrgContactId::new(),
        household_id: s.h,
        org_id: org,
        name: name.into(),
        email: email.into(),
        role: ContactRole::Teacher,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let c1 = OrgContactRepo::insert(
        &mut tx,
        &contact("Ms Example", "MS@example.edu", o.value.id),
    )
    .await
    .unwrap();
    OrgContactRepo::insert(&mut tx, &contact("Coach", "ms@example.edu", o2.value.id))
        .await
        .unwrap();
    let hits = OrgContactRepo::by_email(&mut tx, "Ms@Example.edu")
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert!(
        hits.iter()
            .any(|(c, org)| c.id == c1.value.id && *org == o.value.id)
    );
    OrgContactRepo::archive(&mut tx, c1.value.id).await.unwrap();
    assert_eq!(
        OrgContactRepo::by_email(&mut tx, "ms@example.edu")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        OrgContactRepo::list(&mut tx, o.value.id)
            .await
            .unwrap()
            .is_empty()
    );
    let mut renamed = c1.value.clone();
    renamed.name = "Mrs Example".into();
    OrgContactRepo::update(&mut tx, &renamed, c1.version + 1)
        .await
        .unwrap();
    assert!(matches!(
        OrgContactRepo::update(&mut tx, &renamed, 1).await,
        Err(StoreError::Conflict)
    ));

    // enrollments and their contacts
    let e = EnrollmentRepo::insert(
        &mut tx,
        &Enrollment {
            id: EnrollmentId::new(),
            household_id: s.h,
            member_id: s.child,
            org_id: o.value.id,
            grade: Some("3".into()),
            class_name: Some("3B".into()),
            from_date: NaiveDate::from_ymd_opt(2026, 9, 1),
            until_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    EnrollmentRepo::set_contacts(&mut tx, e.value.id, &[c1.value.id])
        .await
        .unwrap();
    EnrollmentRepo::set_contacts(&mut tx, e.value.id, &[c1.value.id])
        .await
        .unwrap();
    assert_eq!(
        EnrollmentRepo::contacts(&mut tx, e.value.id).await.unwrap(),
        vec![c1.value.id]
    );
    assert_eq!(
        EnrollmentRepo::list(&mut tx, Some(s.child))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        EnrollmentRepo::list(&mut tx, Some(s.adult))
            .await
            .unwrap()
            .is_empty()
    );
    let mut changed = e.value.clone();
    changed.grade = Some("4".into());
    EnrollmentRepo::update(&mut tx, &changed, e.version)
        .await
        .unwrap();
    assert!(matches!(
        EnrollmentRepo::update(&mut tx, &changed, e.version).await,
        Err(StoreError::Conflict)
    ));
    EnrollmentRepo::set_contacts(&mut tx, e.value.id, &[])
        .await
        .unwrap();
    assert!(
        EnrollmentRepo::contacts(&mut tx, e.value.id)
            .await
            .unwrap()
            .is_empty()
    );
    EnrollmentRepo::delete(&mut tx, e.value.id).await.unwrap();
    assert!(matches!(
        EnrollmentRepo::delete(&mut tx, e.value.id).await,
        Err(StoreError::NotFound)
    ));

    // suggestions: one per (org, email) in any status
    let sug = |email: &str| ContactSuggestion {
        id: ContactSuggestionId::new(),
        household_id: s.h,
        org_id: o.value.id,
        email: email.into(),
        name: Some("Coach".into()),
        member_id: Some(s.child),
        message_id: None,
        status: SuggestionStatus::Proposed,
        decided_by: None,
        decided_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let first = sug("Coach@example.edu");
    assert!(
        ContactSuggestionRepo::insert_if_new(&mut tx, &first)
            .await
            .unwrap()
    );
    assert!(
        !ContactSuggestionRepo::insert_if_new(&mut tx, &sug("coach@example.edu"))
            .await
            .unwrap()
    );
    ContactSuggestionRepo::set_status(&mut tx, first.id, SuggestionStatus::Rejected, Some(s.adult))
        .await
        .unwrap();
    assert!(
        !ContactSuggestionRepo::insert_if_new(&mut tx, &sug("coach@example.edu"))
            .await
            .unwrap(),
        "rejected stays rejected"
    );
    let got = ContactSuggestionRepo::get(&mut tx, first.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (got.status, got.decided_by),
        (SuggestionStatus::Rejected, Some(s.adult))
    );
    assert!(got.decided_at.is_some());
    assert!(
        ContactSuggestionRepo::insert_if_new(&mut tx, &sug("office@example.edu"))
            .await
            .unwrap()
    );
    assert_eq!(
        ContactSuggestionRepo::list(&mut tx, Some(SuggestionStatus::Proposed))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ContactSuggestionRepo::expire_older_than(&mut tx, Utc::now() - Duration::days(1))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        ContactSuggestionRepo::expire_older_than(&mut tx, Utc::now() + Duration::days(1))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        ContactSuggestionRepo::list(&mut tx, None)
            .await
            .unwrap()
            .len(),
        2
    );
    OrgRepo::archive(&mut tx, o.value.id).await.unwrap();
    assert!(
        OrgRepo::get_by_name(&mut tx, "Example School")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(OrgRepo::list(&mut tx, false).await.unwrap().len(), 1);
    assert_eq!(OrgRepo::list(&mut tx, true).await.unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn rules_and_categories(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    mail_defaults::ensure(&mut tx, s.h).await.unwrap();
    let rule = |name: &str, dp, prio| MailRule {
        id: MailRuleId::new(),
        household_id: s.h,
        decision_point: dp,
        name: name.into(),
        priority: prio,
        conditions: vec![RuleCondition {
            field: RuleField::Header,
            op: RuleOp::Exists,
            header: Some("list-id".into()),
            values: vec![],
        }],
        value: serde_json::json!("newsletters"),
        confidence: 0.9,
        enabled: true,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let r1 = MailRuleRepo::insert(&mut tx, &rule("late", DecisionPointKey::EmailCategory, 50))
        .await
        .unwrap();
    MailRuleRepo::insert(&mut tx, &rule("early", DecisionPointKey::EmailCategory, 5))
        .await
        .unwrap();
    MailRuleRepo::insert(&mut tx, &rule("bill", DecisionPointKey::EmailIsBill, 1))
        .await
        .unwrap();
    let cat = MailRuleRepo::list(&mut tx, Some(DecisionPointKey::EmailCategory), false)
        .await
        .unwrap();
    assert_eq!(
        cat.iter()
            .map(|r| r.value.name.as_str())
            .collect::<Vec<_>>(),
        ["early", "late"]
    );
    assert_eq!(
        cat[0].value.conditions[0].header.as_deref(),
        Some("list-id")
    );
    assert_eq!(
        MailRuleRepo::list(&mut tx, None, false)
            .await
            .unwrap()
            .len(),
        3
    );
    let mut e = r1.value.clone();
    e.enabled = false;
    e.priority = 1;
    MailRuleRepo::update(&mut tx, &e, r1.version).await.unwrap();
    assert!(matches!(
        MailRuleRepo::update(&mut tx, &e, r1.version).await,
        Err(StoreError::Conflict)
    ));
    MailRuleRepo::archive(&mut tx, r1.value.id).await.unwrap();
    assert_eq!(
        MailRuleRepo::list(&mut tx, None, false)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        MailRuleRepo::list(&mut tx, None, true).await.unwrap().len(),
        3
    );

    let school = CategoryRepo::get_by_key(&mut tx, "school")
        .await
        .unwrap()
        .unwrap();
    let mut c = school.value.clone();
    c.auto_actions = vec![AutoAction::Label, AutoAction::Archive];
    c.email_tasks = EmailTasksMode::Create;
    c.color = Some("#336699".into());
    let v = CategoryRepo::update(&mut tx, &c, school.version)
        .await
        .unwrap();
    assert!(matches!(
        CategoryRepo::update(&mut tx, &c, school.version).await,
        Err(StoreError::Conflict)
    ));
    let got = CategoryRepo::get(&mut tx, school.value.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.version, v);
    assert_eq!(
        got.value.auto_actions,
        vec![AutoAction::Label, AutoAction::Archive]
    );
    let custom = Category {
        id: CategoryId::new(),
        household_id: s.h,
        key: "pets".into(),
        label: "Pets".into(),
        behaviour: CategoryBehaviour::Keep,
        notify_time_sensitive: false,
        auto_actions: vec![],
        email_tasks: EmailTasksMode::Off,
        color: None,
        ord: 20,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let pets = CategoryRepo::insert(&mut tx, &custom).await.unwrap();
    assert!(pets.value.auto_actions.is_empty());
    let mut dup = custom.clone();
    dup.id = CategoryId::new();
    assert!(CategoryRepo::insert(&mut tx, &dup).await.is_err());
    tx.rollback().await.unwrap();
    // archive through update
    let mut tx = store.tx_for(s.h).await.unwrap();
    mail_defaults::ensure(&mut tx, s.h).await.unwrap();
    let school = CategoryRepo::get_by_key(&mut tx, "school")
        .await
        .unwrap()
        .unwrap();
    let mut c = school.value.clone();
    c.archived_at = Some(Utc::now());
    CategoryRepo::update(&mut tx, &c, school.version)
        .await
        .unwrap();
    assert_eq!(CategoryRepo::list(&mut tx, false).await.unwrap().len(), 11);
    assert_eq!(CategoryRepo::list(&mut tx, true).await.unwrap().len(), 12);
}

#[sqlx::test(migrations = "./migrations")]
async fn bills_dedupe_paid_and_reopen(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let msg = add_msg(&store, s.h, src, "bill1", Some("bills"), at(20, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let mut b = bill(s.h, "Example Power Co.", 12_345, BillStatus::Open);
    b.message_id = Some(msg);
    let _first = BillRepo::insert_if_new(&mut tx, &b).await.unwrap().unwrap();
    let mut again = bill(s.h, "EXAMPLE POWER CO", 12_345, BillStatus::Proposed);
    again.message_id = Some(msg);
    assert!(
        BillRepo::insert_if_new(&mut tx, &again)
            .await
            .unwrap()
            .is_none(),
        "same dedupe key"
    );
    assert!(
        BillRepo::insert_if_new(
            &mut tx,
            &bill(s.h, "Example Power Co.", 999, BillStatus::Proposed)
        )
        .await
        .unwrap()
        .is_some()
    );
    assert_eq!(BillRepo::list(&mut tx, None).await.unwrap().len(), 2);
    assert_eq!(
        BillRepo::list(&mut tx, Some(BillStatus::Open))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(BillRepo::for_message(&mut tx, msg).await.unwrap().len(), 1);

    // pay task link
    let list =
        TaskListRepo::get_or_create(&mut tx, TaskListKind::General, "Bills", &list_defaults(s.h))
            .await
            .unwrap();
    let task = NewTaskRow {
        source_ref: None,
        origin: TaskOrigin::System,
        ..email_task_row(s.h, list.value.id, msg, TaskStatus::Open)
    };
    TaskRepo::insert(&mut tx, &task, &[s.adult], &[])
        .await
        .unwrap()
        .unwrap();
    assert!(BillRepo::by_task(&mut tx, task.id).await.unwrap().is_none());
    BillRepo::set_task(&mut tx, b.id, task.id).await.unwrap();
    assert_eq!(
        BillRepo::by_task(&mut tx, task.id)
            .await
            .unwrap()
            .unwrap()
            .value
            .id,
        b.id
    );
    assert!(
        !BillRepo::reopen_by_task(&mut tx, task.id).await.unwrap(),
        "only paid bills reopen"
    );
    assert!(BillRepo::set_paid_by_task(&mut tx, task.id).await.unwrap());
    assert!(
        !BillRepo::set_paid_by_task(&mut tx, task.id).await.unwrap(),
        "idempotent"
    );
    assert_eq!(
        BillRepo::get(&mut tx, b.id)
            .await
            .unwrap()
            .unwrap()
            .value
            .status,
        BillStatus::Paid
    );
    assert!(BillRepo::reopen_by_task(&mut tx, task.id).await.unwrap());
    assert_eq!(
        BillRepo::get(&mut tx, b.id)
            .await
            .unwrap()
            .unwrap()
            .value
            .status,
        BillStatus::Open
    );
    // a dismissed bill is not touched by task changes
    BillRepo::set_status(&mut tx, b.id, BillStatus::Dismissed)
        .await
        .unwrap();
    assert!(!BillRepo::set_paid_by_task(&mut tx, task.id).await.unwrap());

    // versions
    let cur = BillRepo::get(&mut tx, b.id).await.unwrap().unwrap();
    let mut edited = cur.value.clone();
    edited.amount_minor = 20_000;
    edited.status = BillStatus::Open;
    let v = BillRepo::update(&mut tx, &edited, cur.version)
        .await
        .unwrap();
    assert_eq!(v, cur.version + 1);
    assert!(matches!(
        BillRepo::update(&mut tx, &edited, cur.version).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        BillRepo::update(&mut tx, &bill(s.h, "ghost", 1, BillStatus::Open), 1).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_actions_undo(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let msg = add_msg(&store, s.h, src, "m", None, at(20, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let a = MailActionRepo::insert(
        &mut tx,
        &NewMailAction {
            household_id: s.h,
            message_id: msg,
            action: MailActionKind::Archive,
            label: None,
            automatic: false,
            done_by: Some(s.adult),
            error: None,
        },
    )
    .await
    .unwrap();
    assert!(a.undone_at.is_none());
    assert!(
        MailActionRepo::mark_undone(&mut tx, a.id, Some(s.adult))
            .await
            .unwrap()
    );
    assert!(
        !MailActionRepo::mark_undone(&mut tx, a.id, Some(s.adult))
            .await
            .unwrap()
    );
    assert!(matches!(
        MailActionRepo::mark_undone(&mut tx, MailActionId::new(), None).await,
        Err(StoreError::NotFound)
    ));
    MailActionRepo::set_error(&mut tx, a.id, Some("boom"))
        .await
        .unwrap();
    let got = MailActionRepo::get(&mut tx, a.id).await.unwrap().unwrap();
    assert_eq!(
        (got.undone_by, got.error.as_deref()),
        (Some(s.adult), Some("boom"))
    );
    assert_eq!(
        MailActionRepo::list_for_message(&mut tx, msg)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn upsert_synced_keeps_extracted_rows(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let msg = add_msg(&store, s.h, src, "m", Some("school"), at(20, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let cs = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: s.h,
        kind: SourceKind::Ics,
        name: "Feed".into(),
        owner_member: None,
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: true,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    CalendarSourceRepo::insert(&mut tx, &cs).await.unwrap();
    let (cal, _) = CalendarRepo::upsert_discovered(&mut tx, cs.id, "remote-1", "Family")
        .await
        .unwrap();
    let cal = cal.value.id;

    let mut e = extracted(s.h, "Concert", at(30, 18));
    e.message_id = Some(msg);
    let id = EventRepo::insert_extracted(&mut tx, &e).await.unwrap();
    EventRepo::set_status(&mut tx, id, EventStatus::Approved, Some(s.adult))
        .await
        .unwrap();
    EventRepo::set_written(
        &mut tx,
        id,
        cal,
        "uid-1@example.com",
        Some("etag-1"),
        Some("https://example.com/e/1"),
    )
    .await
    .unwrap();
    let before = EventRepo::get_full(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(before.event.status, Some(EventStatus::Written));
    assert_eq!(before.event.calendar, Some(cal));
    assert_eq!(
        before.event.source_ref.as_ref().unwrap()["external_url"],
        "https://example.com/e/1"
    );
    assert_eq!(before.event.source_ref.as_ref().unwrap()["kind"], "email");

    // the next sync sees our event with provider-side changes
    let occ = NewOccurrence {
        remote_uid: "uid-1@example.com".into(),
        instance_key: String::new(),
        title: "Concert (renamed upstream)".into(),
        description: None,
        location: None,
        start_at: at(30, 19),
        end_at: at(30, 20),
        all_day: false,
        start_date: None,
        end_date: None,
        source_tz: None,
        is_private: false,
        cancelled: false,
        remote_etag: Some("etag-2".into()),
        external_url: Some("https://example.com/other".into()),
    };
    assert_eq!(
        EventRepo::upsert_synced(&mut tx, cal, std::slice::from_ref(&occ))
            .await
            .unwrap(),
        1
    );
    let after = EventRepo::get_full(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(after.event.origin, EventOrigin::Extracted);
    assert_eq!(after.event.status, Some(EventStatus::Written));
    assert_eq!(after.event.source_ref, before.event.source_ref);
    assert_eq!(after.message_id, Some(msg));
    assert_eq!(after.event.title, "Concert");
    assert_eq!(after.event.remote_etag.as_deref(), Some("etag-2"));
    assert_eq!(after.event.start_at, at(30, 19), "times are refreshed");
    // repeating the same sync changes nothing
    assert_eq!(
        EventRepo::upsert_synced(&mut tx, cal, &[occ])
            .await
            .unwrap(),
        0
    );
    // the only row for that uid is still ours
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM event WHERE remote_uid = 'uid-1@example.com'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(n, 1);
    // and a window sweep never deletes it
    assert_eq!(
        EventRepo::delete_unseen_in_window(
            &mut tx,
            cal,
            at(1, 0),
            at(31, 0) + Duration::days(30),
            &[]
        )
        .await
        .unwrap(),
        0
    );
    assert!(EventRepo::get_full(&mut tx, id).await.unwrap().is_some());

    // the other order: the sync imported the event before it was marked written
    let id2 = EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Recital", at(29, 10)))
        .await
        .unwrap();
    let occ2 = NewOccurrence {
        remote_uid: "uid-2@example.com".into(),
        title: "Recital".into(),
        start_at: at(29, 10),
        end_at: at(29, 11),
        remote_etag: None,
        external_url: None,
        ..NewOccurrence {
            remote_uid: String::new(),
            instance_key: String::new(),
            title: String::new(),
            description: None,
            location: None,
            start_at: at(1, 1),
            end_at: at(1, 2),
            all_day: false,
            start_date: None,
            end_date: None,
            source_tz: None,
            is_private: false,
            cancelled: false,
            remote_etag: None,
            external_url: None,
        }
    };
    EventRepo::upsert_synced(&mut tx, cal, &[occ2])
        .await
        .unwrap();
    EventRepo::set_written(&mut tx, id2, cal, "uid-2@example.com", None, None)
        .await
        .unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM event WHERE remote_uid = 'uid-2@example.com'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(n, 1, "the imported duplicate is replaced by our row");
}

#[sqlx::test(migrations = "./migrations")]
async fn extracted_event_lifecycle(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let msg = add_msg(&store, s.h, src, "m", Some("school"), at(20, 9)).await;
    let msg2 = add_msg(&store, s.h, src, "m2", Some("school"), at(21, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    let o = OrgRepo::insert(&mut tx, &org(s.h, "Example School"))
        .await
        .unwrap();
    let mut e = extracted(s.h, "Concert", at(30, 18));
    e.message_id = Some(msg);
    e.org_id = Some(o.value.id);
    e.members = vec![s.child];
    let id = EventRepo::insert_extracted(&mut tx, &e).await.unwrap();
    let full = EventRepo::get_full(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(full.event.origin, EventOrigin::Extracted);
    assert_eq!(full.event.status, Some(EventStatus::Extracted));
    assert_eq!(
        (full.message_id, full.org_id, full.members.clone()),
        (Some(msg), Some(o.value.id), vec![s.child])
    );
    assert!(full.event.calendar.is_none());
    // extracted rows are not shown on the calendar
    let seen = EventRepo::range(&mut tx, at(1, 0), at(31, 23), None)
        .await
        .unwrap();
    assert!(seen.is_empty());

    // dedupe candidates: any origin, window on start, not rejected/expired
    let near = EventRepo::dedupe_candidates(&mut tx, s.h, at(30, 0), at(31, 0))
        .await
        .unwrap();
    assert_eq!(near.len(), 1);
    assert!(
        EventRepo::dedupe_candidates(&mut tx, s.h, at(1, 0), at(2, 0))
            .await
            .unwrap()
            .is_empty()
    );
    EventRepo::set_route(&mut tx, id, None, Some(0.4))
        .await
        .unwrap();
    EventRepo::set_status(&mut tx, id, EventStatus::Proposed, None)
        .await
        .unwrap();
    let props = EventRepo::proposals(&mut tx).await.unwrap();
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].message_subject.as_deref(), Some("m"));
    assert_eq!(props[0].message_from.as_deref(), Some("Teacher"));
    assert_eq!(props[0].org_name.as_deref(), Some("Example School"));
    assert_eq!(props[0].event.route_confidence, Some(0.4));
    assert_eq!(
        InboxRepo::counts(&mut tx).await.unwrap(),
        InboxCounts::new(1, 0, 0, 0)
    );

    // provenance
    EventRepo::link_message(&mut tx, id, msg2).await.unwrap();
    EventRepo::link_message(&mut tx, id, msg2).await.unwrap();
    assert_eq!(
        EventRepo::derived_for_message(&mut tx, msg)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        EventRepo::derived_for_message(&mut tx, msg2)
            .await
            .unwrap()
            .len(),
        1
    );

    // approve and write; a write error keeps it approved
    EventRepo::set_status(&mut tx, id, EventStatus::Approved, Some(s.adult))
        .await
        .unwrap();
    let ap = EventRepo::get_full(&mut tx, id).await.unwrap().unwrap();
    assert_eq!(
        (ap.decided_by, ap.decided_at.is_some()),
        (Some(s.adult), true)
    );
    EventRepo::set_write_error(&mut tx, id, Some("reconnect with calendar write access"))
        .await
        .unwrap();
    assert_eq!(
        EventRepo::get_full(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
            .write_error
            .as_deref(),
        Some("reconnect with calendar write access")
    );
    // an approved event whose write failed waits in the inbox again
    assert_eq!(EventRepo::proposals(&mut tx).await.unwrap().len(), 1);
    assert_eq!(InboxRepo::counts(&mut tx).await.unwrap().event, 1);
    EventRepo::set_write_error(&mut tx, id, None).await.unwrap();
    assert!(EventRepo::proposals(&mut tx).await.unwrap().is_empty());
    assert_eq!(InboxRepo::counts(&mut tx).await.unwrap().event, 0);

    // pending cancel on a written event shows up as a proposal
    let cs = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: s.h,
        kind: SourceKind::Ics,
        name: "Feed".into(),
        owner_member: None,
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: true,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    CalendarSourceRepo::insert(&mut tx, &cs).await.unwrap();
    let (cal, _) = CalendarRepo::upsert_discovered(&mut tx, cs.id, "r", "Family")
        .await
        .unwrap();
    EventRepo::set_written(
        &mut tx,
        id,
        cal.value.id,
        "uid@example.com",
        Some("e"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        EventRepo::get_full(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
            .write_error,
        None
    );
    EventRepo::set_pending_cancel(&mut tx, id, true)
        .await
        .unwrap();
    assert_eq!(EventRepo::proposals(&mut tx).await.unwrap().len(), 1);
    assert_eq!(InboxRepo::counts(&mut tx).await.unwrap().event, 1);
    EventRepo::set_pending_cancel(&mut tx, id, false)
        .await
        .unwrap();

    // expiry only touches proposed events that already started
    let old = EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Old", at(2, 10)))
        .await
        .unwrap();
    let future = EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Future", at(28, 10)))
        .await
        .unwrap();
    for e in [old, future] {
        EventRepo::set_status(&mut tx, e, EventStatus::Proposed, None)
            .await
            .unwrap();
    }
    assert_eq!(EventRepo::expire_past(&mut tx, at(10, 0)).await.unwrap(), 1);
    assert_eq!(
        EventRepo::get_full(&mut tx, old)
            .await
            .unwrap()
            .unwrap()
            .event
            .status,
        Some(EventStatus::Expired)
    );
    assert_eq!(
        EventRepo::get_full(&mut tx, future)
            .await
            .unwrap()
            .unwrap()
            .event
            .status,
        Some(EventStatus::Proposed)
    );
    let near = EventRepo::dedupe_candidates(&mut tx, s.h, at(1, 0), at(5, 0))
        .await
        .unwrap();
    assert!(near.is_empty(), "expired rows are not candidates");
    assert!(EventRepo::delete(&mut tx, future).await.unwrap());
    assert!(!EventRepo::delete(&mut tx, future).await.unwrap());
    assert!(matches!(
        EventRepo::set_route(&mut tx, future, None, None).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn email_tasks_lists_and_inbox_counts(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let b = seed(&store, "B").await;
    let src = add_source(&store, s.h, &[s.adult]).await;
    let msg = add_msg(&store, s.h, src, "m", Some("school"), at(20, 9)).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    assert_eq!(
        InboxRepo::counts(&mut tx).await.unwrap(),
        InboxCounts::default()
    );

    let defaults = TaskList {
        kind: TaskListKind::Email,
        ..list_defaults(s.h)
    };
    let l1 = TaskListRepo::get_or_create(&mut tx, TaskListKind::Email, "Email", &defaults)
        .await
        .unwrap();
    let l2 = TaskListRepo::get_or_create(&mut tx, TaskListKind::Email, "Email", &defaults)
        .await
        .unwrap();
    assert_eq!(l1.value.id, l2.value.id);
    assert_eq!(l1.value.kind, TaskListKind::Email);
    assert_eq!(l1.value.name, "Email");
    assert_eq!(l1.value.calendar_default, CalendarDisplay::UntilDone);
    let l3 = TaskListRepo::get_or_create(&mut tx, TaskListKind::General, "Bills", &defaults)
        .await
        .unwrap();
    assert_ne!(l3.value.id, l1.value.id);
    assert_eq!(l3.value.kind, TaskListKind::General);

    let suggested = email_task_row(s.h, l1.value.id, msg, TaskStatus::Suggested);
    let open = email_task_row(s.h, l1.value.id, msg, TaskStatus::Open);
    for t in [&suggested, &open] {
        TaskRepo::insert(&mut tx, t, &[], &[])
            .await
            .unwrap()
            .unwrap();
    }
    let derived = TaskRepo::derived_for_message(&mut tx, msg).await.unwrap();
    assert_eq!(derived.len(), 2);
    assert!(
        derived
            .iter()
            .any(|d| d.0 == suggested.id && d.2 == TaskStatus::Suggested)
    );
    assert!(
        TaskRepo::derived_for_message(&mut tx, MessageId::new())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        TaskRepo::suggested_email_older_than(&mut tx, Utc::now() - Duration::days(1))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        TaskRepo::suggested_email_older_than(&mut tx, Utc::now() + Duration::days(1))
            .await
            .unwrap(),
        vec![suggested.id]
    );

    // counts: suggested email task, proposed bill, proposed contact, proposed event
    let o = OrgRepo::insert(&mut tx, &org(s.h, "Example School"))
        .await
        .unwrap();
    ContactSuggestionRepo::insert_if_new(
        &mut tx,
        &ContactSuggestion {
            id: ContactSuggestionId::new(),
            household_id: s.h,
            org_id: o.value.id,
            email: "c@example.edu".into(),
            name: None,
            member_id: None,
            message_id: None,
            status: SuggestionStatus::Proposed,
            decided_by: None,
            decided_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    BillRepo::insert_if_new(
        &mut tx,
        &bill(s.h, "Example Power", 100, BillStatus::Proposed),
    )
    .await
    .unwrap();
    BillRepo::insert_if_new(&mut tx, &bill(s.h, "Example Water", 100, BillStatus::Open))
        .await
        .unwrap();
    let ev = EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Concert", at(30, 18)))
        .await
        .unwrap();
    EventRepo::set_status(&mut tx, ev, EventStatus::Proposed, None)
        .await
        .unwrap();
    EventRepo::insert_extracted(&mut tx, &extracted(s.h, "Not yet routed", at(30, 19)))
        .await
        .unwrap();
    let c = InboxRepo::counts(&mut tx).await.unwrap();
    assert_eq!(
        (c.event, c.bill, c.task, c.contact, c.total),
        (1, 1, 1, 1, 4)
    );
    tx.commit().await.unwrap();
    // other households do not see them
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert_eq!(InboxRepo::counts(&mut tx).await.unwrap().total, 0);
}

async fn send(tx: &mut Tx, subject: uuid::Uuid) -> Result<u64> {
    NotificationRepo::insert_for_adults(
        tx,
        NotifyTrigger::ApprovalNeeded,
        "event",
        Some(subject),
        "New event to approve",
        "Concert",
        Some("/inbox"),
        &format!("approval:event:{subject}"),
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
async fn insert_for_adults_dedupes(pool: PgPool) {
    let store = Store::from_pool(pool);
    let a = seed(&store, "A").await;
    let b = seed(&store, "B").await;
    let mut tx = store.tx_for(a.h).await.unwrap();
    let subject = uuid::Uuid::now_v7();
    assert_eq!(
        send(&mut tx, subject).await.unwrap(),
        2,
        "two adults, not the child"
    );
    assert_eq!(send(&mut tx, subject).await.unwrap(), 0, "deduped");
    let rows: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT member_id, trigger, dedupe_key FROM notification ORDER BY dedupe_key",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.1 == "approval_needed"));
    let members: std::collections::HashSet<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(
        members,
        [a.adult.as_uuid(), a.adult2.as_uuid()]
            .into_iter()
            .collect()
    );
    assert!(!members.contains(&a.child.as_uuid()));
    assert!(
        rows.iter()
            .all(|r| r.2.starts_with(&format!("approval:event:{subject}:")))
    );
    // an archived adult is skipped
    MemberRepo::archive(&mut tx, a.adult2).await.unwrap();
    let other = uuid::Uuid::now_v7();
    let n = NotificationRepo::insert_for_adults(
        &mut tx,
        NotifyTrigger::MailTimeSensitive,
        "message",
        Some(other),
        "t",
        "b",
        None,
        &format!("mts:{other}"),
    )
    .await
    .unwrap();
    assert_eq!(n, 1);
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(b.h).await.unwrap();
    assert_eq!(count(&mut tx, "notification").await, 0);
}

async fn add_source_vis(
    store: &Store,
    h: HouseholdId,
    owners: &[MemberId],
    visibility: MailVisibility,
) -> MailSourceId {
    let mut s = source(h, "Shared mail");
    s.visibility = visibility;
    let mut tx = store.tx_for(h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &s, owners).await.unwrap();
    tx.commit().await.unwrap();
    s.id
}

#[sqlx::test(migrations = "./migrations")]
async fn pending_by_adult_follows_source_scope(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let own = add_source(&store, s.h, &[s.adult]).await;
    let shared = add_source_vis(&store, s.h, &[s.adult2], MailVisibility::Household).await;
    let m_own = add_msg(&store, s.h, own, "own-1", None, at(20, 9)).await;
    let m_shared = add_msg(&store, s.h, shared, "shared-1", None, at(20, 10)).await;

    let mut tx = store.tx_for(s.h).await.unwrap();
    let with_msg = |payee: &str, amount: i64, m: Option<MessageId>| {
        let mut b = bill(s.h, payee, amount, BillStatus::Proposed);
        b.message_id = m;
        b
    };
    for b in [
        with_msg("Own Power", 100, Some(m_own)),
        with_msg("Shared Water", 200, Some(m_shared)),
        with_msg("Orphan Gas", 300, None),
    ] {
        BillRepo::insert_if_new(&mut tx, &b).await.unwrap().unwrap();
    }
    let l = TaskListRepo::get_or_create(
        &mut tx,
        TaskListKind::Email,
        "Email",
        &TaskList {
            kind: TaskListKind::Email,
            ..list_defaults(s.h)
        },
    )
    .await
    .unwrap();
    TaskRepo::insert(
        &mut tx,
        &email_task_row(s.h, l.value.id, m_own, TaskStatus::Suggested),
        &[],
        &[],
    )
    .await
    .unwrap()
    .unwrap();
    let o = OrgRepo::insert(&mut tx, &org(s.h, "Example School"))
        .await
        .unwrap();
    ContactSuggestionRepo::insert_if_new(
        &mut tx,
        &ContactSuggestion {
            id: ContactSuggestionId::new(),
            household_id: s.h,
            org_id: o.value.id,
            email: "c@example.edu".into(),
            name: None,
            member_id: None,
            message_id: Some(m_shared),
            status: SuggestionStatus::Proposed,
            decided_by: None,
            decided_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();

    let got: std::collections::HashMap<_, _> = InboxRepo::pending_by_adult(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(got.len(), 2, "adults only, the child is not listed");
    // owner of the owners-only source: own bill + own task + shared bill + shared contact + orphan bill
    assert_eq!(got[&s.adult], 5);
    // the other adult: household-source items and the orphan only
    assert_eq!(got[&s.adult2], 3);

    // an archived source reaches every adult
    sqlx::query("UPDATE mail_source SET archived_at = now() WHERE id = $1")
        .bind(own.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    let got: std::collections::HashMap<_, _> = InboxRepo::pending_by_adult(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!((got[&s.adult], got[&s.adult2]), (5, 5));
    tx.commit().await.unwrap();
}

async fn send_src(tx: &mut Tx, src: Option<MailSourceId>, key: &str) -> u64 {
    NotificationRepo::insert_for_source_adults(
        tx,
        src,
        NotifyTrigger::ApprovalNeeded,
        "event",
        None,
        "t",
        "b",
        None,
        key,
    )
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn insert_for_source_adults_scopes_recipients(pool: PgPool) {
    let store = Store::from_pool(pool);
    let s = seed(&store, "A").await;
    let own = add_source(&store, s.h, &[s.adult]).await;
    let shared = add_source_vis(&store, s.h, &[s.adult], MailVisibility::Household).await;
    let mut tx = store.tx_for(s.h).await.unwrap();
    assert_eq!(
        send_src(&mut tx, Some(own), "k1").await,
        1,
        "owners-only: the owner"
    );
    assert_eq!(
        send_src(&mut tx, Some(shared), "k2").await,
        2,
        "household: every adult"
    );
    assert_eq!(
        send_src(&mut tx, None, "k3").await,
        2,
        "no source: every adult"
    );
    assert_eq!(send_src(&mut tx, None, "k3").await, 0, "deduped");
    sqlx::query("UPDATE mail_source SET archived_at = now() WHERE id = $1")
        .bind(own.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        send_src(&mut tx, Some(own), "k4").await,
        2,
        "archived: every adult"
    );
    let to_owner: i64 =
        sqlx::query_scalar("SELECT count(*) FROM notification WHERE dedupe_key = $1")
            .bind(format!("k1:{}", s.adult))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(to_owner, 1);
    tx.commit().await.unwrap();
}
