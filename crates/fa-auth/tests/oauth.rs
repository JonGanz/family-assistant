use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use fa_auth::{
    AccountService, AuthConfig, AuthError, AuthService, DeviceService, OAuthPurpose, OAuthService,
};
use fa_connectors::Http;
use fa_connectors::google::oauth::GoogleOAuth;
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REFRESH: &str = "refresh-token-plaintext-1234567890";

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
            units: Units::Metric,
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
    for m in [&adult, &child] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
    }
    tx.commit().await.unwrap();
    Fx {
        store,
        h,
        adult: adult.id,
        child: child.id,
    }
}

fn key() -> MasterKey {
    MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
}

fn svc(fx: &Fx, server: Option<&MockServer>) -> OAuthService {
    let google = server.map(|s| {
        let mut g = GoogleOAuth::new("client-id", "client-secret");
        g.token_url = format!("{}/token", s.uri());
        g
    });
    OAuthService::new(
        fx.store.clone(),
        key(),
        google,
        "https://fa.example.com/",
        Http::new(true).unwrap(),
    )
}

fn adult(fx: &Fx) -> Actor {
    Actor::User {
        member_id: fx.adult,
        role: Role::Adult,
        household_id: fx.h,
    }
}

fn id_token(email: &str) -> String {
    let p = URL_SAFE_NO_PAD.encode(serde_json::json!({ "email": email }).to_string());
    format!("e30.{p}.sig")
}

async fn mock_exchange(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "access-1", "refresh_token": REFRESH, "expires_in": 3600,
            "scope": "openid email https://www.googleapis.com/auth/calendar.readonly",
            "id_token": id_token("alex@example.com"),
        })))
        .mount(server)
        .await;
}

