---
id: T-08
title: "fa-config: process config keys, TOML sections, round trip"
milestone: M1
wave: 3
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-config/**", "config/examples/**"]
---

# T-08: fa-config: process config keys, TOML sections, round trip

You are a sub-agent implementing one task of the M1 milestone (Dashboard core) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §2 (household columns, sources, calendars, rules) and §5 (Configuration)
- arch/Platform.md §6
- plan/M1.md: *Process config* and *Household TOML additions*
- crates/fa-config (M0), and the fa-store M1 repos (from T-04)

## You own
- `crates/fa-config/**`
- `config/examples/**`

## Work
1. **Process config** (figment, env + file): add `google_client_id: Option<String>` (`FA_GOOGLE_CLIENT_ID`), `google_client_secret: Option<Secret-ish String>` (`FA_GOOGLE_CLIENT_SECRET`, redacted in `Debug`), `public_url: Option<Url>` (`FA_PUBLIC_URL`), `open_meteo_url: Url` (`FA_OPEN_METEO_URL`, default `https://api.open-meteo.com`), and `allow_http_feeds: bool` (`FA_ALLOW_HTTP_FEEDS`, default false). Validation: when a Google client ID is set, `public_url` is required and must be https (or http when `allow_http_feeds` is on, for dev).
2. **Household TOML schema:**
   - `[household]` gains `units`, `calendar_past_days`, `calendar_future_days`, `weather_retention_days`, and `private_events_as_busy`. Each is optional, with the defaults from Feeds.md §2.
   - New `[[calendar_sources]]` table: `{key, kind, name, owner? (member key), url? (caldav only), username?, poll_minutes?, enabled?}` with `[[calendar_sources.calendars]] {remote_id, name?, enabled, color?, members = [member keys]}`.
   - New `[[weather_rules]]` table: `{name, condition, action, severity, message, status}`.
3. **Export** writes the new sections **without secrets**:
   - Google sources become `kind = "google"` with no account, and a comment saying they need reconnecting.
   - CalDAV sources are written without the password.
   - ICS sources are written without the URL.

   Weather rules with `suggested_by = default` that are unchanged are omitted, because `fa_weather::ensure_default_rules` seeds them.
4. **Import:**
   - Upserts by key and name.
   - Sources arrive with `state = needs_reauth` and their calendars disabled, unless the source already exists with credentials (in which case its credentials are kept).
   - Calendar member links resolve by member key, and an unknown key is an error that names the line.
   - Don't depend on fa-weather. Store weather-rule conditions as given; `fa-server config import` (T-09) validates them with fa-weather's parser before writing.
   - Un-archive archived members found in the file, using `MemberRepo::unarchive` (M0 follow-up).
5. Update `config/examples/household.example.toml`:
   - `units = "metric"`;
   - one disabled ICS source on a placeholder URL (`https://calendar.example.com/school.ics`), with one calendar linked to a child member;
   - one custom weather rule.

   It must still pass `scripts/check-example-data.sh`.
6. **Tests:**
   - an export → import → export round trip is byte-stable, using an insta snapshot;
   - secrets never appear in an export;
   - an unknown member key gives an error that points to the line;
   - process config validation covers its cases.

## Acceptance
- [ ] `cargo test -p fa-config` passes
- [ ] `cargo clippy -p fa-config -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `scripts/check-example-data.sh` passes

## Wave 1 outcomes (read before starting)
- The main session put **interim defaults** for the new `Household` fields in `crates/fa-config/src/household.rs` (the import path). Replace them with values read from the TOML.
- The M0 provenance enum was renamed `SourceKind` → `RefKind`.

## Wave 2 outcomes (fa-store, read before starting)
- Read the real signatures in `crates/fa-store/src/repo/*.rs` and `src/change.rs`. Don't assume them from the brief.
- **Transactions:** `WeatherSnapshotRepo::insert`, `WeatherRuleRepo::ensure_default`, and `AdvisoryRepo::upsert_by_dedupe` take the household from the transaction, so they must run in `tx_for`. `CalendarSourceRepo::due()` returns `(HouseholdId, CalendarSourceId)` pairs and must run in `tx_system`.
- `ChangeBus::publish` rejects payloads over 1 KB (`StoreError::PayloadTooLarge`). Exports: `fa_store::{CHANNEL, ChangeMessage}`.
- **If a repo method you need is missing:** add it in a **new file** `crates/fa-store/src/repo/ext_<your task id>.rs` (plus a one-line `mod`/`pub use` in `repo/mod.rs`), using the dev DB (migrated) for the sqlx macros. **Don't run `cargo sqlx prepare`**; the main session does that after the wave. List every such addition in your report. Don't edit other fa-store files.
- `HouseholdRepo` has the real M1 columns plus `update_settings`. `MemberRepo::unarchive` and `list_including_archived` exist. `CalendarSourceRepo::set_enabled` exists, which you need for importing sources as disabled with `needs_reauth`.

## Ground rules (all M1 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M1.md` (especially *Contracts fixed up front*), and the arch sections listed under *Read first*. The contracts in `plan/M1.md` are the source of truth. If one is wrong or missing, don't invent a new one: report it.
- **Edit only the paths you own.** If you need a change somewhere else, don't make it: say so in your report.
- Run `df -h /` before building. **If less than 6 GB is free, stop and report.**
- Rust: idiomatic 2024 edition, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs. Run `cargo fmt` before you finish. Keep comments sparse and useful.
- Web: TypeScript strict. **API types come from `apps/web/src/gen/` only. Never hand-write API types.** Run prettier and eslint (`corepack pnpm@9 lint`, `corepack pnpm@9 format` or the equivalent scripts in package.json) before you finish.
- **Verify every external name** before using it: crate features with `cargo add --dry-run`, images with `docker manifest inspect`, and API paths against the vendor docs. Don't guess.
- CI-equivalent checks run against a **fresh** Postgres (`scripts/ci-local.sh`, `SQLX_OFFLINE=true`), not only the dev DB.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com`.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **README:** only T-12 owns `README.md`. Every other task lists the README changes it needs in its report.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups, requests for other tasks' paths, and README changes needed.
