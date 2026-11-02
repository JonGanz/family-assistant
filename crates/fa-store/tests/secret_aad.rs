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

fn key() -> MasterKey {
    MasterKey::from_bytes(&[5u8; 32], 1).unwrap()
}

async fn seed(store: &Store) -> HouseholdId {
    let h = household("Aad");
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    tx.commit().await.unwrap();
    h.id
}

async fn put(store: &Store, h: HouseholdId, owner: uuid::Uuid) {
    let mut tx = store.tx_for(h).await.unwrap();
    SecretRepo::put(
        &mut tx,
        &key(),
        h,
        "mail_source",
        owner,
        SecretKind::OauthToken,
        b"tok",
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn get(store: &Store, h: HouseholdId, owner: uuid::Uuid) -> Result<Option<Vec<u8>>> {
    let mut tx = store.tx_for(h).await.unwrap();
    SecretRepo::get(
        &mut tx,
        &key(),
        h,
        "mail_source",
        owner,
        SecretKind::OauthToken,
    )
    .await
}

#[test]
fn sealed_secret_needs_its_aad() {
    let h = HouseholdId::new();
    let owner = uuid::Uuid::now_v7();
    let aad = secret_aad(h, "mail_source", owner, SecretKind::OauthToken);
    let other = secret_aad(
        h,
        "mail_source",
        uuid::Uuid::now_v7(),
        SecretKind::OauthToken,
    );
    assert!(
        String::from_utf8(aad.clone())
            .unwrap()
            .starts_with("secret:v1|")
    );
    let sealed = key().seal(b"tok", &aad).unwrap();
    assert_eq!(key().open(&sealed, &aad).unwrap(), b"tok");
    assert!(key().open(&sealed, &other).is_err());
    assert!(key().open(&sealed, &[]).is_err());
    assert_eq!(
        instance_setting_aad("ntfy_token"),
        b"instance_setting:v1|ntfy_token"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn moved_row_no_longer_opens(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let owner = uuid::Uuid::now_v7();
    put(&store, h, owner).await;
    assert_eq!(
        get(&store, h, owner).await.unwrap().as_deref(),
        Some(&b"tok"[..])
    );

    let other = uuid::Uuid::now_v7();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("UPDATE secret SET owner_id = $1 WHERE owner_id = $2")
        .bind(other)
        .bind(owner)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        get(&store, h, other).await,
        Err(StoreError::Crypto(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn rebind_all_binds_version_zero_rows(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store).await;
    let owner = uuid::Uuid::now_v7();
    let sealed = key().seal(b"old", &[]).unwrap();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query(
        "INSERT INTO secret (id, household_id, owner_type, owner_id, kind, ciphertext, nonce, wrapped_dek, key_version)
         VALUES ($1, $2, 'mail_source', $3, 'oauth_token', $4, $5, $6, $7)",
    )
    .bind(uuid::Uuid::now_v7())
    .bind(h.as_uuid())
    .bind(owner)
    .bind(&sealed.ciphertext)
    .bind(&sealed.nonce)
    .bind(&sealed.wrapped_dek)
    .bind(sealed.key_version)
    .execute(&mut *tx)
    .await
    .unwrap();
    InstanceSettingRepo::set_secret(&mut tx, "ntfy_token", &key(), Some("tk_example"))
        .await
        .unwrap();
    let legacy = key().seal(b"legacy", &[]).unwrap();
    sqlx::query(
        "INSERT INTO instance_setting (key, ciphertext, nonce, wrapped_dek, key_version)
         VALUES ('smtp_password', $1, $2, $3, $4)",
    )
    .bind(&legacy.ciphertext)
    .bind(&legacy.nonce)
    .bind(&legacy.wrapped_dek)
    .bind(legacy.key_version)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // The version-0 fallback still opens it.
    assert_eq!(
        get(&store, h, owner).await.unwrap().as_deref(),
        Some(&b"old"[..])
    );

    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(SecretRepo::rebind_all(&mut tx, &key()).await.unwrap(), 1);
    assert_eq!(
        InstanceSettingRepo::rebind_all(&mut tx, &key())
            .await
            .unwrap(),
        1
    );
    tx.commit().await.unwrap();

    let mut tx = store.tx_system().await.unwrap();
    let v: i16 = sqlx::query_scalar("SELECT aad_version FROM secret WHERE owner_id = $1")
        .bind(owner)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(v, 1);
    assert_eq!(
        InstanceSettingRepo::secret(&mut tx, "smtp_password", &key())
            .await
            .unwrap()
            .as_deref(),
        Some("legacy")
    );
    assert_eq!(SecretRepo::rebind_all(&mut tx, &key()).await.unwrap(), 0);
    assert_eq!(
        InstanceSettingRepo::rebind_all(&mut tx, &key())
            .await
            .unwrap(),
        0
    );
    tx.commit().await.unwrap();
    assert_eq!(
        get(&store, h, owner).await.unwrap().as_deref(),
        Some(&b"old"[..])
    );

    // Bound now: moving the row breaks it.
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("UPDATE secret SET owner_id = $1")
        .bind(uuid::Uuid::now_v7())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(get(&store, h, owner).await.unwrap().is_none());
}
