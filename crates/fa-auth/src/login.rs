use std::net::IpAddr;

use chrono::{DateTime, Utc};
use fa_domain::{Action, PolicyCtx};
use fa_domain::{Actor, HouseholdId, MemberId, Resource, Role, authorize};
use fa_store::{MemberRepo, SessionRecord, SessionRepo, UserAccountRepo};
use sha2::{Digest, Sha256};
use tower_sessions::session::Id;

use crate::error::{AuthError, Result};
use crate::password::{self, PasswordCheck};
use crate::service::{ActorIds, AuthService, actor_ids, audit};

use crate::session_store::identity_data;

#[derive(Debug, Clone)]
pub struct LoginOutcome {
    /// The cookie value (a `tower-sessions` id).
    pub session_id: String,
    pub expires_at: DateTime<Utc>,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub role: Role,
}

/// A session as shown to its owner or an adult. `handle` is a one-way id, never the cookie secret.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub handle: String,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub(crate) fn session_handle(id: &str) -> String {
    let digest = Sha256::digest(id.as_bytes());
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

fn secs(d: std::time::Duration) -> Result<chrono::Duration> {
    chrono::Duration::from_std(d).map_err(|e| AuthError::Internal(e.to_string()))
}

/// A password that passed the lockout and hash checks; `tx` holds the account row lock.
pub(crate) struct Verified {
    pub tx: fa_store::Tx,
    pub needs_rehash: bool,
}

impl AuthService {
    /// Checks `password` against the account with the lockout rules of D-019.
    ///
    /// The argon2 verify runs with no transaction open (C-12). The lockout row is then read
    /// `FOR UPDATE` and the outcome is recorded in that same short transaction, so parallel guesses
    /// are serialised and a lock set by an earlier one is honoured. On success the transaction
    /// stays open for the caller, with any failure counter already cleared.
    pub(crate) async fn verify_account(
        &self,
        account: &fa_domain::UserAccount,
        password: &str,
    ) -> Result<Verified> {
        let ids = ActorIds {
            household: account.household_id,
            member: Some(account.member_id),
            device: None,
        };
        let mut tx = self.store.tx_for(account.household_id).await?;
        let lock = UserAccountRepo::lockout(&mut tx, account.id).await?;
        tx.rollback().await?;
        if let Some(until) = lock.locked_until.filter(|u| *u > Utc::now()) {
            return Err(AuthError::Locked { until });
        }

        let check =
            password::verify_blocking(password.to_owned(), account.password_hash.clone()).await?;

        let mut tx = self.store.tx_for(account.household_id).await?;
        let lock = UserAccountRepo::lockout(&mut tx, account.id).await?;
        if let Some(until) = lock.locked_until.filter(|u| *u > Utc::now()) {
            tx.rollback().await?;
            return Err(AuthError::Locked { until });
        }
        let PasswordCheck::Match { needs_rehash } = check else {
            let after = UserAccountRepo::record_failure(
                &mut tx,
                account.id,
                self.cfg.max_login_failures,
                self.cfg.login_lock.as_secs() as i64,
            )
            .await?;
            let locked = after.locked_until.is_some_and(|u| u > Utc::now());
            if locked {
                audit(
                    &mut tx,
                    &ids,
                    "account.locked",
                    "user_account",
                    Some(account.id.to_string()),
                    None,
                )
                .await?;
            }
            tx.commit().await?;
            if locked {
                self.events
                    .account_locked(account.household_id, account.member_id);
            }
            return Err(AuthError::InvalidCredentials);
        };
        if lock.failed_attempts > 0 || lock.locked_until.is_some() {
            UserAccountRepo::clear_failures(&mut tx, account.id).await?;
        }
        Ok(Verified { tx, needs_rehash })
    }

