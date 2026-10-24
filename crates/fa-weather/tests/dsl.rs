mod common;

use common::*;
use fa_domain::Units;
use fa_weather::dsl::{Agg, Field, Op, Window};
use fa_weather::units::format_value;
use fa_weather::{DEFAULT_RULES, Rule, evaluate, parse, render_message};

fn p(s: &str) -> Rule {
    parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[test]
fn every_production_parses() {
    for agg in ["min", "max", "sum", "avg", "any"] {
        for f in Field::ALL {
            let r = p(&format!("{agg}({}, today) > 1", f.name()));
            assert_eq!(r.groups[0][0].field, f);
        }
    }
    assert_eq!(p("min(temp_c, today) > 1").groups[0][0].agg, Agg::Min);
    for (w, want) in [
        ("next 12h", Window::Next(12)),
        ("next 1 h", Window::Next(1)),
        ("next 48h", Window::Next(48)),
        ("today", Window::Today),
        ("tonight", Window::Tonight),
        ("tomorrow", Window::Tomorrow),
    ] {
        assert_eq!(p(&format!("max(uv, {w}) > 1")).groups[0][0].window, want);
    }
    for (o, want) in [
        ("<=", Op::Le),
        ("<", Op::Lt),
        (">=", Op::Ge),
        (">", Op::Gt),
        ("==", Op::Eq),
        ("!=", Op::Ne),
    ] {
        assert_eq!(p(&format!("max(uv, today) {o} 1")).groups[0][0].op, want);
    }
    assert_eq!(p("max(uv,today)>-2.5").groups[0][0].value, -2.5);
}

#[test]
fn case_and_whitespace_tolerant() {
    let a = p("  MAX ( Temp_C ,  NEXT  6  H )   >=\t 3 ");
    assert_eq!(a, p("max(temp_c, next 6h) >= 3"));
}

#[test]
fn and_binds_tighter_than_or() {
    let r = p("max(uv, today) > 1 or min(temp_c, today) < 2 and sum(precip_mm, next 3h) > 0");
    assert_eq!(r.groups.len(), 2);
    assert_eq!(r.groups[0].len(), 1);
    assert_eq!(r.groups[1].len(), 2);
    let r = p("max(uv, today) > 1 and min(temp_c, today) < 2 or any(code, today) == 95");
    assert_eq!((r.groups[0].len(), r.groups[1].len()), (2, 1));
}

#[test]
fn malformed_inputs_report_positions() {
    let cases: [(&str, usize, &str); 15] = [
        ("", 0, "aggregate"),
        ("foo(temp_c, today) > 1", 0, "aggregate"),
        ("min temp_c", 4, "`(`"),
        ("min(temp, today) > 1", 4, "field such as temp_c"),
        ("min(temp_c today) > 1", 11, "`,`"),
        ("min(temp_c, soon) > 1", 12, "window"),
        ("min(temp_c, next 0h) > 1", 17, "1 to 48"),
        ("min(temp_c, next 49h) > 1", 17, "1 to 48"),
        ("min(temp_c, next 1.5h) > 1", 17, "1 to 48"),
        ("min(temp_c, next 5x) > 1", 18, "`h`"),
        ("min(temp_c, today) = 1", 19, "comparison operator"),
        ("min(temp_c, today) > abc", 21, "number"),
        ("min(temp_c, today) > 1 xor", 23, "`and`, `or`"),
        ("min(temp_c, today) > 1 and", 26, "aggregate"),
        ("min(temp_c, today) > 1 $", 23, "unexpected character"),
    ];
    for (src, pos, frag) in cases {
        let e = parse(src).unwrap_err();
        assert_eq!(e.position, pos, "{src}: {e}");
        assert!(e.message.contains(frag), "{src}: {e}");
        assert!(e.message.ends_with(&format!("at {pos}")), "{src}: {e}");
    }
}

#[test]
fn canonical_round_trip() {
    let r =
        p("MIN( Temp_C ,next  12 H )<=  -2.5 OR any(code,today)==95 AND avg(uv, tomorrow) != 3");
    let s = r.to_string();
    assert_eq!(
        s,
        "min(temp_c, next 12h) <= -2.5 or any(code, today) == 95 and avg(uv, tomorrow) != 3"
    );
    assert_eq!(p(&s), r);
}

#[test]
fn default_rules_parse_and_are_canonical() {
    for d in &DEFAULT_RULES {
        assert_eq!(p(d.condition).to_string(), d.condition, "{}", d.name);
    }
}

// ---- evaluator ----

/// 48 h from 2026-10-24 10:00 UTC (12:00 CEST); DST ends 2026-10-25 03:00 local.
fn base() -> Synth {
    Synth::new(utc(2026, 10, 24, 10, 0), 48)
}

fn ev(s: &Synth, cond: &str, now: chrono::DateTime<chrono::Utc>) -> fa_weather::Evaluation {
    evaluate(&p(cond), &s.snapshot(), TZ, now)
}

#[test]
fn next_n_hours_window() {
    let s = base().set("precipitation", |i| Some(if i < 4 { 1.0 } else { 0.0 }));
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(&s, "sum(precip_mm, next 3h) >= 3", now);
    assert!(e.matches);
    assert_eq!(e.value, Some(3.0));
    assert_eq!(e.when, Some((now, utc(2026, 10, 24, 13, 0))));
    assert!(!ev(&s, "sum(precip_mm, next 3h) >= 4", now).matches);
    assert!(ev(&s, "sum(precip_mm, next 4h) >= 4", now).matches);
}

#[test]
fn today_window_ends_at_local_midnight() {
    // 22:00 UTC = 00:00 CEST on the 25th, so index 12 is excluded.
    let s = base()
        .at("temperature_2m", 11, Some(20.0))
        .at("temperature_2m", 12, Some(30.0));
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(&s, "max(temp_c, today) > 0", now);
    assert_eq!(e.value, Some(20.0));
    assert_eq!(e.when, Some((now, utc(2026, 10, 24, 22, 0))));
    assert_eq!(e.key_date.unwrap().to_string(), "2026-10-24");
}

#[test]
fn tonight_before_18_starts_at_18_and_crosses_dst() {
    // index 5 = 15:00Z (before the window), index 6 = 16:00Z = 18:00 CEST.
    let s = base()
        .at("temperature_2m", 5, Some(-5.0))
        .at("temperature_2m", 6, Some(-1.0));
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(&s, "min(temp_c, tonight) <= 0", now);
    assert!(e.matches);
    assert_eq!(e.value, Some(-1.0));
    // 08:00 on the 25th is CET (UTC+1) after the DST change.
    assert_eq!(
        e.when,
        Some((utc(2026, 10, 24, 16, 0), utc(2026, 10, 25, 7, 0)))
    );
}

#[test]
fn tonight_after_18_starts_now() {
    let s = base();
    let now = utc(2026, 10, 24, 17, 0); // 19:00 CEST
    let e = ev(&s, "min(temp_c, tonight) <= 99", now);
    assert_eq!(e.when, Some((now, utc(2026, 10, 25, 7, 0))));
    assert_eq!(e.key_date.unwrap().to_string(), "2026-10-24");
}

#[test]
fn tonight_after_midnight_belongs_to_previous_evening() {
    let s = base();
    let now = utc(2026, 10, 25, 5, 0); // 06:00 CET
    let e = ev(&s, "min(temp_c, tonight) <= 99", now);
    assert_eq!(e.when, Some((now, utc(2026, 10, 25, 7, 0))));
    assert_eq!(e.key_date.unwrap().to_string(), "2026-10-24");
}

#[test]
fn tomorrow_window_spans_the_25_hour_dst_day() {
    let s = base();
    let e = ev(&s, "max(temp_c, tomorrow) > 0", utc(2026, 10, 24, 10, 0));
    assert_eq!(
        e.when,
        Some((utc(2026, 10, 24, 22, 0), utc(2026, 10, 25, 23, 0)))
    );
    assert_eq!(e.key_date.unwrap().to_string(), "2026-10-25");
}

#[test]
fn aggregates_and_any() {
    let s = base().set("temperature_2m", |i| Some(i as f64));
    let now = utc(2026, 10, 24, 10, 0);
    assert_eq!(ev(&s, "min(temp_c, next 5h) >= 0", now).value, Some(0.0));
    assert_eq!(ev(&s, "max(temp_c, next 5h) >= 0", now).value, Some(4.0));
    assert_eq!(ev(&s, "sum(temp_c, next 5h) == 10", now).value, Some(10.0));
    let avg = ev(&s, "avg(temp_c, next 5h) == 2", now);
    assert!(avg.matches);
    assert_eq!(avg.value, Some(2.0));
    let any = ev(&s, "any(temp_c, next 5h) >= 3", now);
    assert!(any.matches);
    assert_eq!(any.value, Some(3.0));
    assert!(!ev(&s, "any(temp_c, next 5h) > 4", now).matches);
    assert!(ev(&s, "any(temp_c, next 5h) != 2", now).matches);
}

#[test]
fn null_gaps_are_skipped() {
    let s = base().set(
        "temperature_2m",
        |i| if i % 2 == 0 { None } else { Some(4.0) },
    );
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(&s, "avg(temp_c, next 6h) == 4", now);
    assert!(e.matches);
    let all_null = base().set("temperature_2m", |_| None);
    let e = ev(&all_null, "any(temp_c, next 6h) > -100", now);
    assert!(!e.matches);
    assert_eq!(e.value, None);
}

#[test]
fn empty_window_is_false_without_value() {
    let s = Synth::new(utc(2026, 10, 24, 10, 0), 6);
    let e = ev(&s, "max(temp_c, tomorrow) > -100", utc(2026, 10, 24, 10, 0));
    assert!(!e.matches);
    assert_eq!(e.value, None);
    let e = ev(&s, "min(temp_c, next 3h) > 100", utc(2026, 10, 30, 10, 0));
    assert!(!e.matches && e.value.is_none());
}

#[test]
fn and_or_take_value_from_first_matching_condition() {
    let s = base().set("precipitation", |i| Some(if i < 3 { 1.0 } else { 0.0 }));
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(
        &s,
        "max(temp_c, today) > 100 or sum(precip_mm, next 3h) >= 3",
        now,
    );
    assert!(e.matches);
    assert_eq!(e.value, Some(3.0));
    assert_eq!(e.window, Some(Window::Next(3)));
    assert_eq!(e.field, Some(Field::PrecipMm));
    let e = ev(
        &s,
        "max(temp_c, today) >= 10 and sum(precip_mm, next 3h) >= 3",
        now,
    );
    assert!(e.matches);
    assert_eq!(e.field, Some(Field::TempC));
    let e = ev(
        &s,
        "max(temp_c, today) >= 10 and sum(precip_mm, next 3h) >= 4",
        now,
    );
    assert!(!e.matches);
}

#[test]
fn field_mapping_uses_the_right_series() {
    let s = base()
        .at("wind_speed_10m", 0, Some(33.0))
        .at("wind_gusts_10m", 0, Some(77.0))
        .at("apparent_temperature", 0, Some(-9.0))
        .at("snowfall", 0, Some(2.5))
        .at("uv_index", 0, Some(8.0))
        .at("precipitation_probability", 0, Some(90.0));
    let now = utc(2026, 10, 24, 10, 0);
    for (c, v) in [
        ("max(wind_kmh, next 1h) > 0", 33.0),
        ("max(gust_kmh, next 1h) > 0", 77.0),
        ("min(feels_c, next 1h) < 0", -9.0),
        ("sum(snow_cm, next 1h) > 0", 2.5),
        ("max(uv, next 1h) > 0", 8.0),
        ("max(precip_prob, next 1h) > 0", 90.0),
        ("max(code, next 1h) > 0", 1.0),
    ] {
        assert_eq!(ev(&s, c, now).value, Some(v), "{c}");
    }
}

// ---- units and messages ----

#[test]
fn unit_formatting() {
    use Field::*;
    let m = Units::Metric;
    let i = Units::Imperial;
    assert_eq!(format_value(TempC, -2.4, m), "-2°C");
    assert_eq!(format_value(TempC, 0.0, i), "32°F");
    assert_eq!(format_value(TempC, -0.2, m), "0°C");
    assert_eq!(format_value(FeelsC, 34.0, i), "93°F");
    assert_eq!(format_value(PrecipMm, 16.25, m), "16.3 mm");
    assert_eq!(format_value(PrecipMm, 25.4, i), "1 in");
    assert_eq!(format_value(SnowCm, 2.54, i), "1 in");
    assert_eq!(format_value(SnowCm, 3.0, m), "3 cm");
    assert_eq!(format_value(GustKmh, 64.0, m), "64 km/h");
    assert_eq!(format_value(GustKmh, 80.4672, i), "50 mph");
    assert_eq!(format_value(WindKmh, 16.09344, i), "10 mph");
    assert_eq!(format_value(PrecipProb, 70.0, m), "70");
    assert_eq!(format_value(Uv, 8.25, m), "8.3");
    assert_eq!(format_value(Code, 95.0, i), "95");
}

#[test]
fn message_templates() {
    let s = base().set("precipitation", |i| Some(if i < 12 { 1.5 } else { 0.0 }));
    let now = utc(2026, 10, 24, 10, 0);
    let e = ev(&s, "sum(precip_mm, next 12h) >= 15", now);
    assert_eq!(
        render_message(
            "Heavy rain: {value} in the {when} ({field})",
            &e,
            Units::Metric
        ),
        "Heavy rain: 18 mm in the next 12 h (precipitation)"
    );
    assert_eq!(render_message("{value}", &e, Units::Imperial), "0.71 in");
    let t = ev(&base(), "min(temp_c, tonight) <= 99", now);
    assert_eq!(
        render_message("Frost {when}, low {value}", &t, Units::Metric),
        "Frost tonight, low 10°C"
    );
}
