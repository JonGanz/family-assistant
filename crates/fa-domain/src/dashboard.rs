//! Per-device dashboard layout (arch/Frontend.md §4.1).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ids::MemberId;
use crate::theme::{ThemeChoice, is_theme_id};

pub const MAX_PANELS: usize = 12;
pub const MAX_AGENDA_DAYS: u8 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Landscape,
    Portrait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AllMarker {
    #[serde(rename = "all")]
    All,
}

/// `"all"` or a list of member IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum MemberSelection {
    All(AllMarker),
    Ids(Vec<MemberId>),
}

/// A panel with a kind and kind-specific options. Unknown kinds and options are kept as they are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Panel {
    pub kind: String,
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Panel {
    fn new(kind: &str, extra: serde_json::Value) -> Self {
        Self {
            kind: kind.into(),
            extra: extra.as_object().cloned().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("agenda_days must be 1..={MAX_AGENDA_DAYS}")]
    AgendaDays,
    #[error("at most {MAX_PANELS} panels")]
    TooManyPanels,
    #[error("theme family and variant must match ^[a-z0-9][a-z0-9-]{{0,31}}$")]
    ThemeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardSettings {
    pub orientation: Orientation,
    pub theme: ThemeChoice,
    pub agenda_days: u8,
    pub members: MemberSelection,
    pub panels: Vec<Panel>,
}

impl DashboardSettings {
    /// Orientation and theme are enums, so unknown values already fail at deserialisation.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if !(1..=MAX_AGENDA_DAYS).contains(&self.agenda_days) {
            return Err(SettingsError::AgendaDays);
        }
        if self.panels.len() > MAX_PANELS {
            return Err(SettingsError::TooManyPanels);
        }
        if !is_theme_id(&self.theme.family)
            || self
                .theme
                .variant
                .as_deref()
                .is_some_and(|v| !is_theme_id(v))
        {
            return Err(SettingsError::ThemeId);
        }
        Ok(())
    }
}

/// The "Family hub" preset.
impl Default for DashboardSettings {
    fn default() -> Self {
        use serde_json::json;
        Self {
            orientation: Orientation::Landscape,
            theme: ThemeChoice::default(),
            agenda_days: 3,
            members: MemberSelection::All(AllMarker::All),
            panels: vec![
                Panel::new("clock", json!({})),
                Panel::new("weather", json!({"hourly": true})),
                Panel::new("advisories", json!({})),
                Panel::new("agenda", json!({})),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        DashboardSettings::default().validate().unwrap();
    }

    #[test]
    fn bounds() {
        let mut s = DashboardSettings::default();
        for (d, ok) in [(0, false), (1, true), (14, true), (15, false)] {
            s.agenda_days = d;
            assert_eq!(s.validate().is_ok(), ok, "{d}");
        }
        s.agenda_days = 3;
        s.panels = (0..12)
            .map(|_| Panel::new("clock", serde_json::json!({})))
            .collect();
        assert!(s.validate().is_ok());
        s.panels.push(Panel::new("clock", serde_json::json!({})));
        assert_eq!(s.validate(), Err(SettingsError::TooManyPanels));
    }

    #[test]
    fn unknown_orientation_and_theme_rejected() {
        let j = r#"{"orientation":"sideways","theme":"auto","agenda_days":3,"members":"all","panels":[]}"#;
        assert!(serde_json::from_str::<DashboardSettings>(j).is_err());
        let j = r#"{"orientation":"portrait","theme":"neon","agenda_days":3,"members":"all","panels":[]}"#;
        assert!(serde_json::from_str::<DashboardSettings>(j).is_err());
    }

    #[test]
    fn theme_id_validated() {
        let mut s = DashboardSettings::default();
        s.theme.family = "Nord".into();
        assert_eq!(s.validate(), Err(SettingsError::ThemeId));
        s.theme.family = "nord".into();
        s.theme.variant = Some("a b".into());
        assert_eq!(s.validate(), Err(SettingsError::ThemeId));
        s.theme.variant = Some("polar".into());
        assert!(s.validate().is_ok());
    }

    #[test]
    fn unknown_panels_round_trip() {
        let j = serde_json::json!({
            "orientation": "portrait", "theme": {"family": "catppuccin", "mode": "dark"}, "agenda_days": 5,
            "members": "all",
            "panels": [{"kind": "clock"}, {"kind": "hologram", "depth": 3, "opts": {"a": [1, 2]}}]
        });
        let s: DashboardSettings = serde_json::from_value(j.clone()).unwrap();
        assert_eq!(s.panels[1].kind, "hologram");
        assert_eq!(serde_json::to_value(&s).unwrap(), j);
    }

    #[test]
    fn members_all_or_list() {
        let id = MemberId::new();
        let all: MemberSelection = serde_json::from_str("\"all\"").unwrap();
        assert_eq!(all, MemberSelection::All(AllMarker::All));
        assert_eq!(serde_json::to_string(&all).unwrap(), "\"all\"");
        let ids: MemberSelection = serde_json::from_str(&format!("[\"{id}\"]")).unwrap();
        assert_eq!(ids, MemberSelection::Ids(vec![id]));
        assert_eq!(serde_json::to_string(&ids).unwrap(), format!("[\"{id}\"]"));
        assert!(serde_json::from_str::<MemberSelection>("\"some\"").is_err());
    }
}
