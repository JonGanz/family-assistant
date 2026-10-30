use chrono::Utc;
use fa_domain::{
    Action, Actor, HouseholdId, MemberId, PolicyCtx, Resource, UserAccount, UserAccountId,
    authorize, validate_password,
};
use fa_store::{MemberRepo, SessionRepo, Store, UserAccountRepo};

use crate::error::{AuthError, Result};
use crate::password::hash_blocking;
use crate::service::{ActorIds, actor_ids, audit};

/// Account creation and password reset. Accepts `Actor::System` (the CLI), and users where
/// `Policy::authorize` allows it (an adult managing a child's account).
#[derive(Clone)]
pub struct AccountService {
    store: Store,
}

impl AccountService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    fn scope(actor: &Actor, household: HouseholdId) -> Result<ActorIds> {
        if matches!(actor, Actor::System) {
            return Ok(ActorIds {
                household,
                member: None,
                device: None,
            });
        }
        let ids = actor_ids(actor)?;
        if ids.household != household {
            return Err(AuthError::Forbidden(fa_domain::Denied));
        }
        Ok(ids)
    }

    pub async fn create_account(
        &self,
        actor: &Actor,
        household: HouseholdId,
        member: MemberId,
        username: &str,
        password: &str,
    ) -> Result<UserAccount> {
        let ids = Self::scope(actor, household)?;
        let username = username.trim().to_owned();
        if username.is_empty() {
            return Err(AuthError::Internal("username must not be empty".into()));
        }
        let mut tx = self.store.tx_for(household).await?;
        let m = MemberRepo::get(&mut tx, member)
            .await?
            .ok_or(AuthError::NotFound)?
            .value;
        authorize(
            actor,
            Action::ManageChildAccount,
            &Resource::Member(m.id, m.role),
            &PolicyCtx::default(),
        )?;
        if m.archived_at.is_some() {
            return Err(AuthError::NotFound);
        }
        if UserAccountRepo::get_by_member(&mut tx, m.id)
            .await?
            .is_some()
        {
            return Err(AuthError::Internal("member already has an account".into()));
        }
        validate_password(m.role, password)?;
        let account = UserAccount {
            id: UserAccountId::new(),
            household_id: household,
            member_id: m.id,
            username,
            password_hash: hash_blocking(password.to_owned()).await?,
            disabled_at: None,
            initial_password: false,
            created_at: Utc::now(),
        };
        UserAccountRepo::insert(&mut tx, &account).await?;
        audit(
            &mut tx,
            &ids,
            "user.add",
            "user_account",
            Some(account.id.to_string()),
            system_marker(actor),
        )
        .await?;
        tx.commit().await?;
        Ok(account)
    }

    /// Sets a new password, clears any lockout and ends the member's sessions.
    pub async fn reset_password(
        &self,
        actor: &Actor,
        household: HouseholdId,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let ids = Self::scope(actor, household)?;
        let mut tx = self.store.tx_for(household).await?;
        let found = UserAccountRepo::find_by_username(&mut tx, Some(household), username)
            .await?
            .ok_or(AuthError::NotFound)?;
        let m = MemberRepo::get(&mut tx, found.value.member_id)
            .await?
            .ok_or(AuthError::NotFound)?
            .value;
        authorize(
            actor,
            Action::ManageChildAccount,
            &Resource::Member(m.id, m.role),
            &PolicyCtx::default(),
        )?;
        validate_password(m.role, password)?;
        let hash = hash_blocking(password.to_owned()).await?;
        UserAccountRepo::update_password(&mut tx, found.value.id, &hash, found.version).await?;
        SessionRepo::delete_for_member(&mut tx, m.id).await?;
        audit(
            &mut tx,
            &ids,
            "user.reset_password",
            "user_account",
            Some(found.value.id.to_string()),
            system_marker(actor),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn system_marker(actor: &Actor) -> Option<serde_json::Value> {
    matches!(actor, Actor::System).then(|| serde_json::json!({ "actor": "system" }))
}
