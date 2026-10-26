use chrono::Utc;
use fa_domain::Actor;
use fa_store::{DeviceElevationRepo, MemberRepo, SessionRecord, SessionRepo, UserAccountRepo};

use crate::error::{AuthError, Result};
use crate::service::AuthService;

impl AuthService {
    /// Bearer token first (a device, elevated when a PIN elevation is active), then the session
    /// cookie (an adult or child user). The bearer wins because the client sends it only on purpose
    /// (kiosk calls), while the cookie rides along on every request: a dashboard opened in a browser
    /// that is also logged in must still act as the device. Rolls the session expiry forward.
    /// `Unauthenticated` otherwise.
    pub async fn resolve_actor(&self, cookie: Option<&str>, bearer: Option<&str>) -> Result<Actor> {
        if let Some(token) = bearer {
            let device = self.authenticate_bearer(token).await?;
            let mut tx = self.store.tx_for(device.household_id).await?;
            let elevation = DeviceElevationRepo::active(&mut tx, device.id).await?;
            tx.rollback().await?;
            return Ok(match elevation {
                Some(e) => Actor::ElevatedDevice {
                    device_id: device.id,
                    household_id: device.household_id,
                    adult: e.member_id,
                },
                None => Actor::Device {
                    device_id: device.id,
                    household_id: device.household_id,
                },
            });
        }
        if let Some(id) = cookie
            && let Some(actor) = self.resolve_session(id).await?
        {
            return Ok(actor);
        }
        Err(AuthError::Unauthenticated)
    }

    async fn resolve_session(&self, id: &str) -> Result<Option<Actor>> {
        let mut sys = self.store.tx_system().await?;
        let rec: Option<SessionRecord> = SessionRepo::get(&mut sys, id).await?;
        sys.rollback().await?;
        let Some(rec) = rec else { return Ok(None) };

        let mut tx = self.store.tx_for(rec.household_id).await?;
        let member = MemberRepo::get(&mut tx, rec.member_id)
            .await?
            .map(|m| m.value)
            .filter(|m| m.archived_at.is_none());
        let account_ok = UserAccountRepo::get(&mut tx, rec.account_id)
            .await?
            .is_some_and(|a| a.value.disabled_at.is_none());
        let Some(member) = member.filter(|_| account_ok) else {
            tx.rollback().await?;
            return Ok(None);
        };

        let now = Utc::now();
        let ttl = chrono::Duration::from_std(self.cfg.session_ttl(member.role))
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        // Rolling expiry; skip the write when it would move by less than a minute.
        if now + ttl - rec.expires_at >= chrono::Duration::minutes(1) {
            SessionRepo::put(
                &mut tx,
                &SessionRecord {
                    last_seen: now,
                    expires_at: now + ttl,
                    ..rec
                },
            )
            .await?;
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        Ok(Some(Actor::User {
            member_id: member.id,
            role: member.role,
            household_id: member.household_id,
        }))
    }
}
