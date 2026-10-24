//! Rule evaluation against a stored snapshot (arch/Feeds.md section 4).

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use fa_connectors::Forecast;
use fa_domain::{Units, WeatherSnapshot};

use crate::dsl::{Agg, Cond, Field, Rule, Window};
use crate::units::format_value;

pub type Span = (DateTime<Utc>, DateTime<Utc>);

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub matches: bool,
    /// Metric value of the first matching condition (or, without a match, the first with data).
    pub value: Option<f64>,
    pub when: Option<Span>,
    pub field: Option<Field>,
    pub window: Option<Window>,
    /// Local date the window nominally starts on (advisory dedupe key).
    pub key_date: Option<NaiveDate>,
}

impl Evaluation {
    fn empty() -> Self {
        Self {
            matches: false,
            value: None,
            when: None,
            field: None,
            window: None,
            key_date: None,
        }
    }
}

struct CondEval {
    matches: bool,
    value: Option<f64>,
    span: Span,
    key_date: NaiveDate,
}

/// A snapshot's hourly data, ready for evaluation.
pub struct SnapshotData {
    forecast: Forecast,
}

impl SnapshotData {
    pub fn new(snapshot: &WeatherSnapshot, tz: Tz) -> Self {
        Self {
            forecast: Forecast {
                current: snapshot.current.clone(),
                hourly: snapshot.hourly.clone(),
                daily: snapshot.daily.clone(),
                timezone: Some(tz.name().to_string()),
                utc_offset_seconds: 0,
            },
        }
    }

    pub fn forecast(&self) -> &Forecast {
        &self.forecast
    }
}

pub fn evaluate(rule: &Rule, snapshot: &WeatherSnapshot, tz: Tz, now: DateTime<Utc>) -> Evaluation {
    evaluate_data(rule, &SnapshotData::new(snapshot, tz), tz, now)
}

pub fn evaluate_data(rule: &Rule, data: &SnapshotData, tz: Tz, now: DateTime<Utc>) -> Evaluation {
    let mut fallback: Option<Evaluation> = None;
    for group in &rule.groups {
        let results: Vec<(&Cond, CondEval)> = group
            .iter()
            .map(|c| (c, eval_cond(c, data, tz, now)))
            .collect();
        let all = results.iter().all(|(_, r)| r.matches);
        let rep = if all {
            results.first()
        } else {
            results.iter().find(|(_, r)| r.value.is_some())
        };
        let Some((cond, r)) = rep else { continue };
        let ev = Evaluation {
            matches: all,
            value: r.value,
            when: Some(r.span),
            field: Some(cond.field),
            window: Some(cond.window),
            key_date: Some(r.key_date),
        };
        if all {
            return ev;
        }
        if fallback.is_none() {
            fallback = Some(ev);
        }
    }
    fallback.unwrap_or_else(Evaluation::empty)
}

fn eval_cond(cond: &Cond, data: &SnapshotData, tz: Tz, now: DateTime<Utc>) -> CondEval {
    let (start, end, key_date) = resolve_window(cond.window, now, tz);
    let vals: Vec<f64> = data
        .forecast
        .hourly_series(cond.field.series())
        .into_iter()
        .filter(|(t, _)| *t >= start && *t < end)
        .filter_map(|(_, v)| v)
        .collect();
    let (matches, value) = if vals.is_empty() {
        (false, None)
    } else {
        match cond.agg {
            Agg::Any => {
                let hit = vals.iter().copied().find(|v| cond.op.test(*v, cond.value));
                (hit.is_some(), hit.or(vals.first().copied()))
            }
            agg => {
                let v = match agg {
                    Agg::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
                    Agg::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    Agg::Sum => vals.iter().sum(),
                    _ => vals.iter().sum::<f64>() / vals.len() as f64,
                };
                (cond.op.test(v, cond.value), Some(v))
            }
        }
    };
    CondEval {
        matches,
        value,
        span: (start, end),
        key_date,
    }
}

/// Local wall-clock `hour:00` on `date`, as UTC (skips DST gaps forward).
fn local_at(tz: Tz, date: NaiveDate, hour: u32) -> DateTime<Utc> {
    let naive = date.and_hms_opt(hour, 0, 0).unwrap_or_default();
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| {
            tz.from_local_datetime(&(naive + Duration::hours(1)))
                .earliest()
        })
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|| naive.and_utc())
}

/// Window bounds plus the local date the window nominally starts on.
pub fn resolve_window(
    w: Window,
    now: DateTime<Utc>,
    tz: Tz,
) -> (DateTime<Utc>, DateTime<Utc>, NaiveDate) {
    let today = now.with_timezone(&tz).date_naive();
    let next_day = today.succ_opt().unwrap_or(today);
    match w {
        Window::Next(n) => (now, now + Duration::hours(i64::from(n)), today),
        Window::Today => (now, local_at(tz, next_day, 0), today),
        Window::Tonight => {
            use chrono::Timelike;
            if now.with_timezone(&tz).hour() < 8 {
                let yesterday = today.pred_opt().unwrap_or(today);
                (now, local_at(tz, today, 8), yesterday)
            } else {
                (
                    now.max(local_at(tz, today, 18)),
                    local_at(tz, next_day, 8),
                    today,
                )
            }
        }
        Window::Tomorrow => {
            let after = next_day.succ_opt().unwrap_or(next_day);
            (local_at(tz, next_day, 0), local_at(tz, after, 0), next_day)
        }
    }
}

/// Fills `{value}`, `{when}` and `{field}` in a rule message.
pub fn render_message(template: &str, ev: &Evaluation, units: Units) -> String {
    let value = match (ev.field, ev.value) {
        (Some(f), Some(v)) => format_value(f, v, units),
        _ => String::new(),
    };
    let when = ev.window.map(Window::label).unwrap_or_default();
    let field = ev.field.map(Field::label).unwrap_or_default();
    template
        .replace("{value}", &value)
        .replace("{when}", &when)
        .replace("{field}", field)
}
