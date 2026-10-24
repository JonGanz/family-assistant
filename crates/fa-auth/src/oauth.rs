//! Google OAuth (authorisation code + PKCE S256) and the access-token provider fa-calendar uses.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use fa_connectors::google::oauth::GoogleOAuth;
use fa_connectors::{ConnectorError, Http};
use fa_domain::{
    AccessTokenProvider, Action, Actor, HouseholdId, MemberId, OAuthAccountId, PolicyCtx, Resource,
    Role, SecretKind, SourceState, TokenError, authorize,
};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{MasterKey, MemberRepo, OAuthAccountRepo, OAuthPendingRepo, SecretRepo, Store};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{AuthError, Result};
use crate::service::{actor_ids, audit};

const PROVIDER: &str = "google";
const SECRET_OWNER: &str = "oauth_account";
const SCOPES: [&str; 3] = [
    "openid",
    "email",
    "https://www.googleapis.com/auth/calendar.readonly",
];
const PENDING_TTL_MINUTES: i64 = 10;
const EXPIRY_MARGIN_SECONDS: i64 = 60;

type TokenCache = HashMap<OAuthAccountId, (String, DateTime<Utc>)>;

/// An OAuth account as shown to clients: never carries secret material.
#[derive(Debug, Clone)]
pub struct OAuthAccountView {
    pub id: OAuthAccountId,
    pub provider: String,
    pub owner_member: MemberId,
    pub email: String,
    pub scopes: Vec<String>,
    pub state: SourceState,
    pub last_error: Option<String>,
}

#[derive(Clone)]
pub struct OAuthService {
    store: Store,
    key: Arc<MasterKey>,
    google: Option<GoogleOAuth>,
    public_url: String,
    http: Http,
    cache: Arc<Mutex<TokenCache>>,
}

impl std::fmt::Debug for OAuthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthService").finish_non_exhaustive()
    }
}

impl OAuthService {
    /// `google: None` disables every call (`AuthError::NotConfigured`).
    pub fn new(
        store: Store,
        key: MasterKey,
        google: Option<GoogleOAuth>,
        public_url: impl Into<String>,
        http: Http,
    ) -> Self {
        Self {
            store,
            key: Arc::new(key),
            google,
            public_url: public_url.into(),
            http,
            cache: Arc::default(),
        }
    }

    /// Reads `FA_GOOGLE_CLIENT_ID` / `FA_GOOGLE_CLIENT_SECRET`; unset or empty disables Google.
    pub fn from_env(
        store: Store,
        key: MasterKey,
        public_url: impl Into<String>,
        http: Http,
    ) -> Self {
        let get = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let google = get("FA_GOOGLE_CLIENT_ID")
            .zip(get("FA_GOOGLE_CLIENT_SECRET"))
            .map(|(id, secret)| GoogleOAuth::new(id, secret));
        Self::new(store, key, google, public_url, http)
    }

    pub fn enabled(&self) -> bool {
        self.google.is_some()
    }

    fn google(&self) -> Result<&GoogleOAuth> {
        self.google.as_ref().ok_or(AuthError::NotConfigured)
    }

    fn redirect_uri(&self) -> String {
        format!(
            "{}/api/v1/oauth/google/callback",
            self.public_url.trim_end_matches('/')
        )
    }

    fn authorize_own(actor: &Actor, member: MemberId) -> Result<()> {
        authorize(
            actor,
            Action::ManageCalendarSource,
            &Resource::Source {
                owner: Some(member),
                owner_role: Some(Role::Adult),
            },
            &PolicyCtx::default(),
        )?;
        Ok(())
    }

    /// Starts the flow for an adult; returns the Google consent URL.
    pub async fn start(
        &self,
        actor: &Actor,
        session_id: &str,
        return_to: Option<&str>,
    ) -> Result<String> {
        let google = self.google()?;
        let ids = actor_ids(actor)?;
        let member = ids.member.ok_or(AuthError::Forbidden(fa_domain::Denied))?;
        Self::authorize_own(actor, member)?;
        if !matches!(
            actor,
            Actor::User {
                role: Role::Adult,
                ..
            }
        ) {
            return Err(AuthError::Forbidden(fa_domain::Denied));
        }

        let state = random_b64(32);
        let verifier = random_b64(32);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        // Only same-site paths may be echoed back after the flow.
        let return_to = return_to.filter(|r| r.starts_with('/') && !r.starts_with("//"));

        let mut tx = self.store.tx_for(ids.household).await?;
        OAuthPendingRepo::insert(
            &mut tx,
            &state,
            session_id,
            member,
            &verifier,
            return_to,
            Utc::now() + Duration::minutes(PENDING_TTL_MINUTES),
        )
        .await?;
        tx.commit().await?;
        google
            .authorize_url(&self.redirect_uri(), &SCOPES, &state, &challenge)
            .map_err(|e| AuthError::OAuth(e.to_string()))
    }

