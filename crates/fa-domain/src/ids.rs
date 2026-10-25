use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
            sqlx::Type,
            TS,
            utoipa::ToSchema,
        )]
        #[sqlx(transparent)]
        #[ts(
            export,
            export_to = "../../../apps/web/src/gen/types/",
            type = "string"
        )]
        pub struct $name(pub Uuid);

        impl $name {
            /// New time-ordered (v7) id.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

id_type!(HouseholdId);
id_type!(MemberId);
id_type!(UserAccountId);
id_type!(SessionId);
id_type!(DeviceId);
id_type!(AuditEntryId);
id_type!(SecretId);
id_type!(JobId);
id_type!(OAuthAccountId);
id_type!(CalendarSourceId);
id_type!(CalendarId);
id_type!(EventId);
id_type!(WeatherRuleId);
id_type!(AdvisoryId);
id_type!(TaskListId);
id_type!(TaskDefinitionId);
id_type!(TaskId);
id_type!(TaskStepId);
id_type!(RoutineId);
id_type!(RoutineStepId);
id_type!(RoutineRunId);
id_type!(MemberAwayId);
id_type!(NotificationId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v7_roundtrip() {
        let id = MemberId::new();
        assert_eq!(id.0.get_version_num(), 7);
        assert_eq!(id.to_string().parse::<MemberId>().unwrap(), id);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
    }
}
