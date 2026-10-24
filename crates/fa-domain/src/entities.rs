use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::*;

macro_rules! text_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, TS)]
        #[serde(rename_all = "snake_case")]
        #[sqlx(type_name = "text", rename_all = "snake_case")]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        pub enum $name { $($var),+ }
    };
}

text_enum!(Role { Adult, Child });
text_enum!(DeviceKind { Dashboard });
text_enum!(SecretKind { OauthToken, Other });

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}

// ---- Entities (internal, may carry hashes; never serialised to clients) ----

#[derive(Debug, Clone)]
pub struct Household {
    pub id: HouseholdId,
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub meal_plan_horizon_weeks: i32,
    pub dashboard_completion_override: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub id: MemberId,
    pub household_id: HouseholdId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    pub color: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    pub interests: Vec<String>,
    pub hobbies: Vec<String>,
    pub responsibilities: Vec<String>,
    /// Argon2id hash, adults only.
    pub pin_hash: Option<String>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserAccount {
    pub id: UserAccountId,
    pub household_id: HouseholdId,
    pub member_id: MemberId,
    pub username: String,
    pub password_hash: String,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub household_id: HouseholdId,
    pub kind: DeviceKind,
    pub name: String,
    pub token_hash: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeviceElevation {
    pub device_id: DeviceId,
    pub member_id: MemberId,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: AuditEntryId,
    pub household_id: HouseholdId,
    pub actor_member: Option<MemberId>,
    pub device: Option<DeviceId>,
    pub action: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub at: DateTime<Utc>,
}

/// Provenance of a derived entity (shown as a "from" link in the UI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct SourceRef {
    pub kind: SourceKind,
    pub message_id: Option<String>,
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum SourceKind {
    Email,
    Ics,
    Weather,
    Manual,
    System,
}

// ---- API DTOs (exported to TypeScript) ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct HouseholdDto {
    pub id: HouseholdId,
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub meal_plan_horizon_weeks: i32,
    pub dashboard_completion_override: bool,
}

impl From<&Household> for HouseholdDto {
    fn from(h: &Household) -> Self {
        Self {
            id: h.id,
            name: h.name.clone(),
            tz: h.tz.clone(),
            location: h.location.clone(),
            meal_plan_horizon_weeks: h.meal_plan_horizon_weeks,
            dashboard_completion_override: h.dashboard_completion_override,
        }
    }
}

/// Public fields, visible to children and devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MemberPublicDto {
    pub id: MemberId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    pub color: Option<String>,
}

/// Full member view for adults. Never includes the PIN hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct MemberDto {
    pub id: MemberId,
    pub display_name: String,
    pub role: Role,
    pub avatar: Option<String>,
    pub color: Option<String>,
    #[ts(type = "string | null")]
    pub birthdate: Option<NaiveDate>,
    pub grade: Option<String>,
    pub interests: Vec<String>,
    pub hobbies: Vec<String>,
    pub responsibilities: Vec<String>,
    pub has_pin: bool,
    pub has_account: bool,
}

impl From<&Member> for MemberPublicDto {
    fn from(m: &Member) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name.clone(),
            role: m.role,
            avatar: m.avatar.clone(),
            color: m.color.clone(),
        }
    }
}

impl MemberDto {
    pub fn from_member(m: &Member, has_account: bool) -> Self {
        Self {
            id: m.id,
            display_name: m.display_name.clone(),
            role: m.role,
            avatar: m.avatar.clone(),
            color: m.color.clone(),
            birthdate: m.birthdate,
            grade: m.grade.clone(),
            interests: m.interests.clone(),
            hobbies: m.hobbies.clone(),
            responsibilities: m.responsibilities.clone(),
            has_pin: m.pin_hash.is_some(),
            has_account,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct UserAccountDto {
    pub id: UserAccountId,
    pub member_id: MemberId,
    pub username: String,
    pub disabled: bool,
}

impl From<&UserAccount> for UserAccountDto {
    fn from(a: &UserAccount) -> Self {
        Self {
            id: a.id,
            member_id: a.member_id,
            username: a.username.clone(),
            disabled: a.disabled_at.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct DeviceDto {
    pub id: DeviceId,
    pub kind: DeviceKind,
    pub name: String,
    pub last_seen: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl From<&Device> for DeviceDto {
    fn from(d: &Device) -> Self {
        Self {
            id: d.id,
            kind: d.kind,
            name: d.name.clone(),
            last_seen: d.last_seen,
            revoked: d.revoked_at.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct DeviceElevationDto {
    pub device_id: DeviceId,
    pub member_id: MemberId,
    pub expires_at: DateTime<Utc>,
}

impl From<&DeviceElevation> for DeviceElevationDto {
    fn from(e: &DeviceElevation) -> Self {
        Self {
            device_id: e.device_id,
            member_id: e.member_id,
            expires_at: e.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct AuditEntryDto {
    pub id: AuditEntryId,
    pub actor_member: Option<MemberId>,
    pub device: Option<DeviceId>,
    pub action: String,
    pub subject_type: String,
    pub subject_id: Option<String>,
    pub at: DateTime<Utc>,
}

impl From<&AuditEntry> for AuditEntryDto {
    fn from(a: &AuditEntry) -> Self {
        Self {
            id: a.id,
            actor_member: a.actor_member,
            device: a.device,
            action: a.action.clone(),
            subject_type: a.subject_type.clone(),
            subject_id: a.subject_id.clone(),
            at: a.at,
        }
    }
}
