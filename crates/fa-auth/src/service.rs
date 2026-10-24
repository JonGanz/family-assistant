use std::fmt;
use std::sync::{Arc, OnceLock};

use chrono::Utc;
use fa_domain::{Actor, AuditEntry, AuditEntryId, DeviceId, HouseholdId, MemberId};
use fa_store::Tx;
use fa_store::{AuditRepo, Store};

use crate::config::AuthConfig;
use crate::error::{AuthError, Result};
use crate::limiter::IpRateLimiter;
use crate::password;

/// Hooks for security events. TODO: wire a notification (ntfy) implementation after M0.
pub trait AuthEvents: Send + Sync {
    fn pin_locked(&self, _household: HouseholdId, _member: MemberId) {}
    fn account_locked(&self, _household: HouseholdId, _member: MemberId) {}
}

struct NoEvents;
impl AuthEvents for NoEvents {}

#[derive(Clone)]
pub struct AuthService {
    pub(crate) store: Store,
    pub(crate) cfg: Arc<AuthConfig>,
    pub(crate) ip_limiter: Arc<IpRateLimiter>,
    pub(crate) events: Arc<dyn AuthEvents>,
}

impl fmt::Debug for AuthService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService").finish_non_exhaustive()
    }
}

impl AuthService {
    pub fn new(store: Store, cfg: AuthConfig) -> Self {
        let ip_limiter = Arc::new(IpRateLimiter::new(cfg.ip_burst, cfg.ip_refill));
        Self {
            store,
            cfg: Arc::new(cfg),
            ip_limiter,
            events: Arc::new(NoEvents),
        }
    }

    pub fn with_events(mut self, events: Arc<dyn AuthEvents>) -> Self {
        self.events = events;
        self
    }

    pub fn config(&self) -> &AuthConfig {
        &self.cfg
    }

    /// Hash verified when the username is unknown, so timing does not reveal existence.
    pub(crate) async fn dummy_verify(&self, password: &str) {
        static DUMMY: OnceLock<String> = OnceLock::new();
        let hash = match DUMMY.get() {
            Some(h) => h.clone(),
            None => match password::hash_blocking("dummy-password-for-timing".into()).await {
                Ok(h) => DUMMY.get_or_init(|| h).clone(),
                Err(_) => return,
            },
        };
        let _ = password::verify_blocking(password.to_owned(), hash).await;
    }
}

/// Household and audit identity of an acting principal.
pub(crate) struct ActorIds {
    pub household: HouseholdId,
    pub member: Option<MemberId>,
    pub device: Option<DeviceId>,
}

pub(crate) fn actor_ids(actor: &Actor) -> Result<ActorIds> {
    match actor {
        Actor::User {
            member_id,
            household_id,
            ..
        } => Ok(ActorIds {
            household: *household_id,
            member: Some(*member_id),
            device: None,
        }),
        Actor::ElevatedDevice {
            device_id,
            household_id,
            adult,
        } => Ok(ActorIds {
            household: *household_id,
            member: Some(*adult),
            device: Some(*device_id),
        }),
        Actor::Device {
            device_id,
            household_id,
        } => Ok(ActorIds {
            household: *household_id,
            member: None,
            device: Some(*device_id),
        }),
        Actor::System => Err(AuthError::Forbidden(fa_domain::Denied)),
    }
}

pub(crate) async fn audit(
    conn: &mut Tx,
    who: &ActorIds,
    action: &str,
    subject_type: &str,
    subject_id: Option<String>,
    after: Option<serde_json::Value>,
) -> Result<()> {
    AuditRepo::insert(
        conn,
        &AuditEntry {
            id: AuditEntryId::new(),
            household_id: who.household,
            actor_member: who.member,
            device: who.device,
            action: action.into(),
            subject_type: subject_type.into(),
            subject_id,
            before: None,
            after,
            at: Utc::now(),
        },
    )
    .await?;
    Ok(())
}
