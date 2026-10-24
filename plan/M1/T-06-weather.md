---
id: T-06
title: "fa-weather: fetch, DSL, evaluate, advisories, defaults, jobs"
milestone: M1
wave: 3
depends_on: [T-02, T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-weather/**"]
---

# T-06: fa-weather: fetch, DSL, evaluate, advisories, defaults, jobs

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §4 and §4.1, §6
- plan/M1.md: *Jobs*, the weather and weather-rules rows of the API table, *Process config* (`FA_OPEN_METEO_URL`)
- crates/fa-connectors `openmeteo` (T-02), fa-store repos (T-04: `WeatherSnapshotRepo`, `WeatherRuleRepo`, `AdvisoryRepo`, `HouseholdRepo`, `ChangeBus`), crates/fa-jobs

## You own
- `crates/fa-weather/**` (W0 created the empty crate)

## Work
1. **DSL** (`dsl/`: a hand-written lexer and parser, no parser-generator crate). The grammar is exactly Feeds.md §4.1:
   ```
   rule      := cond ( ("and" | "or") cond )*
   cond      := agg "(" field "," window ")" op number
   agg       := min | max | sum | avg | any
   field     := temp_c | feels_c | precip_prob | precip_mm | snow_cm | gust_kmh | wind_kmh | uv | code
   window    := "next" INT "h" | today | tonight | tomorrow
   op        := <= | < | >= | > | == | !=
   ```
   - `and` binds tighter than `or`, and there are no parentheses.
   - Keywords are case-insensitive and whitespace-tolerant.
   - `next Nh` takes N in 1..=48 (the hourly horizon).
   - `parse(&str) -> Result<Rule, ParseError{position (char offset), message}>`. The messages are human-readable ("expected a field such as temp_c at 5").
   - `Rule::to_string()` round-trips to the canonical form.
2. **Field mapping** to Open-Meteo hourly series:
   - `temp_c` = temperature_2m, `feels_c` = apparent_temperature, `precip_prob` = precipitation_probability, `precip_mm` = precipitation, `snow_cm` = snowfall, `gust_kmh` = wind_gusts_10m, `uv` = uv_index, `code` = weather_code.
   - `wind_kmh` = hourly **wind_speed_10m**.
3. **Evaluator:** `evaluate(rule, snapshot, tz, now) -> Evaluation{matches, value: Option<f64>, when: Option<(start, end)>}`.
   - **Windows** (household tz): `next Nh` = [now, now + N h); `today` = [now, next midnight); `tonight` = [18:00 today, 08:00 tomorrow) (if now is already past 18:00, it starts at now); `tomorrow` = [next midnight, +24 h).
   - **Aggregates:** `min`, `max`, `sum`, `avg` over the non-null hourly values in the window. `any(f, w) op n` is true if any hour satisfies the comparison.
   - **Combining:** for `and`/`or`, `value` and `when` come from the first matching condition.
   - An empty window evaluates to false with no value.
4. **Message templates:** `{value}` (formatted in household units, using a small Rust conversion that matches T-03's TS units: °C/°F, mm/in, cm/in, km/h → mph), `{when}` (for example "tonight", "next 12 h", "today"), `{field}`.
5. **`WeatherService`:**
   - `fetch(household)`: skips without a location. It calls Open-Meteo through `fa-connectors` (base from config), stores the snapshot, enqueues `weather.evaluate`, and publishes `weather.updated`. On failure, it sets `system_status['weather'] = {ok: false, error, at}` and returns Err so the job retries; on success, `ok: true`.
   - `evaluate(household)`: runs every `active` alert rule. A match goes to `AdvisoryRepo::upsert_by_dedupe` with the key `rule:{id}:{window start date in household tz}`, `valid_from` and `valid_until` = the window, and source `rule`. Rules with `action = task` are skipped in M1 (log at debug). If anything changed, publish `advisory.updated`.
   - `validate(condition, message?) -> {ok, error?, preview?}` against the latest snapshot (used by `POST /weather-rules/validate`).
   - `retention(household)`: deletes snapshots older than `weather_retention_days` and advisories expired more than 7 days ago.
   - `latest_view(household) -> Option<WeatherView>`: hourly trimmed to the next 48 h, daily to 7 days, and sunrise/sunset for today.
6. **Default rules:** `ensure_default_rules(conn)` uses `WeatherRuleRepo::ensure_default` for each row (all `action = alert`, `status = active`, `suggested_by = default`):

   | Name | Condition | Severity | Message |
   |---|---|---|---|
   | Frost tonight | `min(temp_c, tonight) <= 0` | warning | Frost tonight, low of {value} |
   | Heavy rain | `sum(precip_mm, next 12h) >= 15` | warning | Heavy rain: {value} in the {when} |
   | Likely rain today | `max(precip_prob, today) >= 70` | info | Rain likely today ({value}%) |
   | Snow | `sum(snow_cm, next 24h) >= 2` | warning | Snow expected: {value} in the {when} |
   | Strong wind | `max(gust_kmh, next 12h) >= 60` | warning | Gusts up to {value} in the {when} |
   | Heat | `max(feels_c, today) >= 32` | warning | Hot today, feels like {value} |
   | High UV | `max(uv, today) >= 8` | info | High UV today ({value}) |
   | Thunderstorm | `any(code, today) >= 95` | severe | Thunderstorms possible today |
7. **Jobs** (`handlers()` + `default_schedules()`): `weather.fetch` (`*/30 * * * *`, all households with a location), `weather.evaluate{household}`, `weather.retention` (`45 3 * * *`).
8. **Tests:**
   - **Parser:** every grammar production, precedence (`a or b and c`), error positions for at least 8 malformed inputs, a canonical round trip, and every default rule parses.
   - **Evaluator** on a synthetic snapshot in a fixed tz: each window (including `tonight` before and after 18:00, and across midnight and DST), each aggregate, `any`, null gaps, and an empty window.
   - **Units formatting.**
   - **Service** against a test DB with a wiremock Open-Meteo: fetch → snapshot → evaluate → advisory; a second evaluate doesn't duplicate; a dismissed advisory isn't resurrected; failure sets the status to not ok; retention deletes old snapshots; `ensure_default_rules` is idempotent.

## Acceptance
- [ ] `cargo test -p fa-weather` passes (no live network)
- [ ] `cargo clippy -p fa-weather --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` is clean
- [ ] New store queries needed → listed in your report (no SQL in this crate)

## Wave 1 outcomes (read before starting)
- **fa-connectors (T-02):** `openmeteo::forecast(...)` returns a `Forecast` with the raw `current`/`hourly`/`daily` JSON (store these), plus `timezone` and `utc_offset_seconds`. Accessors: `current_f64`, `hourly_series(field)` (UTC datetimes), `daily_series(field)` (`NaiveDate`), and `daily_times("sunrise"|"sunset")`. The request pins 48 hourly hours, 7 days, and metric units.
- fa-domain (T-01) is done. Read its weather enums and DTOs in `crates/fa-domain/src` (`feeds.rs`, `api.rs`).

## Wave 2 outcomes (fa-store, read before starting)
- Read the real signatures in `crates/fa-store/src/repo/*.rs` and `src/change.rs`. Don't assume them from the brief.
- **Transactions:** `WeatherSnapshotRepo::insert`, `WeatherRuleRepo::ensure_default`, and `AdvisoryRepo::upsert_by_dedupe` take the household from the transaction, so they must run in `tx_for`. `CalendarSourceRepo::due()` returns `(HouseholdId, CalendarSourceId)` pairs and must run in `tx_system`.
- `ChangeBus::publish` rejects payloads over 1 KB (`StoreError::PayloadTooLarge`). Exports: `fa_store::{CHANNEL, ChangeMessage}`.
- **If a repo method you need is missing:** add it in a **new file** `crates/fa-store/src/repo/ext_<your task id>.rs` (plus a one-line `mod`/`pub use` in `repo/mod.rs`), using the dev DB (migrated) for the sqlx macros. **Don't run `cargo sqlx prepare`**; the main session does that after the wave. List every such addition in your report. Don't edit other fa-store files.
- `AdvisoryRepo::active` orders by severity. `WeatherRuleRepo::update(conn, &WeatherRule, version)` takes the whole rule.

## Ground rules (all M1 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M1.md` (especially *Contracts fixed up front*), and the arch sections listed under *Read first*. The contracts in `plan/M1.md` are the source of truth. If one is wrong or missing, don't invent a new one: work around it minimally and report it.
- **Edit only the paths you own.** The root `Cargo.toml` belongs to the main session. Add dependencies to your own crate only as `workspace = true` entries (pre-declared in W0). If one is missing, report it.
- **Disk:** run `df -h /` before building. If less than 6 GB is free, stop and report.
- External names: verify crate features with `cargo add --dry-run <crate> --features …` before you rely on them. Never guess API paths. Use arch/Feeds.md or the vendor docs.
- Code style: idiomatic Rust 2024, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs (never log secrets, tokens, or full feed URLs). Keep comments sparse. Match the surrounding code (for example the `XxxRepo` associated functions taking `&mut PgConnection` and returning `Versioned<T>`).
- Always run `cargo fmt` (and `pnpm format` for web) before reporting.
- No hand-written API types in the web app: `apps/web/src/gen` is generated by the main session.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com` and fictional names.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure; quote counts only from actual output);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups or requests for other tasks' paths.
