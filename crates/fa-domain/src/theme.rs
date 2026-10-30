//! Member hues and per-device theme choice (D-080, D-081).

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

/// Family used when a legacy `"auto" | "light" | "dark"` string is read.
pub const DEFAULT_THEME_FAMILY: &str = "catppuccin";

/// One of eight named hues; each theme supplies its own shade (wire: lowercase name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum Hue {
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Pink,
}

impl Hue {
    pub const ALL: [Hue; 8] = [
        Hue::Red,
        Hue::Orange,
        Hue::Yellow,
        Hue::Green,
        Hue::Cyan,
        Hue::Blue,
        Hue::Purple,
        Hue::Pink,
    ];
    pub const DEFAULT: Hue = Hue::Blue;

    pub fn as_str(self) -> &'static str {
        match self {
            Hue::Red => "red",
            Hue::Orange => "orange",
            Hue::Yellow => "yellow",
            Hue::Green => "green",
            Hue::Cyan => "cyan",
            Hue::Blue => "blue",
            Hue::Purple => "purple",
            Hue::Pink => "pink",
        }
    }
}

impl fmt::Display for Hue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown hue {0:?}")]
pub struct ParseHueError(pub String);

impl FromStr for Hue {
    type Err = ParseHueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Hue::ALL
            .into_iter()
            .find(|h| h.as_str() == s)
            .ok_or_else(|| ParseHueError(s.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

/// A device's theme. Reads the legacy string form; always writes the object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, ToSchema)]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub struct ThemeChoice {
    /// Slug, `^[a-z0-9][a-z0-9-]{0,31}$`.
    pub family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub variant: Option<String>,
    pub mode: ThemeMode,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ThemeRepr {
    Legacy(ThemeMode),
    Object {
        family: String,
        #[serde(default)]
        variant: Option<String>,
        mode: ThemeMode,
    },
}

impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match ThemeRepr::deserialize(d)? {
            ThemeRepr::Legacy(mode) => ThemeChoice {
                family: DEFAULT_THEME_FAMILY.into(),
                variant: None,
                mode,
            },
            ThemeRepr::Object {
                family,
                variant,
                mode,
            } => ThemeChoice {
                family,
                variant,
                mode,
            },
        })
    }
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self {
            family: DEFAULT_THEME_FAMILY.into(),
            variant: None,
            mode: ThemeMode::Auto,
        }
    }
}

/// `^[a-z0-9][a-z0-9-]{0,31}$`
pub fn is_theme_id(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=32).contains(&b.len())
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_round_trip_and_rejects() {
        for h in Hue::ALL {
            let j = serde_json::to_string(&h).unwrap();
            assert_eq!(j, format!("\"{h}\""));
            assert_eq!(serde_json::from_str::<Hue>(&j).unwrap(), h);
            assert_eq!(h.as_str().parse::<Hue>().unwrap(), h);
        }
        assert!(serde_json::from_str::<Hue>("\"#ff0000\"").is_err());
        assert!(serde_json::from_str::<Hue>("\"teal\"").is_err());
        assert!("teal".parse::<Hue>().is_err());
        assert_eq!(Hue::DEFAULT, Hue::Blue);
    }

    #[test]
    fn theme_reads_legacy_and_object() {
        let t: ThemeChoice = serde_json::from_str("\"dark\"").unwrap();
        assert_eq!(t.family, DEFAULT_THEME_FAMILY);
        assert_eq!(t.mode, ThemeMode::Dark);
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!({"family":"catppuccin","mode":"dark"})
        );
        let o: ThemeChoice =
            serde_json::from_str(r#"{"family":"nord","variant":"polar","mode":"light"}"#).unwrap();
        assert_eq!(o.variant.as_deref(), Some("polar"));
        assert_eq!(
            serde_json::from_value::<ThemeChoice>(serde_json::to_value(&o).unwrap()).unwrap(),
            o
        );
        assert!(serde_json::from_str::<ThemeChoice>("\"neon\"").is_err());
    }

    #[test]
    fn theme_ids() {
        for ok in ["a", "catppuccin", "0x", "a-b", &"a".repeat(32)] {
            assert!(is_theme_id(ok), "{ok}");
        }
        for bad in ["", "Abc", "a b", "-a", "a_b", &"a".repeat(33)] {
            assert!(!is_theme_id(bad), "{bad}");
        }
    }
}
