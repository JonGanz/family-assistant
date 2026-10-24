use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;
use url::Url;

use crate::{ConnectorError, Forecast, Http, error::Result};

pub const DEFAULT_BASE: &str = "https://api.open-meteo.com";
pub const CURRENT: &str = "temperature_2m,apparent_temperature,weather_code,wind_speed_10m,wind_gusts_10m,precipitation,is_day";
pub const HOURLY: &str = "temperature_2m,apparent_temperature,precipitation_probability,precipitation,snowfall,wind_speed_10m,wind_gusts_10m,uv_index,weather_code";
pub const DAILY: &str = "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max,precipitation_sum,sunrise,sunset,uv_index_max";

pub async fn forecast(http: &Http, base: &str, lat: f64, lon: f64, tz: &str) -> Result<Forecast> {
    let mut url = Url::parse(base).map_err(|e| ConnectorError::Parse(e.to_string()))?;
    url.path_segments_mut()
        .map_err(|_| ConnectorError::Parse("bad base url".into()))?
        .pop_if_empty()
        .extend(["v1", "forecast"]);
    url.query_pairs_mut()
        .append_pair("latitude", &lat.to_string())
        .append_pair("longitude", &lon.to_string())
        .append_pair("current", CURRENT)
        .append_pair("hourly", HOURLY)
        .append_pair("daily", DAILY)
        .append_pair("forecast_hours", "48")
        .append_pair("forecast_days", "7")
        .append_pair("temperature_unit", "celsius")
        .append_pair("wind_speed_unit", "kmh")
        .append_pair("precipitation_unit", "mm")
        .append_pair("timezone", tz);
    let mut json = http.get_json(http.get(&url)?).await?;
    let take = |json: &mut Value, k: &str| {
        json.get_mut(k)
            .map(Value::take)
            .ok_or_else(|| ConnectorError::Parse(format!("missing `{k}`")))
    };
    Ok(Forecast {
        current: take(&mut json, "current")?,
        hourly: take(&mut json, "hourly")?,
        daily: take(&mut json, "daily")?,
        timezone: json
            .get("timezone")
            .and_then(Value::as_str)
            .map(str::to_owned),
        utc_offset_seconds: json
            .get("utc_offset_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
    })
}

impl Forecast {
    fn local_to_utc(&self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        if let Some(tz) = self
            .timezone
            .as_deref()
            .and_then(|t| t.parse::<chrono_tz::Tz>().ok())
        {
            return tz
                .from_local_datetime(&naive)
                .earliest()
                .map(|d| d.with_timezone(&Utc));
        }
        Some((naive - chrono::Duration::seconds(i64::from(self.utc_offset_seconds))).and_utc())
    }

    fn parse_local(&self, s: &str) -> Option<DateTime<Utc>> {
        let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").ok()?;
        self.local_to_utc(naive)
    }

    pub fn current_f64(&self, field: &str) -> Option<f64> {
        self.current.get(field)?.as_f64()
    }

    /// Hourly series for `field`, with timestamps converted from the response timezone to UTC.
    pub fn hourly_series(&self, field: &str) -> Vec<(DateTime<Utc>, Option<f64>)> {
        series(&self.hourly, field, |s| self.parse_local(s))
    }

    pub fn daily_series(&self, field: &str) -> Vec<(NaiveDate, Option<f64>)> {
        series(&self.daily, field, |s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
        })
    }

    /// Daily date-time fields such as `sunrise` / `sunset`.
    pub fn daily_times(&self, field: &str) -> Vec<(NaiveDate, Option<DateTime<Utc>>)> {
        let days = self.daily.get("time").and_then(Value::as_array);
        let vals = self.daily.get(field).and_then(Value::as_array);
        let (Some(days), Some(vals)) = (days, vals) else {
            return Vec::new();
        };
        days.iter()
            .zip(vals)
            .filter_map(|(d, v)| {
                let d = NaiveDate::parse_from_str(d.as_str()?, "%Y-%m-%d").ok()?;
                Some((d, v.as_str().and_then(|s| self.parse_local(s))))
            })
            .collect()
    }
}

fn series<K>(
    section: &Value,
    field: &str,
    key: impl Fn(&str) -> Option<K>,
) -> Vec<(K, Option<f64>)> {
    let times = section.get("time").and_then(Value::as_array);
    let vals = section.get(field).and_then(Value::as_array);
    let (Some(times), Some(vals)) = (times, vals) else {
        return Vec::new();
    };
    times
        .iter()
        .zip(vals)
        .filter_map(|(t, v)| Some((key(t.as_str()?)?, v.as_f64())))
        .collect()
}
