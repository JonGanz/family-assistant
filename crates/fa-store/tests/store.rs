use chrono::Utc;
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;

fn household(name: &str) -> Household {
    Household {
        id: HouseholdId::new(),
        name: name.into(),
        tz: "UTC".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: fa_domain::Units::Metric,
        week_start: Default::default(),
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
        interests: vec!["chess".into()],
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

async fn seed(store: &Store, name: &str) -> (HouseholdId, MemberId) {
    let h = household(name);
    let m = member(h.id, &format!("{name} adult"), Role::Adult);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    MemberRepo::insert(&mut tx, &m, Some("adult1"), None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (h.id, m.id)
}

#[sqlx::test(migrations = "./migrations")]
async fn rls_isolates_households(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (a, ma) = seed(&store, "A").await;
    let (b, mb) = seed(&store, "B").await;

    let mut tx = store.tx_for(a).await.unwrap();
    let list = MemberRepo::list(&mut tx, true).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].value.id, ma);
    assert!(MemberRepo::get(&mut tx, mb).await.unwrap().is_none());
    // raw SQL cannot escape either
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM member")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM household")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 1);
    // cannot write into another household
    let evil = member(b, "evil", Role::Child);
    assert!(
        MemberRepo::insert(&mut tx, &evil, None, None)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();

    // a tx with no household sees nothing
    let mut tx = store.pool().begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE fa_app")
        .execute(&mut *tx)
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM member")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0);
    tx.rollback().await.unwrap();

    // system tx sees both
    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(HouseholdRepo::list(&mut tx).await.unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn audit_is_append_only(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, m) = seed(&store, "A").await;
    let entry = AuditEntry {
        id: AuditEntryId::new(),
        household_id: h,
        actor_member: Some(m),
        device: None,
        action: "member.create".into(),
        subject_type: "member".into(),
        subject_id: Some(m.to_string()),
        before: None,
        after: Some(serde_json::json!({"x": 1})),
        at: Utc::now(),
    };
    let mut tx = store.tx_for(h).await.unwrap();
    AuditRepo::insert(&mut tx, &entry).await.unwrap();
    tx.commit().await.unwrap();

    for sql in [
        "UPDATE audit_entry SET action = 'x'",
        "DELETE FROM audit_entry",
        "TRUNCATE audit_entry",
    ] {
        // as the app role: privilege error
        let mut tx = store.tx_for(h).await.unwrap();
        assert!(
            sqlx::query(sql).execute(&mut *tx).await.is_err(),
            "app role: {sql}"
        );
        tx.rollback().await.unwrap();
        // as the owner/superuser: the trigger still refuses
        let err = sqlx::query(sql).execute(store.pool()).await.unwrap_err();
        assert!(
            err.to_string().contains("append-only"),
            "owner: {sql}: {err}"
        );
    }
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(AuditRepo::list(&mut tx, h, 10).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn optimistic_concurrency(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, m) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let cur = MemberRepo::get(&mut tx, m).await.unwrap().unwrap();
    let mut edited = cur.value.clone();
    edited.display_name = "Renamed".into();
    let v2 = MemberRepo::update(&mut tx, &edited, cur.version)
        .await
        .unwrap();
    assert_eq!(v2, cur.version + 1);
    // stale version
    assert!(matches!(
        MemberRepo::update(&mut tx, &edited, cur.version).await,
        Err(StoreError::Conflict)
    ));
    // missing row
    let ghost = member(h, "ghost", Role::Child);
    assert!(matches!(
        MemberRepo::update(&mut tx, &ghost, 1).await,
        Err(StoreError::NotFound)
    ));
    MemberRepo::archive(&mut tx, m).await.unwrap();
    assert!(MemberRepo::list(&mut tx, false).await.unwrap().is_empty());
    assert_eq!(MemberRepo::list(&mut tx, true).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn config_key_unique_and_lockouts(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, m) = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(
        MemberRepo::get_by_config_key(&mut tx, h, "adult1")
            .await
            .unwrap()
            .unwrap()
            .value
            .id,
        m
    );
    let dup = member(h, "dup", Role::Adult);
    assert!(matches!(
        MemberRepo::insert(&mut tx, &dup, Some("adult1"), None).await,
        Err(StoreError::AlreadyExists)
    ));
    tx.rollback().await.unwrap();

    let mut tx = store.tx_for(h).await.unwrap();
    for i in 1..5 {
        let l = MemberRepo::record_pin_failure(&mut tx, m, 5, 900)
            .await
            .unwrap();
        assert_eq!(l.failed_attempts, i);
        assert!(l.locked_until.is_none());
    }
    let l = MemberRepo::record_pin_failure(&mut tx, m, 5, 900)
        .await
        .unwrap();
    assert!(l.locked_until.unwrap() > Utc::now());

    let acct = UserAccount {
        id: UserAccountId::new(),
        household_id: h,
        member_id: m,
        username: "Alice".into(),
        password_hash: "hash".into(),
        disabled_at: None,
        created_at: Utc::now(),
    };
    UserAccountRepo::insert(&mut tx, &acct).await.unwrap();
    tx.commit().await.unwrap();

    // login lookup before the household is known, case-insensitive
    let mut tx = store.tx_system().await.unwrap();
    let found = UserAccountRepo::find_by_username(&mut tx, None, "alice")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.value.household_id, h);
    let l = UserAccountRepo::record_failure(&mut tx, found.value.id, 1, 60)
        .await
        .unwrap();
    assert!(l.locked_until.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn devices_sessions_secrets_jobs(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (h, m) = seed(&store, "A").await;
    let key = MasterKey::from_bytes(&[3u8; 32], 1).unwrap();

    let dev = Device {
        id: DeviceId::new(),
        household_id: h,
        kind: DeviceKind::Dashboard,
        name: "Kitchen".into(),
        token_hash: "abc123".into(),
        last_seen: None,
        revoked_at: None,
        created_at: Utc::now(),
    };
    let mut tx = store.tx_for(h).await.unwrap();
    DeviceRepo::insert(&mut tx, &dev, Some(m)).await.unwrap();
    DeviceElevationRepo::create(
        &mut tx,
        &DeviceElevation {
            device_id: dev.id,
            member_id: m,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        },
        h,
    )
    .await
    .unwrap();
    assert!(
        DeviceElevationRepo::active(&mut tx, dev.id)
            .await
            .unwrap()
            .is_some()
    );
    let owner = uuid::Uuid::now_v7();
    SecretRepo::put(
        &mut tx,
        &key,
        h,
        "mail_source",
        owner,
        SecretKind::OauthToken,
        b"tok",
        Some(m),
    )
    .await
    .unwrap();
    let got = SecretRepo::get(
        &mut tx,
        &key,
        h,
        "mail_source",
        owner,
        SecretKind::OauthToken,
    )
    .await
    .unwrap();
    assert_eq!(got.as_deref(), Some(&b"tok"[..]));
    let raw: Vec<u8> = sqlx::query_scalar("SELECT ciphertext FROM secret")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_ne!(raw, b"tok");
    tx.commit().await.unwrap();

    let mut tx = store.tx_system().await.unwrap();
    let found = DeviceRepo::get_by_token_hash(&mut tx, "abc123")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.value.id, dev.id);
    // rotate
    let new = MasterKey::from_bytes(&[4u8; 32], 2).unwrap();
    assert_eq!(SecretRepo::rotate(&mut tx, &key, &new).await.unwrap(), 1);
    let got = SecretRepo::get(
        &mut tx,
        &new,
        h,
        "mail_source",
        owner,
        SecretKind::OauthToken,
    )
    .await
    .unwrap();
    assert_eq!(got.as_deref(), Some(&b"tok"[..]));

    let sess = SessionRecord {
        id: "sess-1".into(),
        household_id: h,
        member_id: m,
        account_id: {
            let a = UserAccount {
                id: UserAccountId::new(),
                household_id: h,
                member_id: m,
                username: "u".into(),
                password_hash: "x".into(),
                disabled_at: None,
                created_at: Utc::now(),
            };
            UserAccountRepo::insert(&mut tx, &a).await.unwrap().value.id
        },
        data: vec![1, 2, 3],
        user_agent: None,
        created_at: Utc::now(),
        last_seen: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(1),
    };
    SessionRepo::put(&mut tx, &sess).await.unwrap();
    assert!(SessionRepo::get(&mut tx, "sess-1").await.unwrap().is_some());
    assert_eq!(
        SessionRepo::list_for_member(&mut tx, m)
            .await
            .unwrap()
            .len(),
        1
    );

    let mut nj = NewJob::new("backup.verify");
    nj.idempotency_key = Some("k1".into());
    let id = JobRepo::enqueue(&mut tx, &nj).await.unwrap().unwrap();
    assert!(JobRepo::enqueue(&mut tx, &nj).await.unwrap().is_none());
    let claimed = JobRepo::claim(&mut tx, "w1", 60).await.unwrap().unwrap();
    assert_eq!(claimed.id, id);
    assert!(JobRepo::claim(&mut tx, "w2", 60).await.unwrap().is_none());
    JobRepo::complete(&mut tx, id).await.unwrap();

    SystemStatusRepo::set(&mut tx, "backup", &serde_json::json!({"ok": true}))
        .await
        .unwrap();
    assert!(
        SystemStatusRepo::get(&mut tx, "backup")
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn migrate_is_idempotent(pool: PgPool) {
    let store = Store::from_pool(pool);
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();
    assert!(store.migrations_applied().await.unwrap());
}