    /// Completes the flow. Returns the account and the `return_to` stored at `start`.
    pub async fn callback(
        &self,
        actor: &Actor,
        session_id: &str,
        code: &str,
        state: &str,
    ) -> Result<(OAuthAccountId, Option<String>)> {
        let google = self.google()?;
        let ids = actor_ids(actor)?;
        let member = ids.member.ok_or(AuthError::Forbidden(fa_domain::Denied))?;
        Self::authorize_own(actor, member)?;

        // Single use: the row is consumed (and the delete committed) before any check.
        let mut tx = self.store.tx_for(ids.household).await?;
        let pending = OAuthPendingRepo::take(&mut tx, state).await?;
        tx.commit().await?;
        let pending = pending.ok_or(AuthError::InvalidState)?;
        if pending.expires_at <= Utc::now()
            || pending.session_id != session_id
            || pending.member != member
        {
            return Err(AuthError::InvalidState);
        }

        let tokens = google
            .exchange_code(
                &self.http,
                code,
                &self.redirect_uri(),
                &pending.pkce_verifier,
            )
            .await
            .map_err(|e| AuthError::OAuth(format!("token exchange failed: {e}")))?;
        let refresh = tokens.refresh.clone().ok_or_else(|| {
            AuthError::OAuth(
                "Google returned no refresh token; revoke this app's access in your Google \
                 account and connect again"
                    .into(),
            )
        })?;
        let email = tokens
            .id_token_email
            .clone()
            .ok_or_else(|| AuthError::OAuth("Google did not return an email address".into()))?;

        let mut tx = self.store.tx_for(ids.household).await?;
        let account = OAuthAccountRepo::upsert(
            &mut tx,
            ids.household,
            PROVIDER,
            member,
            &email,
            &tokens.scopes,
            None,
        )
        .await?
        .value;
        let secret = SecretRepo::put(
            &mut tx,
            &self.key,
            ids.household,
            SECRET_OWNER,
            account.id.as_uuid(),
            SecretKind::OauthToken,
            refresh.as_bytes(),
            Some(member),
        )
        .await?;
        OAuthAccountRepo::set_secret(&mut tx, account.id, secret).await?;
        audit(
            &mut tx,
            &ids,
            "oauth.connect",
            "oauth_account",
            Some(account.id.to_string()),
            Some(serde_json::json!({ "provider": PROVIDER, "email": email })),
        )
        .await?;
        tx.commit().await?;
        if let Ok(mut c) = self.cache.lock() {
            c.insert(account.id, (tokens.access, tokens.expires_at));
        }
        Ok((account.id, pending.return_to))
    }