fn state_of(url: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn start_callback_happy_path(pool: PgPool) {
    let fx = seed(pool.clone()).await;
    let server = MockServer::start().await;
    mock_exchange(&server).await;
    let s = svc(&fx, Some(&server));
    assert!(s.enabled());

    let url = s
        .start(
            &adult(&fx),
            "sess-1",
            Some("/settings"),
            OAuthPurpose::Calendar,
        )
        .await
        .unwrap();
    let u = url::Url::parse(&url).unwrap();
    let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["access_type"], "offline");
    assert_eq!(q["prompt"], "consent");
    assert_eq!(
        q["redirect_uri"],
        "https://fa.example.com/api/v1/oauth/google/callback"
    );
    assert!(q["scope"].contains("calendar.readonly") && q["scope"].contains("openid"));

    let (id, ret) = s
        .callback(&adult(&fx), "sess-1", "the-code", &state_of(&url))
        .await
        .unwrap();
    assert_eq!(ret.as_deref(), Some("/settings"));

    let accounts = s.list_accounts(&adult(&fx)).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, id);
    assert_eq!(accounts[0].email, "alex@example.com");
    assert_eq!(accounts[0].state, SourceState::Ok);

    // Refresh token is sealed: the plaintext is nowhere in the secret row.
    let row: (Vec<u8>, Vec<u8>, Vec<u8>) =
        sqlx::query_as("SELECT ciphertext, nonce, wrapped_dek FROM secret")
            .fetch_one(&pool)
            .await
            .unwrap();
    let needle = REFRESH.as_bytes();
    for blob in [&row.0, &row.1, &row.2] {
        assert!(!blob.windows(needle.len()).any(|w| w == needle));
    }
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    let acct = OAuthAccountRepo::get(&mut tx, id)
        .await
        .unwrap()
        .unwrap()
        .value;
    assert!(acct.secret_id.is_some());
    let opened = SecretRepo::get(
        &mut tx,
        &key(),
        fx.h,
        "oauth_account",
        id.as_uuid(),
        SecretKind::OauthToken,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(opened, REFRESH.as_bytes());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn bad_states_are_rejected(pool: PgPool) {
    let fx = seed(pool).await;
    let server = MockServer::start().await;
    mock_exchange(&server).await;
    let s = svc(&fx, Some(&server));
    let a = adult(&fx);

    // Reuse.
    let url = s
        .start(&a, "sess-1", None, OAuthPurpose::Calendar)
        .await
        .unwrap();
    let st = state_of(&url);
    s.callback(&a, "sess-1", "c", &st).await.unwrap();
    assert!(matches!(
        s.callback(&a, "sess-1", "c", &st).await,
        Err(AuthError::InvalidState)
    ));

    // Wrong session; the state is burnt even so.
    let url = s
        .start(&a, "sess-1", None, OAuthPurpose::Calendar)
        .await
        .unwrap();
    let st = state_of(&url);
    assert!(matches!(
        s.callback(&a, "other-session", "c", &st).await,
        Err(AuthError::InvalidState)
    ));
    assert!(matches!(
        s.callback(&a, "sess-1", "c", &st).await,
        Err(AuthError::InvalidState)
    ));

    // Expired.
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    OAuthPendingRepo::insert(
        &mut tx,
        "old-state",
        "sess-1",
        fx.adult,
        "v",
        None,
        Utc::now() - Duration::minutes(1),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        s.callback(&a, "sess-1", "c", "old-state").await,
        Err(AuthError::InvalidState)
    ));
    assert_eq!(s.cleanup_expired().await.unwrap(), 0, "already consumed");

    // Children cannot start the flow.
    let child = Actor::User {
        member_id: fx.child,
        role: Role::Child,
        household_id: fx.h,
    };
    assert!(matches!(
        s.start(&child, "sess-2", None, OAuthPurpose::Calendar)
            .await,
        Err(AuthError::Forbidden(_))
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn cleanup_removes_expired_pending(pool: PgPool) {
    let fx = seed(pool).await;
    let s = svc(&fx, None);
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    OAuthPendingRepo::insert(
        &mut tx,
        "old",
        "s",
        fx.adult,
        "v",
        None,
        Utc::now() - Duration::minutes(1),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(s.cleanup_expired().await.unwrap(), 1);
    assert_eq!(fa_auth::handlers()[0].kind(), "oauth.cleanup");
    assert_eq!(fa_auth::default_schedules()[0].1, "0 * * * *");
}

async fn connected(fx: &Fx, server: &MockServer) -> OAuthAccountId {
    mock_exchange(server).await;
    let s = svc(fx, Some(server));
    let url = s
        .start(&adult(fx), "s", None, OAuthPurpose::Calendar)
        .await
        .unwrap();
    s.callback(&adult(fx), "s", "c", &state_of(&url))
        .await
        .unwrap()
        .0
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn refresh_is_cached(pool: PgPool) {
    let fx = seed(pool).await;
    let server = MockServer::start().await;
    let id = connected(&fx, &server).await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "access_token": "access-2", "expires_in": 3600 }),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    // A fresh service has an empty cache.
    let s = svc(&fx, Some(&server));
    assert_eq!(s.access_token(fx.h, id).await.unwrap(), "access-2");
    assert_eq!(s.access_token(fx.h, id).await.unwrap(), "access-2");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn invalid_grant_sets_needs_reauth(pool: PgPool) {
    let fx = seed(pool).await;
    let server = MockServer::start().await;
    let id = connected(&fx, &server).await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({ "error": "invalid_grant" })),
        )
        .expect(1)
        .mount(&server)
        .await;
    let s = svc(&fx, Some(&server));
    assert_eq!(s.access_token(fx.h, id).await, Err(TokenError::NeedsReauth));
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    let a = OAuthAccountRepo::get(&mut tx, id)
        .await
        .unwrap()
        .unwrap()
        .value;
    assert_eq!(a.state, SourceState::NeedsReauth);
    drop(tx);
    // No second network call: the state short-circuits.
    assert_eq!(s.access_token(fx.h, id).await, Err(TokenError::NeedsReauth));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn not_configured(pool: PgPool) {
    let fx = seed(pool).await;
    let s = svc(&fx, None);
    assert!(!s.enabled());
    assert!(matches!(
        s.start(&adult(&fx), "s", None, OAuthPurpose::Calendar)
            .await,
        Err(AuthError::NotConfigured)
    ));
    assert!(matches!(
        s.callback(&adult(&fx), "s", "c", "x").await,
        Err(AuthError::NotConfigured)
    ));
    assert!(matches!(
        s.fresh_token(fx.h, OAuthAccountId::new()).await,
        Err(AuthError::NotConfigured)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn pair_system_token_resolves(pool: PgPool) {
    let fx = seed(pool).await;
    let devices = DeviceService::new(fx.store.clone(), "https://fa.example.com");
    let paired = devices.pair_system(fx.h, " Kitchen ").await.unwrap();
    assert_eq!(paired.device.name, "Kitchen");
    assert!(
        paired
            .pair_url
            .starts_with("https://fa.example.com/dashboard?device=")
    );
    let auth = AuthService::new(fx.store.clone(), AuthConfig::default());
    let actor = auth.resolve_actor(None, Some(&paired.token)).await.unwrap();
    assert!(matches!(actor, Actor::Device { device_id, .. } if device_id == paired.device.id));
    let mut tx = fx.store.tx_for(fx.h).await.unwrap();
    let log = AuditRepo::list(&mut tx, fx.h, 10).await.unwrap();
    assert!(
        log.iter()
            .any(|e| e.action == "device.pair" && e.actor_member.is_none())
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn account_service_system_paths(pool: PgPool) {
    let fx = seed(pool).await;
    let accounts = AccountService::new(fx.store.clone());
    accounts
        .create_account(&Actor::System, fx.h, fx.child, "sam", "abcd")
        .await
        .unwrap();
    assert!(
        accounts
            .create_account(&Actor::System, fx.h, fx.child, "sam2", "abcd")
            .await
            .is_err()
    );
    accounts
        .reset_password(&Actor::System, fx.h, "sam", "wxyz")
        .await
        .unwrap();
    let auth = AuthService::new(fx.store.clone(), AuthConfig::default());
    let session = auth.login("sam", "wxyz", None, None).await;
    assert!(session.is_ok(), "{session:?}");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn reconnect_with_purpose_unions_scopes(pool: PgPool) {
    let fx = seed(pool).await;
    let first = MockServer::start().await;
    mock_exchange(&first).await;
    let s1 = svc(&fx, Some(&first));
    let url = s1
        .start(&adult(&fx), "s", None, OAuthPurpose::Calendar)
        .await
        .unwrap();
    let (id1, _) = s1
        .callback(&adult(&fx), "s", "c", &state_of(&url))
        .await
        .unwrap();

    let second = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "access-2", "refresh_token": REFRESH, "expires_in": 3600,
            "scope": "openid email https://www.googleapis.com/auth/gmail.readonly",
            "id_token": id_token("alex@example.com"),
        })))
        .mount(&second)
        .await;
    let s2 = svc(&fx, Some(&second));
    let url = s2
        .start(&adult(&fx), "s", None, OAuthPurpose::GmailModify)
        .await
        .unwrap();
    let q: std::collections::HashMap<_, _> = url::Url::parse(&url)
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    assert!(q["scope"].contains("auth/gmail.modify"));
    assert_eq!(q["include_granted_scopes"], "true");
    let (id2, _) = s2
        .callback(&adult(&fx), "s", "c", &state_of(&url))
        .await
        .unwrap();
    assert_eq!(id1, id2);
    let accounts = s2.list_accounts(&adult(&fx)).await.unwrap();
    assert_eq!(accounts.len(), 1);
    for want in ["calendar.readonly", "gmail.readonly"] {
        assert!(
            accounts[0].scopes.iter().any(|x| x.ends_with(want)),
            "{want}"
        );
    }
}
