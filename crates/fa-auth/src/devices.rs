use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use fa_domain::{
    Action, Actor, Device, DeviceId, DeviceKind, HouseholdId, PolicyCtx, Resource, authorize,
};
use fa_store::{DeviceElevationRepo, DeviceRepo, Store};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::error::{AuthError, Result};
use crate::service::{ActorIds, AuthService, actor_ids, audit};

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

async fn pair(
    store: &Store,
    public_base_url: &str,
    ids: ActorIds,
    name: &str,
) -> Result<PairedDevice> {
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
    let mut tx = store.tx_for(ids.household).await?;
    let stored = DeviceRepo::insert(&mut tx, &device, ids.member)
        .await?
        .value;
    let after = if ids.member.is_none() && ids.device.is_none() {
        serde_json::json!({ "name": stored.name, "actor": "system" })
    } else {
        serde_json::json!({ "name": stored.name })
    };
    audit(
        &mut tx,
        &ids,
        "device.pair",
        "device",
        Some(stored.id.to_string()),
        Some(after),
    )
    .await?;
    tx.commit().await?;
    let base = public_base_url.trim_end_matches('/');
    Ok(PairedDevice {
        pair_url: format!("{base}/dashboard?device={token}"),
        device: stored,
        token,
    })
}

/// Device pairing for callers without a user session (the CLI, imports).
#[derive(Clone)]
pub struct DeviceService {
    store: Store,
    public_base_url: String,
}

impl DeviceService {
    pub fn new(store: Store, public_base_url: impl Into<String>) -> Self {
        Self {
            store,
            public_base_url: public_base_url.into(),
        }
    }

    /// Pairs a dashboard device as `Actor::System`; the audit row records the actor as `system`.
    pub async fn pair_system(&self, household: HouseholdId, name: &str) -> Result<PairedDevice> {
        authorize(
            &Actor::System,
            Action::ManageDevices,
            &Resource::Household,
            &PolicyCtx::default(),
        )?;
        let ids = ActorIds {
            household,
            member: None,
            device: None,
        };
        pair(&self.store, &self.public_base_url, ids, name).await
    }
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
        pair(&self.store, &self.cfg.public_base_url, ids, name).await
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
