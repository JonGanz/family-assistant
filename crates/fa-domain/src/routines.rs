//! Pure routine helpers (arch/Tasks.md §4.6).

use chrono::{Datelike, NaiveDate, Weekday};

use crate::entities::DayFilter;

fn is_weekday(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Does a routine with `filter` run on `date`?
///
/// `school_titles` are the all-day event titles on the school calendar that day (`None` =
/// no school calendar, which makes `school_days` behave like `weekdays`). A title matching
/// any non-empty keyword, case-insensitively as a substring, means no school.
pub fn day_qualifies(
    filter: DayFilter,
    date: NaiveDate,
    school_titles: Option<&[String]>,
    keywords: &[String],
) -> bool {
    match filter {
        DayFilter::Daily => true,
        DayFilter::Weekdays => is_weekday(date),
        DayFilter::Weekends => !is_weekday(date),
        DayFilter::SchoolDays => {
            if !is_weekday(date) {
                return false;
            }
            let Some(titles) = school_titles else {
                return true;
            };
            let keywords: Vec<String> = keywords
                .iter()
                .map(|k| k.trim().to_lowercase())
                .filter(|k| !k.is_empty())
                .collect();
            !titles.iter().any(|t| {
                let t = t.to_lowercase();
                keywords.iter().any(|k| t.contains(k))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        // 2026-06-01 is a Monday.
        NaiveDate::from_ymd_opt(2026, 6, day).unwrap()
    }
    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn simple_filters() {
        for day in 1..=7 {
            let weekend = day >= 6;
            assert!(day_qualifies(DayFilter::Daily, d(day), None, &[]));
            assert_eq!(
                day_qualifies(DayFilter::Weekdays, d(day), None, &[]),
                !weekend
            );
            assert_eq!(
                day_qualifies(DayFilter::Weekends, d(day), None, &[]),
                weekend
            );
        }
    }

    #[test]
    fn school_days() {
        let kw = s(&["no school", "holiday", "inset", "closed"]);
        // No calendar: weekdays.
        assert!(day_qualifies(DayFilter::SchoolDays, d(1), None, &kw));
        assert!(!day_qualifies(DayFilter::SchoolDays, d(6), None, &kw));
        // Calendar with nothing special.
        assert!(day_qualifies(
            DayFilter::SchoolDays,
            d(2),
            Some(&s(&["Sports day"])),
            &kw
        ));
        assert!(day_qualifies(DayFilter::SchoolDays, d(2), Some(&[]), &kw));
        // Case-insensitive substring.
        assert!(!day_qualifies(
            DayFilter::SchoolDays,
            d(2),
            Some(&s(&["INSET Day (staff)"])),
            &kw
        ));
        assert!(!day_qualifies(
            DayFilter::SchoolDays,
            d(2),
            Some(&s(&["Bank Holiday"])),
            &kw
        ));
        // Weekends never qualify, even with a calendar.
        assert!(!day_qualifies(DayFilter::SchoolDays, d(7), Some(&[]), &kw));
        // Empty keywords match nothing.
        assert!(day_qualifies(
            DayFilter::SchoolDays,
            d(2),
            Some(&s(&["Anything"])),
            &s(&[""])
        ));
    }
}
