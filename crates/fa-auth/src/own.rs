//! Flows that act on the signed-in user's own account (password confirmation, own password
//! change) and the first-run owner creation (D-073).

use std::net::IpAddr;

use chrono::Utc;
use fa_domain::system_api::NewSetupOwner;
use fa_domain::{
    Actor, HouseholdId, Member, MemberId, Role, UserAccount, UserAccountId, validate_password,
    validate_pin,
};
use fa_store::{ChangeBus, MemberRepo, SessionRecord, SessionRepo, SetupRepo, UserAccountRepo};
use sqlx::PgConnection;
use tower_sessions::session::Id;

use crate::error::{AuthError, Result};
use crate::login::{LoginOutcome, Verified};
use crate::password;
use crate::service::{ActorIds, AuthService, audit};
use crate::session_store::identity_data;

/// Serialises the setup steps (`POST /setup/household` and `/setup/owner`) so two visitors cannot
/// both win. `pg_advisory_xact_lock` is released when the transaction ends.
const SETUP_LOCK_KEY: i64 = 0x4641_5345_5455_5001;

pub async fn lock_setup(conn: &mut PgConnection) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(SETUP_LOCK_KEY)
        .execute(conn)
        .await
        .map_err(fa_store::StoreError::from)?;
    Ok(())
}

fn user_ids(actor: &Actor) -> Result<(HouseholdId, MemberId)> {
    match actor {
        Actor::User {
            household_id,
            member_id,
            ..
        } => Ok((*household_id, *member_id)),
        _ => Err(AuthError::Forbidden(fa_domain::Denied)),
    }
}

fn limit(svc: &AuthService, ip: Option<IpAddr>) -> Result<()> {
    let key = ip.unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    if svc.ip_limiter.check(key) {
        Ok(())
    } else {
        Err(AuthError::RateLimited)
    }
}

impl AuthService {
    async fn own_account(&self, household: HouseholdId, member: MemberId) -> Result<UserAccount> {
        let mut tx = self.store.tx_for(household).await?;
        let account = UserAccountRepo::get_by_member(&mut tx, member).await?;
        tx.rollback().await?;
        account
            .map(|a| a.value)
            .filter(|a| a.disabled_at.is_none())
            .ok_or(AuthError::Unauthenticated)
    }

    /// Re-checks the signed-in user's password for a sensitive action. It uses the login limiter and
    /// lockout, so a wrong password counts toward both. Only a real user session qualifies.
    pub async fn confirm_password(
        &self,
        actor: &Actor,
        password: &str,
        ip: Option<IpAddr>,
    ) -> Result<()> {
        let (household, member) = user_ids(actor)?;
        limit(self, ip)?;
        let account = self.own_account(household, member).await?;
        let Verified { tx, .. } = self.verify_account(&account, password).await?;
        tx.commit().await?;
        Ok(())
    }

