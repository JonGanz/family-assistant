---
id: T-09
title: "fa-api + fa-server: M1 routes, PgListener bridge, OpenAPI, jobs, CLI"
milestone: M1
wave: 4
depends_on: [T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-api/**", "bins/fa-server/**"]
---

# T-09: fa-api + fa-server: M1 routes, PgListener bridge, OpenAPI, jobs, CLI

You are a sub-agent implementing one task of the M1 milestone (Dashboard core) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- plan/M1.md: **HTTP API v1 additions (the whole table)**, Jobs, and Process config
- arch/Feeds.md §3.5 (visibility), §4 (weather API shape), §6 (ChangeBus and SSE kinds), §7
- arch/Frontend.md §4.1 (`DashboardSettings`) and §4.2 (`/dashboard`)
- arch/Platform.md §3 to §5
- crates/fa-api and bins/fa-server (M0), plus the public APIs of fa-calendar, fa-weather, fa-auth (OAuthService), fa-config, and fa-store (ChangeBus)

## You own
- `crates/fa-api/**`
- `bins/fa-server/**`

## Work
1. **Routes:** implement every row of the plan's *HTTP API v1 additions* table, with exactly those paths, actors, DTOs (from fa-domain), and status codes. Handlers stay thin: they authorise through `Policy::authorize` with the M1 actions, call the feature crate, and map errors to problem+json.
   - **Visibility:** `/events` and `/dashboard` use the feature crate's masked queries, which call `fa_domain::calendar::view()`. Never return an event outside the viewer's scope, or a busy event's title, location, or description.
   - **Avatars:** enforce the 512 KB limit before buffering (`DefaultBodyLimit` on that route), sniff the type from the magic bytes rather than trusting `Content-Type`, and return 413 or 415.
   - **OAuth:** `start` redirects with 302. `callback` redirects to `/oauth/done?account=…&return_to=…` or `?error=…`, never to an arbitrary URL: `return_to` must be a same-origin path.
   - **Sources:** `POST /calendar-sources` and `/test` discover synchronously with a 20 s timeout. Responses never contain secrets: show only `masked_url` (scheme://host/…).
   - **Weather rules:** an invalid DSL returns 422 with `{position, message}`.
   - **Mutations** that publish SSE kinds do so through `ChangeBus::publish` inside the same transaction. Never push to the Hub directly.
2. **PgListener → Hub bridge:**
   - A task started by `serve api` (and by `serve all`) `LISTEN`s on `fa_change`.
   - It parses `{household, kind, data}` and pushes each message into the `Hub`.
   - On disconnect, it reconnects with exponential backoff (max 30 s), then broadcasts `resync` to every household.
   - Malformed payloads are logged and dropped.
   - **Test:** a NOTIFY sent from another connection reaches a subscribed SSE client.
3. **OpenAPI:**
   - Derive the schemas from the fa-domain DTOs (`ToSchema`). **Delete `crates/fa-api/src/schemas.rs`** (the hand-written mirrors).
   - `MemberView` is the public | full union.
   - Write the spec to `apps/web/src/gen/openapi.json` **only via the existing `fa-server` openapi command / `just gen` path**. The main session runs the generation after this wave, so don't edit `apps/web` yourself.
4. **Jobs in fa-server:**
   - Register `handlers()` from fa-calendar, fa-weather, and fa-auth.
   - Call each crate's `default_schedules()` through the existing ensure-schedules code.
   - On startup, call `fa_weather::ensure_default_rules` for each household.
   - Pass process config (Open-Meteo URL, allow-http, Google client, public URL) into the crates' constructors.
   - `config import`: validate every weather-rule condition with `fa_weather`'s parser before writing, and fail with the rule name and error position.
   - Fill `avatar_url` on `MemberPublic`/`MemberFull` (`/api/v1/members/{id}/avatar?v=<sha256 prefix>`, null without an avatar).
   - `HouseholdView.google_oauth_enabled` is true when the Google client ID and secret are configured.
5. **CLI additions:**
   - `fa-server calendar sync <source-id>` runs the sync inline and prints a summary (calendars, upserted, deleted).
   - `fa-server weather fetch` fetches and evaluates inline and prints the current conditions and any advisories fired.
   - `user add` and `device pair` switch to the System-capable fa-auth services from T-07, and the duplicated token logic is removed.
6. **Tests** (sqlx::test + tower `oneshot`):
   - an RBAC matrix over the new routes (adult, child, device, elevated device, anonymous);
   - `/events` masking for a child and for the device for adult-owned and child-owned private events, with `private_events_as_busy` on and off;
   - avatar 413 and 415;
   - OAuth `return_to` open-redirect rejection;
   - the shape of `/dashboard`;
   - weather-rule 422;
   - the bridge test from step 2.

## Acceptance
- [ ] `cargo test -p fa-api -p fa-server` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `crates/fa-api/src/schemas.rs` is gone, and the generated `openapi.json` contains every path in the plan's table (report the path count, computed from the output)
- [ ] Manual check: run `serve api` and `serve worker` as **separate processes** against the dev DB. `fa-server weather fetch` (with `FA_OPEN_METEO_URL` pointing at a local stub or the real API) leads to a `weather.updated` event on `curl -N /api/v1/stream` with a device token
- [ ] `scripts/ci-local.sh` passes

## Wave 1 outcomes (read before starting)
- fa-domain DTOs derive `ToSchema`. The M0 DTOs keep their `*Dto` names, with `MemberPublic`/`MemberFull` as aliases and `MemberView` untagged. Replace `crates/fa-api/src/schemas.rs` with the real types. `From<&Member>` leaves `avatar_url: None`, and you fill it in.

## Wave 2 outcomes (fa-store, read before starting)
- Read the real signatures in `crates/fa-store/src/repo/*.rs` and `src/change.rs`. Don't assume them from the brief.
- **Transactions:** `WeatherSnapshotRepo::insert`, `WeatherRuleRepo::ensure_default`, and `AdvisoryRepo::upsert_by_dedupe` take the household from the transaction, so they must run in `tx_for`. `CalendarSourceRepo::due()` returns `(HouseholdId, CalendarSourceId)` pairs and must run in `tx_system`.
- `ChangeBus::publish` rejects payloads over 1 KB (`StoreError::PayloadTooLarge`). Exports: `fa_store::{CHANNEL, ChangeMessage}`.
- **If a repo method you need is missing:** add it in a **new file** `crates/fa-store/src/repo/ext_<your task id>.rs` (plus a one-line `mod`/`pub use` in `repo/mod.rs`), using the dev DB (migrated) for the sqlx macros. **Don't run `cargo sqlx prepare`**; the main session does that after the wave. List every such addition in your report. Don't edit other fa-store files.
- `JobRepo::counts` → `JobCounts{ready, running, dead}` for `StatusView`. `DeviceRepo::get_settings` and `set_settings`, `MemberAvatarRepo::{put, get, hashes, delete}` (`hashes` fills `avatar_url`).

## Wave 3 outcomes (read before starting)
- **fa-auth (T-07):**
  - Build `OAuthService::new(store, MasterKey, Option<GoogleOAuth>, public_url, Http)` from **fa-config process config**. Don't use `OAuthService::from_env`, which reads env vars directly against the injection rule. Delete it if nothing else uses it.
  - `enabled()` feeds `HouseholdView.google_oauth_enabled`.
  - `start(actor, session_id, return_to)` and `callback(actor, session_id, code, state) -> (OAuthAccountId, Option<return_to>)` need the **session id from the cookie**. Map `AuthError::InvalidState`/`OAuth` to a redirect to `/oauth/done?error=…`, `NotConfigured` to 404, and `NeedsReauth` to 409.
  - Register `fa_auth::handlers()` (a free fn) and `fa_auth::default_schedules()`.
  - **fa-server `ops.rs`:** replace the CLI's own device-pairing, token-hash, and account create/reset code with `DeviceService::new(store, public_base_url).pair_system(...)` and `AccountService::new(store).{create_account, reset_password}` (which accept `Actor::System`). Drop the duplicated password validation.

- **fa-config (T-08):**
  - The process config has `google_client_id`, `google_client_secret` (`SecretString`), `public_url`, `open_meteo_url`, and `allow_http_feeds`, already validated.
  - `config import` must validate `[[weather_rules]]` conditions with fa-weather's parser **before** calling fa-config's import, then call `ensure_default_rules`. Print the new `ImportReport` `sources_*`/`rules_*` lists.
  - Imported sources come in disabled with `needs_reauth`, and an ICS calendar has `remote_id = "ics"`.

- **fa-weather (T-06):**
  - `WeatherService::new(store, WeatherConfig{open_meteo_url, allow_http})`. `handlers(WeatherConfig)` returns a `Result`. `default_schedules()` and `ensure_default_rules(&mut conn)` run in `tx_for`.
  - `validate(household, condition, message?)` returns `Validation{ok, error?: ParseError{position, message}, preview?}`: map it to the `/weather-rules/validate` response, and to 422 on create or update.
  - `latest_view(household)` gives `WeatherView` with **column-oriented** `hourly`/`daily` (Open-Meteo field names, 48 h / 7 days).

- **fa-calendar (T-05):**
  - `CalendarService::new(...)`, where the service methods take `household` first: `test_source` (never errors, returns `SourceTestResult`), `create_source`, `update_source`, `archive_source`, `enqueue_sync`, `update_calendar`, `list_sources`, `list_calendars`, and `events(household, viewer, flags, from, to, member)`.
  - Map `CalendarError::Unauthorized` to 422, with a "credentials rejected" message.
  - Register `fa_calendar::handlers(Arc<CalendarService>)` and `default_schedules()`. The `calendar.sync` payload is `{household, source}`.
  - `EventView.external_url` is always `None` for now (main-session follow-up).
- **`AuthError` mapping:** the main session added an interim mapping in `crates/fa-api/src/error.rs` (`NotConfigured`→404, `NeedsReauth`→409, `InvalidState`/`OAuth`→422). Replace it with the redirect behaviour above.

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
