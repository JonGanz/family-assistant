//! M3.5 setup and system DTOs (plan/M3.5.md, Contracts: fa-domain).
//!
//! Wire formats: instants RFC 3339 (`DateTime<Utc>`), colours `#rrggbb`, currency ISO 4217,
//! `tz` an IANA name, `base_url` a browser `location.origin` (`scheme://host[:port]`).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::entities::Units;

macro_rules! dto {
    ($(#[$m:meta])* $v:vis struct $name:ident { $($body:tt)* }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        $v struct $name { $($body)* }
    };
}

dto!(
    pub struct SetupStatus {
        /// No household yet.
        pub needed: bool,
        /// A household exists but no adult account.
        pub owner_needed: bool,
        /// `setup_completed_at` is set.
        pub completed: bool,
        /// This request arrived over HTTPS.
        pub secure: bool,
        /// `scheme://host[:port]` of this request.
        pub origin: String,
        /// Secure and the host ends in `.ts.net`.
        pub via_tailscale: bool,
    }
);

dto!(
    pub struct SetupLocation {
        pub label: String,
        pub lat: f64,
        pub lon: f64,
    }
);

dto!(
    pub struct NewSetupHousehold {
        pub name: String,
        /// IANA time zone.
        pub tz: String,
        #[ts(as = "Option<SetupLocation>", optional = nullable)]
        pub location: Option<SetupLocation>,
        pub units: Units,
        /// ISO 4217.
        pub currency: String,
        /// The browser's `location.origin`.
        pub base_url: String,
    }
);

dto!(
    pub struct NewSetupOwner {
        pub display_name: String,
        /// `#rrggbb`.
        #[ts(optional = nullable)]
        pub color: Option<String>,
        pub username: String,
        pub password: String,
        /// 4 to 6 digits.
        #[ts(optional = nullable)]
        pub pin: Option<String>,
    }
);

dto!(
    pub struct MasterKeyRequest {
        pub password: String,
    }
);

dto!(
    pub struct MasterKeyView {
        /// Standard base64 of the 32 raw key bytes.
        pub key_base64: String,
    }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum SettingSource {
    Env,
    Db,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum SettingKind {
    Url,
    Bool,
    Text,
    Secret,
    Cidrs,
}

dto!(
    pub struct SettingView {
        pub key: String,
        pub kind: SettingKind,
        /// Always `null` for secrets.
        #[ts(type = "unknown", optional = nullable)]
        #[schema(value_type = Option<Object>)]
        pub value: Option<serde_json::Value>,
        pub is_set: bool,
        pub source: SettingSource,
        /// The source is `env`.
        pub locked: bool,
    }
);

dto!(
    pub struct ConnectionInfo {
        pub secure: bool,
        pub origin: String,
    }
);

dto!(
    pub struct SystemSettingsView {
        pub settings: Vec<SettingView>,
        pub connection: ConnectionInfo,
        /// RFC 3339.
        #[ts(as = "Option<String>", optional = nullable)]
        pub key_backup_confirmed_at: Option<DateTime<Utc>>,
    }
);

dto!(
    pub struct SystemSettingsPatch {
        /// `null` resets a key to the default; a secret takes a string.
        #[ts(type = "Record<string, unknown>")]
        #[schema(value_type = BTreeMap<String, Option<Object>>)]
        pub values: BTreeMap<String, Option<serde_json::Value>>,
    }
);

dto!(
    pub struct OwnPasswordChange {
        pub current_password: String,
        pub new_password: String,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_enums_use_snake_case() {
        assert_eq!(
            serde_json::to_string(&SettingSource::Env).unwrap(),
            "\"env\""
        );
        assert_eq!(
            serde_json::to_string(&SettingKind::Cidrs).unwrap(),
            "\"cidrs\""
        );
    }

    #[test]
    fn patch_null_resets() {
        let p: SystemSettingsPatch =
            serde_json::from_str(r#"{"values":{"a":null,"b":"x"}}"#).unwrap();
        assert_eq!(p.values["a"], None);
        assert_eq!(p.values["b"], Some(serde_json::json!("x")));
    }
}
