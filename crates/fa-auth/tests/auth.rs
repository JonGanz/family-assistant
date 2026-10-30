use std::time::Duration;

use chrono::Utc;
use fa_auth::{AuthConfig, AuthError, AuthService, PgSessionStore, hash_password};
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;
use tower_sessions::cookie::time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::SessionStore;

const ADULT_PW: &str = "adult password 123";
const CHILD_PW: &str = "kid1";

struct Fx {
    store: Store,
    h: HouseholdId,
    adult: MemberId,
    child: MemberId,
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

async fn seed(pool: PgPool) -> Fx {
    let store = Store::from_pool(pool);
    let h = HouseholdId::new();
    let adult = member(h, "Alex", Role::Adult);
    let child = member(h, "Sam", Role::Child);
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(
        &mut tx,
        &Household {
            dashboard_pending_approvals: true,
            setup_completed_at: None,
            id: h,
            name: "Example".into(),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    for (m, user, pw) in [(&adult, "alex", ADULT_PW), (&child, "sam", CHILD_PW)] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
        UserAccountRepo::insert(
            &mut tx,
            &UserAccount {
                id: UserAccountId::new(),
                household_id: h,
                member_id: m.id,
                username: user.into(),
                password_hash: hash_password(pw).unwrap(),
                disabled_at: None,
                created_at: Utc::now(),
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    Fx {
        store,
        h,
        adult: adult.id,
        child: child.id,
    }
}

fn svc(fx: &Fx) -> AuthService {
    AuthService::new(fx.store.clone(), AuthConfig::default())
}

fn adult_actor(fx: &Fx) -> Actor {
    Actor::User {
        member_id: fx.adult,
        role: Role::Adult,
        household_id: fx.h,
    }
}

fn child_actor(fx: &Fx) -> Actor {
    Actor::User {
        member_id: fx.child,
        role: Role::Child,
        household_id: fx.h,
    }
}

async fn audit_actions(fx: &Fx) -> Vec<String> {
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    let list = AuditRepo::list(&mut tx, fx.h, 100).await.unwrap();
    list.into_iter().map(|e| e.action).collect()
}

async fn exec(fx: &Fx, sql: &str, id: &str) {
    let mut tx = fx.store.tx_system().await.unwrap();
    sqlx::query(sql).bind(id).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn login_success_and_failure(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let out = auth
        .login("Alex", ADULT_PW, None, Some("test"))
        .await
        .unwrap();
    assert_eq!(out.member_id, fx.adult);
    assert_eq!(out.role, Role::Adult);
    assert!(matches!(
        auth.login("alex", "wrong password!!", None, None).await,
        Err(AuthError::InvalidCredentials)
    ));
    assert!(matches!(
        auth.login("nobody", ADULT_PW, None, None).await,
        Err(AuthError::InvalidCredentials)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn account_locks_after_five_failures(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    for _ in 0..5 {
        assert!(matches!(
            auth.login("alex", "bad bad bad bad", None, None).await,
            Err(AuthError::InvalidCredentials)
        ));
    }
    assert!(matches!(
        auth.login("alex", ADULT_PW, None, None).await,
        Err(AuthError::Locked { .. })
    ));
    assert!(
        audit_actions(&fx)
            .await
            .contains(&"account.locked".to_string())
    );
    // other accounts are unaffected
    auth.login("sam", CHILD_PW, None, None).await.unwrap();
    // lock expiry
    let mut tx = fx.store.tx_system().await.unwrap();
    sqlx::query("UPDATE user_account SET locked_until = now() - interval '1 second'")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    auth.login("alex", ADULT_PW, None, None).await.unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn per_ip_limit(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = AuthService::new(
        fx.store.clone(),
        AuthConfig {
            ip_burst: 2,
            ip_refill: Duration::from_secs(3600),
            ..AuthConfig::default()
        },
    );
    let ip = "192.0.2.7".parse().ok();
    auth.login("alex", ADULT_PW, ip, None).await.unwrap();
    auth.login("alex", ADULT_PW, ip, None).await.unwrap();
    assert!(matches!(
        auth.login("alex", ADULT_PW, ip, None).await,
        Err(AuthError::RateLimited)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn session_ttl_by_role_and_rolling(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let a = auth.login("alex", ADULT_PW, None, None).await.unwrap();
    let c = auth.login("sam", CHILD_PW, None, None).await.unwrap();
    let days = |o: &fa_auth::LoginOutcome| (o.expires_at - Utc::now()).num_days();
    assert!((29..=30).contains(&days(&a)));
    assert!((179..=180).contains(&days(&c)));

    // rolling: shorten the session, resolving pushes it back out
    exec(
        &fx,
        "UPDATE session SET expires_at = now() + interval '1 day' WHERE id = $1",
        &a.session_id,
    )
    .await;
    let actor = auth.resolve_actor(Some(&a.session_id), None).await.unwrap();
    assert_eq!(actor, adult_actor(&fx));
    let mut tx = fx.store.tx_system().await.unwrap();
    let rec = SessionRepo::get(&mut tx, &a.session_id)
        .await
        .unwrap()
        .unwrap();
    assert!((rec.expires_at - Utc::now()).num_days() >= 29);
    tx.rollback().await.unwrap();

    assert_eq!(
        auth.resolve_actor(Some(&c.session_id), None).await.unwrap(),
        child_actor(&fx)
    );
    auth.logout(&a.session_id).await.unwrap();
    assert!(matches!(
        auth.resolve_actor(Some(&a.session_id), None).await,
        Err(AuthError::Unauthenticated)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn session_list_and_revoke(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let c = auth
        .login("sam", CHILD_PW, None, Some("kid phone"))
        .await
        .unwrap();
    auth.login("alex", ADULT_PW, None, None).await.unwrap();

    let list = auth
        .list_sessions(&adult_actor(&fx), fx.child)
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_ne!(list[0].handle, c.session_id);
    // a child cannot list an adult's sessions, nor revoke them
    assert!(matches!(
        auth.list_sessions(&child_actor(&fx), fx.adult).await,
        Err(AuthError::Forbidden(_))
    ));
    auth.revoke_session(&adult_actor(&fx), fx.child, &list[0].handle)
        .await
        .unwrap();
    assert!(matches!(
        auth.resolve_actor(Some(&c.session_id), None).await,
        Err(AuthError::Unauthenticated)
    ));
    assert!(
        audit_actions(&fx)
            .await
            .contains(&"session.revoke".to_string())
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn device_pair_authenticate_revoke(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = AuthService::new(
        fx.store.clone(),
        AuthConfig {
            public_base_url: "https://home.example.com/".into(),
            ..AuthConfig::default()
        },
    );
    assert!(matches!(
        auth.pair_device(&child_actor(&fx), "kitchen").await,
        Err(AuthError::Forbidden(_))
    ));
    let paired = auth
        .pair_device(&adult_actor(&fx), "kitchen")
        .await
        .unwrap();
    assert_eq!(
        paired.pair_url,
        format!("https://home.example.com/dashboard?device={}", paired.token)
    );
    assert_eq!(paired.token.len(), 43); // 32 bytes, base64url, no padding
    assert_ne!(paired.device.token_hash, paired.token);

    let d = auth.authenticate_bearer(&paired.token).await.unwrap();
    assert_eq!(d.id, paired.device.id);
    assert!(d.last_seen.is_none(), "first read predates the touch");
    let d2 = auth.authenticate_bearer(&paired.token).await.unwrap();
    let seen = d2.last_seen.expect("touched");
    // within a minute: not touched again
    let d3 = auth.authenticate_bearer(&paired.token).await.unwrap();
    assert_eq!(d3.last_seen, Some(seen));
    assert!(matches!(
        auth.authenticate_bearer("garbage").await,
        Err(AuthError::Unauthenticated)
    ));

    assert!(matches!(
        auth.resolve_actor(None, Some(&paired.token)).await.unwrap(),
        Actor::Device { device_id, .. } if device_id == paired.device.id
    ));
    // A dashboard opened in a logged-in browser sends both: the bearer wins.
    let session = auth.login("alex", ADULT_PW, None, None).await.unwrap();
    assert!(matches!(
        auth.resolve_actor(Some(&session.session_id), Some(&paired.token)).await.unwrap(),
        Actor::Device { device_id, .. } if device_id == paired.device.id
    ));
    auth.revoke_device(&adult_actor(&fx), paired.device.id)
        .await
        .unwrap();
    assert!(matches!(
        auth.authenticate_bearer(&paired.token).await,
        Err(AuthError::Unauthenticated)
    ));
    let actions = audit_actions(&fx).await;
    assert!(actions.contains(&"device.pair".to_string()));
    assert!(actions.contains(&"device.revoke".to_string()));
}

async fn device_with_pin(fx: &Fx, auth: &AuthService) -> (Device, String) {
    auth.set_pin(&adult_actor(fx), fx.adult, "4321")
        .await
        .unwrap();
    let p = auth.pair_device(&adult_actor(fx), "hall").await.unwrap();
    (p.device, p.token)
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pin_set_rules_and_elevation(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    // validation and policy
    assert!(matches!(
        auth.set_pin(&adult_actor(&fx), fx.adult, "12").await,
        Err(AuthError::Pin(_))
    ));
    assert!(matches!(
        auth.set_pin(&child_actor(&fx), fx.child, "1234").await,
        Err(AuthError::Forbidden(_))
    ));
    let (device, token) = device_with_pin(&fx, &auth).await;
    assert!(matches!(
        auth.elevate(&device, fx.adult, "0000").await,
        Err(AuthError::InvalidCredentials)
    ));
    // children cannot be elevated to
    assert!(matches!(
        auth.elevate(&device, fx.child, "1234").await,
        Err(AuthError::Forbidden(_))
    ));
    let e = auth.elevate(&device, fx.adult, "4321").await.unwrap();
    assert!((e.expires_at - Utc::now()).num_seconds() > 280);
    assert!(matches!(
        auth.resolve_actor(None, Some(&token)).await.unwrap(),
        Actor::ElevatedDevice { adult, .. } if adult == fx.adult
    ));
    let actions = audit_actions(&fx).await;
    assert!(actions.contains(&"pin.set".to_string()));
    assert!(actions.contains(&"device.elevate".to_string()));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn elevation_expires(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = AuthService::new(
        fx.store.clone(),
        AuthConfig {
            elevation_ttl: Duration::ZERO,
            ..AuthConfig::default()
        },
    );
    let (device, token) = device_with_pin(&fx, &auth).await;
    auth.elevate(&device, fx.adult, "4321").await.unwrap();
    assert!(matches!(
        auth.resolve_actor(None, Some(&token)).await.unwrap(),
        Actor::Device { .. }
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pin_locks_after_five_failures(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let (device, _) = device_with_pin(&fx, &auth).await;
    for _ in 0..5 {
        assert!(matches!(
            auth.elevate(&device, fx.adult, "9999").await,
            Err(AuthError::InvalidCredentials)
        ));
    }
    // even the right PIN is refused while locked
    assert!(matches!(
        auth.elevate(&device, fx.adult, "4321").await,
        Err(AuthError::Locked { .. })
    ));
    assert!(audit_actions(&fx).await.contains(&"pin.locked".to_string()));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pg_session_store_roundtrip(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let store = PgSessionStore::new(fx.store.clone(), AuthConfig::default());
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    let account = UserAccountRepo::get_by_member(&mut tx, fx.child)
        .await
        .unwrap()
        .unwrap();
    tx.rollback().await.unwrap();

    let mut rec = Record {
        id: Id::default(),
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc(),
    };
    // anonymous sessions are not persisted
    store.create(&mut rec).await.unwrap();
    assert!(store.load(&rec.id).await.unwrap().is_none());

    for (k, v) in [
        ("household_id", fx.h.to_string()),
        ("member_id", fx.child.to_string()),
        ("account_id", account.value.id.to_string()),
    ] {
        rec.data.insert(k.into(), v.into());
    }
    rec.data.insert("flash".into(), "hi".into());
    store.create(&mut rec).await.unwrap();
    let loaded = store.load(&rec.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.data.get("flash").and_then(|v| v.as_str()),
        Some("hi")
    );
    // child TTL, regardless of the layer's expiry_date
    let left = loaded.expiry_date - OffsetDateTime::now_utc();
    assert!(left.whole_days() >= 179);
    // the session authenticates through the service
    assert_eq!(
        auth.resolve_actor(Some(&rec.id.to_string()), None)
            .await
            .unwrap(),
        child_actor(&fx)
    );
    store.delete(&rec.id).await.unwrap();
    assert!(store.load(&rec.id).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn device_pin_budget_comes_from_the_config(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = AuthService::new(
        fx.store.clone(),
        AuthConfig {
            device_pin_burst: 2,
            device_pin_refill: Duration::from_secs(3600),
            max_pin_failures: 50,
            ..AuthConfig::default()
        },
    );
    let (device, _) = device_with_pin(&fx, &auth).await;
    for _ in 0..2 {
        assert!(matches!(
            auth.elevate(&device, fx.adult, "9999").await,
            Err(AuthError::InvalidCredentials)
        ));
    }
    assert!(matches!(
        auth.elevate(&device, fx.adult, "9999").await,
        Err(AuthError::RateLimited)
    ));
    // Another service instance has its own budget (no process-wide state).
    let other = AuthService::new(fx.store.clone(), AuthConfig::default());
    assert!(matches!(
        other.elevate(&device, fx.adult, "9999").await,
        Err(AuthError::InvalidCredentials)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn correct_pins_do_not_use_up_the_device_budget(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = AuthService::new(
        fx.store.clone(),
        AuthConfig {
            device_pin_burst: 1,
            device_pin_refill: Duration::from_secs(3600),
            ..AuthConfig::default()
        },
    );
    let (device, _) = device_with_pin(&fx, &auth).await;
    for _ in 0..4 {
        auth.elevate(&device, fx.adult, "4321").await.unwrap();
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn parallel_wrong_logins_are_serialised_by_the_lockout(pool: PgPool) {
    let fx = seed(pool).await;
    let auth = svc(&fx);
    let results = futures_join(
        (0..8)
            .map(|_| {
                let auth = auth.clone();
                tokio::spawn(
                    async move { auth.login("alex", "wrong wrong wrong", None, None).await },
                )
            })
            .collect(),
    )
    .await;
    let invalid = results
        .iter()
        .filter(|r| matches!(r, Err(AuthError::InvalidCredentials)))
        .count();
    let locked = results
        .iter()
        .filter(|r| matches!(r, Err(AuthError::Locked { .. })))
        .count();
    assert_eq!(invalid + locked, results.len());
    // Exactly the configured number of failures were recorded before the lock held.
    assert_eq!(invalid, 5, "{results:?}");
    assert!(matches!(
        auth.login("alex", "adult password 123", None, None).await,
        Err(AuthError::Locked { .. })
    ));
}

async fn futures_join<T: Send + 'static>(handles: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
    let mut out = Vec::new();
    for h in handles {
        out.push(h.await.unwrap());
    }
    out
}