    /// Verifies credentials and creates a session. `ip` feeds the per-IP limiter.
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip: Option<IpAddr>,
        user_agent: Option<&str>,
    ) -> Result<LoginOutcome> {
        // Unknown peer address shares one global bucket instead of skipping limiting.
        let key = ip.unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
        if !self.ip_limiter.check(key) {
            return Err(AuthError::RateLimited);
        }

        // Pre-household lookup: system transaction.
        let mut sys = self.store.tx_system().await?;
        let found = UserAccountRepo::find_by_username(&mut sys, None, username).await?;
        sys.rollback().await?;
        let Some(account) = found.filter(|a| a.value.disabled_at.is_none()) else {
            self.dummy_verify(password).await;
            return Err(AuthError::InvalidCredentials);
        };
        let account = account.value;

        let Verified {
            mut tx,
            needs_rehash,
        } = self.verify_account(&account, password).await?;

        let member = MemberRepo::get(&mut tx, account.member_id)
            .await?
            .map(|m| m.value)
            .filter(|m| m.archived_at.is_none())
            .ok_or(AuthError::InvalidCredentials)?;

        if needs_rehash {
            match password::hash_blocking(password.to_owned()).await {
                Ok(new_hash) => {
                    let version = UserAccountRepo::get(&mut tx, account.id)
                        .await?
                        .map(|a| a.version);
                    if let Some(v) = version
                        && let Err(e) =
                            UserAccountRepo::update_password(&mut tx, account.id, &new_hash, v)
                                .await
                    {
                        tracing::warn!(error = %e, "password rehash skipped");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "password rehash failed"),
            }
        }

        let now = Utc::now();
        let expires_at = now + secs(self.cfg.session_ttl(member.role))?;
        let session_id = Id::default().to_string();
        let data = serde_json::to_vec(&identity_data(
            account.household_id,
            account.member_id,
            account.id,
        ))
        .map_err(|e| AuthError::Internal(e.to_string()))?;
        SessionRepo::put(
            &mut tx,
            &SessionRecord {
                id: session_id.clone(),
                household_id: account.household_id,
                member_id: account.member_id,
                account_id: account.id,
                data,
                user_agent: user_agent.map(|s| s.chars().take(256).collect()),
                created_at: now,
                last_seen: now,
                expires_at,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(LoginOutcome {
            session_id,
            expires_at,
            household_id: account.household_id,
            member_id: account.member_id,
            role: member.role,
        })
    }

    /// Deletes the session behind a cookie value. Idempotent.
    pub async fn logout(&self, session_id: &str) -> Result<()> {
        let mut tx = self.store.tx_system().await?;
        SessionRepo::delete(&mut tx, session_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// A member's live sessions: their own, or a child's for an adult.
    pub async fn list_sessions(&self, actor: &Actor, owner: MemberId) -> Result<Vec<SessionInfo>> {
        let ids = actor_ids(actor)?;
        let mut tx = self.store.tx_for(ids.household).await?;
        self.authorize_session_access(&mut tx, actor, owner, Action::ReadMemberFull)
            .await?;
        let rows = SessionRepo::list_for_member(&mut tx, owner).await?;
        tx.rollback().await?;
        Ok(rows
            .into_iter()
            .map(|s| SessionInfo {
                handle: session_handle(&s.id),
                user_agent: s.user_agent,
                created_at: s.created_at,
                last_seen: s.last_seen,
                expires_at: s.expires_at,
            })
            .collect())
    }

    /// Revokes one session by handle: own sessions, or a child's session for an adult.
    pub async fn revoke_session(&self, actor: &Actor, owner: MemberId, handle: &str) -> Result<()> {
        let ids = actor_ids(actor)?;
        let mut tx = self.store.tx_for(ids.household).await?;
        self.authorize_session_access(&mut tx, actor, owner, Action::RevokeChildSession)
            .await?;
        let sessions = SessionRepo::list_for_member(&mut tx, owner).await?;
        let target = sessions
            .iter()
            .find(|s| session_handle(&s.id) == handle)
            .ok_or(AuthError::NotFound)?;
        SessionRepo::delete(&mut tx, &target.id).await?;
        audit(
            &mut tx,
            &ids,
            "session.revoke",
            "session",
            Some(handle.to_owned()),
            Some(serde_json::json!({ "owner": owner.to_string() })),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Owners may always act on their own sessions; otherwise `action` is checked by the policy.
    async fn authorize_session_access(
        &self,
        tx: &mut fa_store::Tx,
        actor: &Actor,
        owner: MemberId,
        action: Action,
    ) -> Result<()> {
        if matches!(actor, Actor::User { member_id, .. } if *member_id == owner) {
            return Ok(());
        }
        let member = MemberRepo::get(tx, owner)
            .await?
            .ok_or(AuthError::NotFound)?
            .value;
        let resource = match action {
            Action::RevokeChildSession => Resource::Session {
                owner,
                owner_role: member.role,
            },
            _ => Resource::Member(owner, member.role),
        };
        authorize(actor, action, &resource, &PolicyCtx::default())?;
        Ok(())
    }
}
