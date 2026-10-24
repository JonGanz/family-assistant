#![allow(dead_code)]

use chrono::{DateTime, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use fa_domain::{HouseholdId, WeatherSnapshot};
use serde_json::{Value, json};

pub const TZ: Tz = chrono_tz::Europe::Berlin;

pub fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
}

/// Synthetic Open-Meteo-shaped data: hourly samples from `start`, local times in `TZ`.
pub struct Synth {
    pub start: DateTime<Utc>,
    pub n: usize,
    series: Vec<(&'static str, Vec<Option<f64>>)>,
}

impl Synth {
    /// Every series defaults to a calm constant.
    pub fn new(start: DateTime<Utc>, n: usize) -> Self {
        let mut s = Self {
            start,
            n,
            series: Vec::new(),
        };
        for (k, v) in [
            ("temperature_2m", 10.0),
            ("apparent_temperature", 10.0),
            ("precipitation_probability", 0.0),
            ("precipitation", 0.0),
            ("snowfall", 0.0),
            ("wind_speed_10m", 5.0),
            ("wind_gusts_10m", 10.0),
            ("uv_index", 0.0),
            ("weather_code", 1.0),
        ] {
            s.series.push((k, vec![Some(v); n]));
        }
        s
    }

    pub fn set(mut self, key: &str, f: impl Fn(usize) -> Option<f64>) -> Self {
        let col = self.series.iter_mut().find(|(k, _)| *k == key).unwrap();
        for i in 0..self.n {
            col.1[i] = f(i);
        }
        self
    }

    pub fn at(mut self, key: &str, idx: usize, v: Option<f64>) -> Self {
        let col = self.series.iter_mut().find(|(k, _)| *k == key).unwrap();
        col.1[idx] = v;
        self
    }

    pub fn hourly(&self) -> Value {
        let times: Vec<String> = (0..self.n)
            .map(|i| {
                (self.start + Duration::hours(i as i64))
                    .with_timezone(&TZ)
                    .format("%Y-%m-%dT%H:%M")
                    .to_string()
            })
            .collect();
        let mut o = serde_json::Map::new();
        o.insert("time".into(), json!(times));
        for (k, v) in &self.series {
            o.insert((*k).into(), json!(v));
        }
        Value::Object(o)
    }

    pub fn daily(&self) -> Value {
        let d0 = self.start.with_timezone(&TZ).date_naive();
        let days: Vec<String> = (0..7)
            .map(|i| (d0 + Duration::days(i)).format("%Y-%m-%d").to_string())
            .collect();
        let sun = |h: u32, m: u32| -> Vec<String> {
            (0..7)
                .map(|i| {
                    format!(
                        "{}T{h:02}:{m:02}",
                        (d0 + Duration::days(i)).format("%Y-%m-%d")
                    )
                })
                .collect()
        };
        json!({
            "time": days,
            "weather_code": [1,1,1,1,1,1,1],
            "temperature_2m_max": [12,12,12,12,12,12,12],
            "temperature_2m_min": [3,3,3,3,3,3,3],
            "precipitation_probability_max": [5,5,5,5,5,5,5],
            "precipitation_sum": [0,0,0,0,0,0,0],
            "sunrise": sun(7, 18),
            "sunset": sun(17, 39),
            "uv_index_max": [2,2,2,2,2,2,2],
        })
    }

    pub fn current(&self) -> Value {
        json!({"time": "2026-10-24T12:00", "temperature_2m": 10.0, "apparent_temperature": 9.0,
               "weather_code": 1, "wind_speed_10m": 5.0, "wind_gusts_10m": 10.0,
               "precipitation": 0.0, "is_day": 1})
    }

    pub fn api_body(&self) -> Value {
        json!({
            "latitude": 52.52, "longitude": 13.42, "utc_offset_seconds": 7200,
            "timezone": "Europe/Berlin",
            "current": self.current(), "hourly": self.hourly(), "daily": self.daily(),
        })
    }

    pub fn snapshot(&self) -> WeatherSnapshot {
        WeatherSnapshot {
            id: uuid::Uuid::nil(),
            household_id: HouseholdId::new(),
            fetched_at: self.start,
            provider: "open-meteo".into(),
            current: self.current(),
            hourly: self.hourly(),
            daily: self.daily(),
        }
    }
}