    /// A valid access token, cached in memory until 60 s before expiry.
    pub async fn fresh_token(
        &self,
        household: HouseholdId,
        account: OAuthAccountId,
    ) -> Result<String> {
        let google = self.google()?;
        if let Ok(c) = self.cache.lock()
            && let Some((tok, exp)) = c.get(&account)
            && *exp - Duration::seconds(EXPIRY_MARGIN_SECONDS) > Utc::now()
        {
            return Ok(tok.clone());
        }

        let mut tx = self.store.tx_for(household).await?;
        let acct = OAuthAccountRepo::get(&mut tx, account)
            .await?
            .ok_or(AuthError::NotFound)?
            .value;
        if acct.state == SourceState::NeedsReauth {
            tx.rollback().await?;
            return Err(AuthError::NeedsReauth);
        }
        let stored = SecretRepo::get(
            &mut tx,
            &self.key,
            household,
            SECRET_OWNER,
            account.as_uuid(),
            SecretKind::OauthToken,
        )
        .await?;
        tx.rollback().await?;
        let refresh = String::from_utf8(stored.ok_or(AuthError::NeedsReauth)?)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        match google.refresh(&self.http, &refresh).await {
            Ok(tokens) => {
                if let Some(new) = tokens.refresh.as_deref().filter(|n| *n != refresh) {
                    let mut tx = self.store.tx_for(household).await?;
                    SecretRepo::put(
                        &mut tx,
                        &self.key,
                        household,
                        SECRET_OWNER,
                        account.as_uuid(),
                        SecretKind::OauthToken,
                        new.as_bytes(),
                        None,
                    )
                    .await?;
                    tx.commit().await?;
                }
                if let Ok(mut c) = self.cache.lock() {
                    c.insert(account, (tokens.access.clone(), tokens.expires_at));
                }
                Ok(tokens.access)
            }
            Err(ConnectorError::Unauthorized) => {
                if let Ok(mut c) = self.cache.lock() {
                    c.remove(&account);
                }
                let mut tx = self.store.tx_for(household).await?;
                OAuthAccountRepo::set_state(
                    &mut tx,
                    account,
                    SourceState::NeedsReauth,
                    Some("Google rejected the stored refresh token (invalid_grant)"),
                )
                .await?;
                tx.commit().await?;
                Err(AuthError::NeedsReauth)
            }
            Err(e) => Err(AuthError::OAuth(e.to_string())),
        }
    }

    /// The caller's own accounts and children's, per the D-006 owner rules.
    pub async fn list_accounts(&self, actor: &Actor) -> Result<Vec<OAuthAccountView>> {
        let ids = actor_ids(actor)?;
        let mut tx = self.store.tx_for(ids.household).await?;
        let accounts = OAuthAccountRepo::list(&mut tx, None).await?;
        let mut roles: HashMap<MemberId, Role> = HashMap::new();
        let mut out = Vec::new();
        for a in accounts.into_iter().map(|v| v.value) {
            let role = match roles.get(&a.owner_member) {
                Some(r) => *r,
                None => {
                    let Some(m) = MemberRepo::get(&mut tx, a.owner_member).await? else {
                        continue;
                    };
                    roles.insert(a.owner_member, m.value.role);
                    m.value.role
                }
            };
            let res = Resource::Source {
                owner: Some(a.owner_member),
                owner_role: Some(role),
            };
            match authorize(
                actor,
                Action::ManageCalendarSource,
                &res,
                &PolicyCtx::default(),
            ) {
                Ok(()) => out.push(OAuthAccountView {
                    id: a.id,
                    provider: a.provider,
                    owner_member: a.owner_member,
                    email: a.email,
                    scopes: a.scopes,
                    state: a.state,
                    last_error: a.last_error,
                }),
                Err(_) => continue,
            }
        }
        tx.rollback().await?;
        Ok(out)
    }

    pub async fn cleanup_expired(&self) -> Result<u64> {
        cleanup_expired(&self.store).await
    }
}

async fn cleanup_expired(store: &Store) -> Result<u64> {
    let mut tx = store.tx_system().await?;
    let n = OAuthPendingRepo::delete_expired(&mut tx).await?;
    tx.commit().await?;
    Ok(n)
}

#[async_trait]
impl AccessTokenProvider for OAuthService {
    async fn access_token(
        &self,
        household: HouseholdId,
        account: OAuthAccountId,
    ) -> std::result::Result<String, TokenError> {
        self.fresh_token(household, account)
            .await
            .map_err(|e| match e {
                AuthError::NeedsReauth => TokenError::NeedsReauth,
                e => TokenError::Other(e.to_string()),
            })
    }
}

fn random_b64(n: usize) -> String {
    let mut b = vec![0u8; n];
    rand::rng().fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

/// `oauth.cleanup`: deletes expired `oauth_pending` rows.
struct OAuthCleanup;

impl JobHandler for OAuthCleanup {
    fn kind(&self) -> &'static str {
        "oauth.cleanup"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, std::result::Result<(), JobError>> {
        Box::pin(async move {
            let n = cleanup_expired(&ctx.store).await.map_err(|e| match e {
                AuthError::Store(s) => JobError::Store(s),
                e => JobError::Handler(e.to_string()),
            })?;
            tracing::info!(removed = n, "expired oauth_pending rows removed");
            Ok(())
        })
    }
}

pub fn handlers() -> Vec<Arc<dyn JobHandler>> {
    vec![Arc::new(OAuthCleanup)]
}

pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![("oauth.cleanup", "0 * * * *")]
}
