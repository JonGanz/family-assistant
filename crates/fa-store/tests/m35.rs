//! M3.5: instance settings, setup state, household columns, lockout races.

use chrono::Utc;
use fa_domain::api::HouseholdPatch;
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
        units: Units::Metric,
        week_start: Default::default(),
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
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

fn adult(h: HouseholdId) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: "Ada Example".into(),
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
    }
}

fn key(byte: u8) -> MasterKey {
    MasterKey::from_bytes(&[byte; 32], 1).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn instance_settings_round_trip(pool: PgPool) {
    let store = Store::from_pool(pool);
    let mut tx = store.tx_system().await.unwrap();
    let url = serde_json::json!("https://fa.example.com");
    InstanceSettingRepo::set(&mut tx, "public_base_url", Some(&url))
        .await
        .unwrap();
    InstanceSettingRepo::set_secret(&mut tx, "ntfy_token", &key(1), Some("tk_example"))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = store.tx_system().await.unwrap();
    let all = InstanceSettingRepo::all(&mut tx).await.unwrap();
    assert_eq!(all.len(), 2);
    let token = all.iter().find(|r| r.key == "ntfy_token").unwrap();
    assert!(token.has_secret && token.value.is_none());
    let base = all.iter().find(|r| r.key == "public_base_url").unwrap();
    assert!(!base.has_secret && base.value.as_ref() == Some(&url));
    assert_eq!(
        InstanceSettingRepo::secret(&mut tx, "ntfy_token", &key(1))
            .await
            .unwrap()
            .as_deref(),
        Some("tk_example")
    );
    // wrong key: error, never the plaintext
    assert!(
        InstanceSettingRepo::secret(&mut tx, "ntfy_token", &key(2))
            .await
            .is_err()
    );
    // the stored bytes do not contain the plaintext
    let ct: Vec<u8> =
        sqlx::query_scalar("SELECT ciphertext FROM instance_setting WHERE key = 'ntfy_token'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(!ct.windows(5).any(|w| w == b"tk_ex"));
    // not a secret / absent
    assert!(
        InstanceSettingRepo::secret(&mut tx, "public_base_url", &key(1))
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        InstanceSettingRepo::secret(&mut tx, "nope", &key(1))
            .await
            .unwrap()
            .is_none()
    );
    // overwrite a secret with a plain value clears the crypto columns; None resets
    InstanceSettingRepo::set(&mut tx, "ntfy_token", Some(&serde_json::json!("x")))
        .await
        .unwrap();
    let row = InstanceSettingRepo::all(&mut tx)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.key == "ntfy_token")
        .unwrap();
    assert!(!row.has_secret);
    InstanceSettingRepo::set(&mut tx, "ntfy_token", None)
        .await
        .unwrap();
    InstanceSettingRepo::set_secret(&mut tx, "public_base_url", &key(1), None)
        .await
        .unwrap();
    assert!(InstanceSettingRepo::all(&mut tx).await.unwrap().is_empty());
    tx.commit().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn settings_write_notifies(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let mut listener = sqlx::postgres::PgListener::connect_with(&pool)
        .await
        .unwrap();
    listener.listen("fa_settings").await.unwrap();
    let mut tx = store.tx_system().await.unwrap();
    InstanceSettingRepo::set(&mut tx, "allow_http_feeds", Some(&serde_json::json!(true)))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let n = tokio::time::timeout(std::time::Duration::from_secs(5), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(n.channel(), "fa_settings");
}

#[sqlx::test(migrations = "./migrations")]
async fn setup_state_and_household_columns(pool: PgPool) {
    let store = Store::from_pool(pool);
    let mut tx = store.tx_system().await.unwrap();
    let st = SetupRepo::state(&mut tx).await.unwrap();
    assert_eq!(st.household, None);
    assert!(!st.has_adult_account && st.completed_at.is_none());
    let hh = household("A");
    let v = HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    assert!(v.value.dashboard_pending_approvals && v.value.setup_completed_at.is_none());
    let st = SetupRepo::state(&mut tx).await.unwrap();
    assert_eq!(st.household, Some(hh.id));
    assert!(!st.has_adult_account);
    let m = adult(hh.id);
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    UserAccountRepo::insert(
        &mut tx,
        &UserAccount {
            id: UserAccountId::new(),
            household_id: hh.id,
            member_id: m.id,
            username: "ada".into(),
            password_hash: "hash".into(),
            disabled_at: None,
            created_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    assert!(SetupRepo::state(&mut tx).await.unwrap().has_adult_account);
    SetupRepo::complete(&mut tx, hh.id).await.unwrap();
    let first = SetupRepo::state(&mut tx)
        .await
        .unwrap()
        .completed_at
        .unwrap();
    SetupRepo::complete(&mut tx, hh.id).await.unwrap();
    assert_eq!(
        SetupRepo::state(&mut tx).await.unwrap().completed_at,
        Some(first),
        "keeps the first time"
    );
    assert!(
        SetupRepo::complete(&mut tx, HouseholdId::new())
            .await
            .is_err()
    );
    tx.commit().await.unwrap();

    // the new patch field is written and read back
    let mut tx = store.tx_for(hh.id).await.unwrap();
    let cur = HouseholdRepo::get(&mut tx, hh.id).await.unwrap().unwrap();
    assert_eq!(cur.value.setup_completed_at, Some(first));
    HouseholdRepo::update_settings(
        &mut tx,
        hh.id,
        &HouseholdPatch {
            dashboard_pending_approvals: Some(false),
            ..Default::default()
        },
        cur.version,
    )
    .await
    .unwrap();
    let cur = HouseholdRepo::get(&mut tx, hh.id).await.unwrap().unwrap();
    assert!(!cur.value.dashboard_pending_approvals);
    tx.commit().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn pin_lockout_counts_parallel_failures(pool: PgPool) {
    let store = Store::from_pool(pool);
    let hh = household("A");
    let m = adult(hh.id);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    tx.commit().await.unwrap();

    // Each attempt reads the lockout (locking the row), then records a failure.
    let attempt = |store: Store, id: MemberId| async move {
        let mut tx = store.tx_system().await.unwrap();
        let before = MemberRepo::pin_lockout(&mut tx, id).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        MemberRepo::record_pin_failure(&mut tx, id, 5, 900)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        before.failed_attempts
    };
    let (a, b) = tokio::join!(attempt(store.clone(), m.id), attempt(store.clone(), m.id));
    let mut seen = [a, b];
    seen.sort();
    assert_eq!(
        seen,
        [0, 1],
        "the second attempt saw the first one's failure"
    );
    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(
        MemberRepo::pin_lockout(&mut tx, m.id)
            .await
            .unwrap()
            .failed_attempts,
        2
    );
    // once locked, further failures leave the counters alone
    for _ in 0..3 {
        MemberRepo::record_pin_failure(&mut tx, m.id, 5, 900)
            .await
            .unwrap();
    }
    let l = MemberRepo::record_pin_failure(&mut tx, m.id, 5, 900)
        .await
        .unwrap();
    assert!(l.locked_until.unwrap() > Utc::now());
    let again = MemberRepo::record_pin_failure(&mut tx, m.id, 5, 900)
        .await
        .unwrap();
    assert_eq!(again.locked_until, l.locked_until);
    tx.commit().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn login_lockout_counts_parallel_failures(pool: PgPool) {
    let store = Store::from_pool(pool);
    let hh = household("A");
    let m = adult(hh.id);
    let acct = UserAccount {
        id: UserAccountId::new(),
        household_id: hh.id,
        member_id: m.id,
        username: "ada".into(),
        password_hash: "hash".into(),
        disabled_at: None,
        created_at: Utc::now(),
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    UserAccountRepo::insert(&mut tx, &acct).await.unwrap();
    tx.commit().await.unwrap();

    let id = acct.id;
    let attempt = |store: Store| async move {
        let mut tx = store.tx_system().await.unwrap();
        let before = UserAccountRepo::lockout(&mut tx, id).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        UserAccountRepo::record_failure(&mut tx, id, 5, 900)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        before.failed_attempts
    };
    let (a, b) = tokio::join!(attempt(store.clone()), attempt(store.clone()));
    let mut seen = [a, b];
    seen.sort();
    assert_eq!(seen, [0, 1]);
    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(
        UserAccountRepo::lockout(&mut tx, id)
            .await
            .unwrap()
            .failed_attempts,
        2
    );
    for _ in 0..3 {
        UserAccountRepo::record_failure(&mut tx, id, 5, 900)
            .await
            .unwrap();
    }
    let l = UserAccountRepo::lockout(&mut tx, id).await.unwrap();
    assert!(l.locked_until.is_some());
    let again = UserAccountRepo::record_failure(&mut tx, id, 5, 900)
        .await
        .unwrap();
    assert_eq!(again.locked_until, l.locked_until);
    tx.commit().await.unwrap();
}
