//! Repositories: stateless structs whose functions take `&mut PgConnection` (a [`crate::Tx`] derefs to it).

use fa_domain::{DeviceKind, Role, SecretKind};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Result, StoreError};

mod account;
mod audit;
mod device;
mod household;
mod jobs;
mod member;
mod notify;
mod secrets;
mod session;
mod status;

pub use account::{AccountLockout, UserAccountRepo};
pub use audit::AuditRepo;
pub use device::{DeviceElevationRepo, DeviceRepo};
pub use household::HouseholdRepo;
pub use jobs::{Job, JobRepo, NewJob, Schedule, ScheduleRepo};
pub use member::{MemberLockout, MemberRepo};
pub use notify::{NotifyPref, NotifyPrefRepo};
pub use secrets::SecretRepo;
pub use session::{SessionRecord, SessionRepo};
pub use status::SystemStatusRepo;

pub(crate) fn role_str(r: Role) -> &'static str {
    match r {
        Role::Adult => "adult",
        Role::Child => "child",
    }
}

pub(crate) fn parse_role(s: &str) -> Result<Role> {
    match s {
        "adult" => Ok(Role::Adult),
        "child" => Ok(Role::Child),
        other => Err(StoreError::Corrupt(format!("role {other}"))),
    }
}

pub(crate) fn device_kind_str(k: DeviceKind) -> &'static str {
    match k {
        DeviceKind::Dashboard => "dashboard",
    }
}

pub(crate) fn parse_device_kind(s: &str) -> Result<DeviceKind> {
    match s {
        "dashboard" => Ok(DeviceKind::Dashboard),
        other => Err(StoreError::Corrupt(format!("device kind {other}"))),
    }
}

pub(crate) fn secret_kind_str(k: SecretKind) -> &'static str {
    match k {
        SecretKind::OauthToken => "oauth_token",
        SecretKind::Other => "other",
    }
}

/// After a version-guarded UPDATE matched nothing: distinguish a stale version from a missing row.
pub(crate) async fn conflict_or_missing(
    conn: &mut PgConnection,
    table: &'static str,
    id: Uuid,
) -> StoreError {
    let sql = format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE id = $1)");
    match sqlx::query_scalar::<_, bool>(&sql)
        .bind(id)
        .fetch_one(conn)
        .await
    {
        Ok(true) => StoreError::Conflict,
        Ok(false) => StoreError::NotFound,
        Err(e) => e.into(),
    }
}
