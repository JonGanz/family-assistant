use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use fa_domain::{Action, Actor, Device, DeviceId, DeviceKind, PolicyCtx, Resource, authorize};
use fa_store::{DeviceElevationRepo, DeviceRepo};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{AuthError, Result};
use crate::service::{AuthService, actor_ids, audit};

#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub device: Device,
    /// Shown once; only its SHA-256 is stored.
    pub token: String,
    pub pair_url: String,
}

pub(crate) fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl AuthService {
    pub async fn pair_device(&self, actor: &Actor, name: &str) -> Result<PairedDevice> {
        authorize(
            actor,
            Action::ManageDevices,
            &Resource::Household,
            &PolicyCtx::default(),
        )?;
        let ids = actor_ids(actor)?;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(bytes);
        let device = Device {
            id: DeviceId::new(),
            household_id: ids.household,
            kind: DeviceKind::Dashboard,
            name: name.trim().to_owned(),
            token_hash: token_hash(&token),
            last_seen: None,
            revoked_at: None,
            created_at: Utc::now(),
        };
        let mut tx = self.store.tx_for(ids.household).await?;
        let stored = DeviceRepo::insert(&mut tx, &device, ids.member)
            .await?
            .value;
        audit(
            &mut tx,
            &ids,
            "device.pair",
            "device",
            Some(stored.id.to_string()),
            Some(serde_json::json!({ "name": stored.name })),
        )
        .await?;
        tx.commit().await?;
        let base = self.cfg.public_base_url.trim_end_matches('/');
        Ok(PairedDevice {
            pair_url: format!("{base}/dashboard#device={token}"),
            device: stored,
            token,
        })
    }

    /// Resolves a bearer token to its (non-revoked) device; `last_seen` is touched at most once a minute.
    pub async fn authenticate_bearer(&self, token: &str) -> Result<Device> {
        let mut tx = self.store.tx_system().await?;
        let device = DeviceRepo::get_by_token_hash(&mut tx, &token_hash(token))
            .await?
            .ok_or(AuthError::Unauthenticated)?
            .value;
        let interval = chrono::Duration::from_std(self.cfg.device_touch_interval)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let stale = device
            .last_seen
            .is_none_or(|t| Utc::now().signed_duration_since(t) >= interval);
        if stale {
            DeviceRepo::touch(&mut tx, device.id).await?;
        }
        tx.commit().await?;
        Ok(device)
    }

    pub async fn revoke_device(&self, actor: &Actor, device: DeviceId) -> Result<()> {
        authorize(
            actor,
            Action::ManageDevices,
            &Resource::Device(device),
            &PolicyCtx::default(),
        )?;
        let ids = actor_ids(actor)?;
        let mut tx = self.store.tx_for(ids.household).await?;
        DeviceRepo::archive(&mut tx, device)
            .await
            .map_err(|e| match e {
                fa_store::StoreError::NotFound => AuthError::NotFound,
                e => e.into(),
            })?;
        DeviceElevationRepo::clear(&mut tx, device).await?;
        audit(
            &mut tx,
            &ids,
            "device.revoke",
            "device",
            Some(device.to_string()),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
