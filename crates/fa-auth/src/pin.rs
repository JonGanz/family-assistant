use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;
use fa_domain::{
    Action, Actor, Device, DeviceElevation, MemberId, PolicyCtx, Resource, Role, authorize,
    validate_pin,
};
use fa_store::{DeviceElevationRepo, MemberRepo};

use crate::error::{AuthError, Result};
use crate::limiter::KeyedRateLimiter;
use crate::password::{self, PasswordCheck};
use crate::service::{ActorIds, AuthService, actor_ids, audit};

/// PIN attempts per device: 5 per 15 minutes.
fn device_limiter() -> &'static KeyedRateLimiter<fa_domain::DeviceId> {
    static L: OnceLock<KeyedRateLimiter<fa_domain::DeviceId>> = OnceLock::new();
    L.get_or_init(|| KeyedRateLimiter::new(5, Duration::from_secs(180)))
}

impl AuthService {
    /// An adult sets their own PIN (4-6 digits, argon2id).
    pub async fn set_pin(&self, actor: &Actor, member: MemberId, pin: &str) -> Result<()> {
        validate_pin(pin)?;
        let ids = actor_ids(actor)?;
        authorize(
            actor,
            Action::SetOwnPin,
            &Resource::Member(member, Role::Adult),
            &PolicyCtx::default(),
        )?;
        let hash = password::hash_blocking(pin.to_owned()).await?;
        let mut tx = self.store.tx_for(ids.household).await?;
        let m = MemberRepo::get(&mut tx, member)
            .await?
            .ok_or(AuthError::NotFound)?;
        if m.value.role != Role::Adult {
            return Err(AuthError::Forbidden(fa_domain::Denied));
        }
        MemberRepo::set_pin_hash(&mut tx, member, Some(&hash)).await?;
        audit(
            &mut tx,
            &ids,
            "pin.set",
            "member",
            Some(member.to_string()),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Dashboard device elevates to an adult for `elevation_ttl` after a correct PIN.
    pub async fn elevate(
        &self,
        device: &Device,
        member: MemberId,
        pin: &str,
    ) -> Result<DeviceElevation> {
        if !device_limiter().check(device.id) {
            return Err(AuthError::RateLimited);
        }
        let dev_actor = Actor::Device {
            device_id: device.id,
            household_id: device.household_id,
        };
        let ids = ActorIds {
            household: device.household_id,
            member: None,
            device: Some(device.id),
        };
        let mut tx = self.store.tx_for(device.household_id).await?;
        let m = MemberRepo::get(&mut tx, member)
            .await?
            .map(|m| m.value)
            .filter(|m| m.archived_at.is_none())
            .ok_or(AuthError::InvalidCredentials)?;
        authorize(
            &dev_actor,
            Action::Elevate,
            &Resource::Member(member, m.role),
            &PolicyCtx::default(),
        )?;
        let Some(pin_hash) = m.pin_hash else {
            return Err(AuthError::InvalidCredentials);
        };
        let lock = MemberRepo::pin_lockout(&mut tx, member).await?;
        if let Some(until) = lock.locked_until.filter(|u| *u > Utc::now()) {
            return Err(AuthError::Locked { until });
        }

        let check = password::verify_blocking(pin.to_owned(), pin_hash).await?;
        if !matches!(check, PasswordCheck::Match { .. }) {
            let after = MemberRepo::record_pin_failure(
                &mut tx,
                member,
                self.cfg.max_pin_failures,
                self.cfg.pin_lock.as_secs() as i64,
            )
            .await?;
            let locked = after.locked_until.is_some_and(|u| u > Utc::now());
            if locked {
                audit(
                    &mut tx,
                    &ids,
                    "pin.locked",
                    "member",
                    Some(member.to_string()),
                    None,
                )
                .await?;
            }
            tx.commit().await?;
            if locked {
                self.events.pin_locked(device.household_id, member);
            }
            return Err(AuthError::InvalidCredentials);
        }

        if lock.failed_attempts > 0 || lock.locked_until.is_some() {
            MemberRepo::clear_pin_failures(&mut tx, member).await?;
        }
        let ttl = chrono::Duration::from_std(self.cfg.elevation_ttl)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let elevation = DeviceElevation {
            device_id: device.id,
            member_id: member,
            expires_at: Utc::now() + ttl,
        };
        DeviceElevationRepo::create(&mut tx, &elevation, device.household_id).await?;
        audit(
            &mut tx,
            &ids,
            "device.elevate",
            "member",
            Some(member.to_string()),
            Some(serde_json::json!({ "expires_at": elevation.expires_at })),
        )
        .await?;
        tx.commit().await?;
        Ok(elevation)
    }
}
