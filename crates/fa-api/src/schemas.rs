//! OpenAPI-only mirrors of the `fa-domain` DTOs (which derive `ts_rs::TS`, not `ToSchema`).
#![allow(dead_code)]

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Problem {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
}

#[derive(Serialize, ToSchema)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}

#[derive(Serialize, ToSchema)]
pub struct HouseholdDto {
    #[schema(value_type = String, format = Uuid)]
    pub id: String,
    pub name: String,
    pub tz: String,
    pub location: Option<Location>,
    pub meal_plan_horizon_weeks: i32,
    pub dashboard_completion_override: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MemberPublicDto {
    #[schema(value_type = String, format = Uuid)]
    pub id: String,
    pub display_name: String,
    /// `adult` or `child`.
    pub role: String,
    pub avatar: Option<String>,
    pub color: Option<String>,
}

/// Full member view (adults). `version` is the optimistic-concurrency value for `If-Match`.
#[derive(Serialize, ToSchema)]
pub struct MemberDto {
    #[schema(value_type = String, format = Uuid)]
    pub id: String,
    pub display_name: String,
    pub role: String,
    pub avatar: Option<String>,
    pub color: Option<String>,
    pub birthdate: Option<String>,
    pub grade: Option<String>,
    pub interests: Vec<String>,
    pub hobbies: Vec<String>,
    pub responsibilities: Vec<String>,
    pub has_pin: bool,
    pub has_account: bool,
    pub version: i32,
}

#[derive(Serialize, ToSchema)]
pub struct DeviceDto {
    #[schema(value_type = String, format = Uuid)]
    pub id: String,
    pub kind: String,
    pub name: String,
    pub last_seen: Option<String>,
    pub revoked: bool,
}

#[derive(Serialize, ToSchema)]
pub struct UserAccountDto {
    #[schema(value_type = String, format = Uuid)]
    pub id: String,
    #[schema(value_type = String, format = Uuid)]
    pub member_id: String,
    pub username: String,
    pub disabled: bool,
}
