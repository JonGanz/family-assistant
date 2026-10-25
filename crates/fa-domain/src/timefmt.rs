//! serde helpers: times of day as `"HH:MM"` (`"HH:MM:SS"` is accepted on input).

use chrono::NaiveTime;
use serde::{Deserialize, Deserializer, Serializer};

pub fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .ok()
}

pub fn format_hhmm(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

pub mod hhmm {
    use super::*;

    pub fn serialize<S: Serializer>(t: &NaiveTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format_hhmm(*t))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveTime, D::Error> {
        let s = String::deserialize(d)?;
        parse_hhmm(&s).ok_or_else(|| serde::de::Error::custom("expected HH:MM"))
    }
}

pub mod hhmm_opt {
    use super::*;

    pub fn serialize<S: Serializer>(t: &Option<NaiveTime>, s: S) -> Result<S::Ok, S::Error> {
        match t {
            Some(t) => s.serialize_str(&format_hhmm(*t)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NaiveTime>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| parse_hhmm(&s).ok_or_else(|| serde::de::Error::custom("expected HH:MM")))
            .transpose()
    }
}

/// For PATCH bodies (`with = "dopt"`): absent = `None`, `null` = `Some(None)`,
/// value = `Some(Some(v))`. Pair with `default` and `skip_serializing_if = "Option::is_none"`.
pub mod dopt {
    use super::*;
    use serde::Serialize;

    pub fn serialize<T: Serialize, S: Serializer>(
        v: &Option<Option<T>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            Some(Some(v)) => v.serialize(s),
            _ => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(d).map(Some)
    }
}

/// [`dopt`] for `"HH:MM"` times.
pub mod dopt_hhmm {
    use super::*;

    pub fn serialize<S: Serializer>(
        t: &Option<Option<NaiveTime>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match t {
            Some(Some(t)) => s.serialize_str(&format_hhmm(*t)),
            _ => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<NaiveTime>>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| parse_hhmm(&s).ok_or_else(|| serde::de::Error::custom("expected HH:MM")))
            .transpose()
            .map(Some)
    }
}