    /// C-03: changes the user's own password after checking the current one, and ends every other
    /// session of the user (`keep_session` is the cookie value of the calling session).
    pub async fn change_own_password(
        &self,
        actor: &Actor,
        current: &str,
        new: &str,
        keep_session: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<()> {
        let (household, member) = user_ids(actor)?;
        limit(self, ip)?;
        let account = self.own_account(household, member).await?;
        let mut tx = self.store.tx_for(household).await?;
        let role = MemberRepo::get(&mut tx, member)
            .await?
            .ok_or(AuthError::NotFound)?
            .value
            .role;
        tx.rollback().await?;
        validate_password(role, new)?;
        let new_hash = password::hash_blocking(new.to_owned()).await?;

        let Verified { mut tx, .. } = self.verify_account(&account, current).await?;
        let version = UserAccountRepo::get(&mut tx, account.id)
            .await?
            .ok_or(AuthError::NotFound)?
            .version;
        UserAccountRepo::update_password(&mut tx, account.id, &new_hash, version).await?;
        for s in SessionRepo::list_for_member(&mut tx, member).await? {
            if Some(s.id.as_str()) != keep_session {
                SessionRepo::delete(&mut tx, &s.id).await?;
            }
        }
        let ids = ActorIds {
            household,
            member: Some(member),
            device: None,
        };
        audit(
            &mut tx,
            &ids,
            "account.change_password",
            "user_account",
            Some(account.id.to_string()),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// First-run step 2 (D-073): creates the household's first adult, account, optional PIN and
    /// session in one transaction. `SetupClosed` when an adult account already exists.
    pub async fn setup_owner(
        &self,
        household: HouseholdId,
        new: &NewSetupOwner,
        user_agent: Option<&str>,
    ) -> Result<LoginOutcome> {
        let display_name = new.display_name.trim().to_owned();
        if display_name.is_empty() || display_name.chars().count() > 100 {
            return Err(AuthError::Invalid(
                "display_name must be 1 to 100 characters".into(),
            ));
        }
        let username = new.username.trim().to_owned();
        if username.is_empty() || username.chars().count() > 64 {
            return Err(AuthError::Invalid(
                "username must be 1 to 64 characters".into(),
            ));
        }
        if let Some(c) = &new.color
            && !(c.len() == 7
                && c.starts_with('#')
                && c[1..].chars().all(|x| x.is_ascii_hexdigit()))
        {
            return Err(AuthError::Invalid("color must look like #rrggbb".into()));
        }
        validate_password(Role::Adult, &new.password)?;
        if let Some(pin) = &new.pin {
            validate_pin(pin)?;
        }
        let password_hash = password::hash_blocking(new.password.clone()).await?;
        let pin_hash = match &new.pin {
            Some(p) => Some(password::hash_blocking(p.clone()).await?),
            None => None,
        };

        let mut tx = self.store.tx_for(household).await?;
        lock_setup(&mut tx).await?;
        if SetupRepo::state(&mut tx).await?.has_adult_account {
            return Err(AuthError::SetupClosed);
        }
        let now = Utc::now();
        let m = Member {
            id: MemberId::new(),
            household_id: household,
            display_name,
            role: Role::Adult,
            avatar: None,
            color: new.color.clone(),
            birthdate: None,
            grade: None,
            interests: vec![],
            hobbies: vec![],
            responsibilities: vec![],
            aliases: vec![],
            gender: None,
            relevance_tags: vec![],
            pin_hash,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        MemberRepo::insert(&mut tx, &m, None, None).await?;
        let account = UserAccount {
            id: UserAccountId::new(),
            household_id: household,
            member_id: m.id,
            username,
            password_hash,
            disabled_at: None,
            created_at: now,
        };
        UserAccountRepo::insert(&mut tx, &account).await?;
        let ttl = chrono::Duration::from_std(self.cfg.session_ttl(Role::Adult))
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let expires_at = now + ttl;
        let session_id = Id::default().to_string();
        let data = serde_json::to_vec(&identity_data(household, m.id, account.id))
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        SessionRepo::put(
            &mut tx,
            &SessionRecord {
                id: session_id.clone(),
                household_id: household,
                member_id: m.id,
                account_id: account.id,
                data,
                user_agent: user_agent.map(|s| s.chars().take(256).collect()),
                created_at: now,
                last_seen: now,
                expires_at,
            },
        )
        .await?;
        let ids = ActorIds {
            household,
            member: Some(m.id),
            device: None,
        };
        audit(
            &mut tx,
            &ids,
            "setup.owner",
            "member",
            Some(m.id.to_string()),
            None,
        )
        .await?;
        ChangeBus::publish(
            &mut tx,
            household,
            "member.updated",
            &serde_json::json!({ "id": m.id }),
        )
        .await?;
        tx.commit().await?;
        Ok(LoginOutcome {
            session_id,
            expires_at,
            household_id: household,
            member_id: m.id,
            role: Role::Adult,
        })
    }
}
