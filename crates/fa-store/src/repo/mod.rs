//! Repositories: stateless structs whose functions take `&mut PgConnection` (a [`crate::Tx`] derefs to it).

use fa_domain::{DeviceKind, Role, SecretKind};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Result, StoreError};

mod account;
mod advisory;
mod audit;
mod avatar;
mod calendar;
mod calendar_source;
mod device;
mod event;
mod ext_t05;
mod ext_t06;
mod ext_t06m2;
mod ext_t07;
mod ext_t08;
mod ext_t08m2;
mod household;
mod jobs;
mod member;
mod notification;
mod notify;
mod oauth;
mod routine;
mod secrets;
mod session;
mod status;
mod task;
mod task_definition;
mod task_list;
mod weather;
mod weather_rule;

pub use account::{AccountLockout, UserAccountRepo};
pub use advisory::AdvisoryRepo;
pub use audit::AuditRepo;
pub use avatar::MemberAvatarRepo;
pub use calendar::CalendarRepo;
pub use calendar_source::CalendarSourceRepo;
pub use device::{DeviceElevationRepo, DeviceRepo};
pub use event::{EventRepo, EventRow, NewOccurrence};
pub use ext_t08::ConfigExtRepo;
pub use household::HouseholdRepo;
pub use jobs::{Job, JobCounts, JobRepo, NewJob, Schedule, ScheduleRepo};
pub use member::{MemberLockout, MemberRepo};
pub use notification::{NewNotification, NotificationRepo};
pub use notify::{NotifyPref, NotifyPrefRepo};
pub use oauth::{OAuthAccountRepo, OAuthPending, OAuthPendingRepo};
pub use routine::{
    MemberAwayRepo, RoutineFull, RoutineRepo, RoutineRunRepo, RoutineStepSpec, RunRow, RunStepRow,
};
pub use secrets::SecretRepo;
pub use session::{SessionRecord, SessionRepo};
pub use status::SystemStatusRepo;
pub use task::{NewTaskRow, TaskFilter, TaskRepo, TaskRow};
pub use task_definition::TaskDefinitionRepo;
pub use task_list::TaskListRepo;
pub use weather::WeatherSnapshotRepo;
pub use weather_rule::WeatherRuleRepo;

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

pub(crate) fn enum_str<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

pub(crate) fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|_| StoreError::Corrupt(format!("enum value {s}")))
}
