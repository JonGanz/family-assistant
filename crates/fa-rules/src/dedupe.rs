//! Event dedupe key and title similarity (Ingest.md 8.1).

use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;

use crate::text::words;

fn normalise(title: &str) -> String {
    words(title).join("-")
}

/// `<title words joined by '-'>|<local date>` and, for timed events, `|<HH:MM>` of the
/// local start rounded down to 15 minutes.
pub fn event_dedupe_key(title: &str, start: DateTime<Utc>, all_day: bool, tz: Tz) -> String {
    let local = start.with_timezone(&tz);
    let mut key = format!("{}|{}", normalise(title), local.format("%Y-%m-%d"));
    if !all_day {
        let t = local.time();
        let minute = t.minute() - t.minute() % 15;
        key.push_str(&format!("|{:02}:{:02}", t.hour(), minute));
    }
    key
}

/// Jaro-Winkler on the lower-cased alphanumeric words of both titles.
pub fn title_similarity(a: &str, b: &str) -> f64 {
    strsim::jaro_winkler(&words(a).join(" "), &words(b).join(" "))
}
