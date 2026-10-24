---
id: T-02
title: "fa-connectors: Google, CalDAV, ICS, Open-Meteo clients"
milestone: M1
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-connectors/**"]
---

# T-02: fa-connectors: Google, CalDAV, ICS, Open-Meteo clients

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §1, §3.1–3.3, §4 (fields used), §7 (fetch limits)
- plan/M1.md: *Crates*, *Process config*

## You own
- `crates/fa-connectors/**` (W0 created the empty crate)

## Work
This is a **leaf crate**: no `fa-*` dependencies and no DB. It returns neutral DTOs defined here. Every client takes a base URL, so tests and stubs can point it elsewhere.

1. **`http`:** a shared `reqwest` client builder:
   - rustls, 20 s timeout, gzip;
   - a body cap of 10 MB, enforced while streaming;
   - an `https`-only check unless `allow_http` is set;
   - a user agent of `family-assistant/<version>`.

   Errors are a `ConnectorError` enum: `Unauthorized` (401, or Google `invalid_grant`), `Gone` (410), `NotModified`, `TooLarge`, `Http(status)`, `Parse`, `Network`, `InsecureUrl`. Logs never contain query strings or credentials, only the host.
2. **`google`:**
   - **`oauth`:** `authorize_url(client, redirect, scopes, state, pkce_challenge)` (S256, `access_type=offline`, `prompt=consent`), `exchange_code(...) -> TokenSet{access, refresh, expires_at, scopes, id_token_email}`, `refresh(refresh_token) -> TokenSet`. Use the `oauth2` crate. Token and auth endpoints are configurable (defaults `https://oauth2.googleapis.com/token`, `https://accounts.google.com/o/oauth2/v2/auth`).
   - **`calendar`:**
     - `list_calendars(access) -> Vec<RemoteCalendar{remote_id, name, color?}>` (`GET /calendar/v3/users/me/calendarList`, paginated).
     - `list_events(access, cal_id, Window | SyncToken) -> EventPage{items, next_sync_token}`, with `singleEvents=true`, `timeMin`/`timeMax` on a full sync, and `syncToken` on an incremental one. Follow `pageToken`.
     - Map items into `RemoteEvent` (below), including `status=cancelled`, `visibility=private`, all-day `date` vs `dateTime`, `htmlLink`, and `recurringEventId` + `originalStartTime` → `instance_key`.
3. **`caldav`:**
   - `discover(base, user, pass)`: try `/.well-known/caldav`, then PROPFIND `current-user-principal` → `calendar-home-set` → Depth 1 listing of collections with `resourcetype calendar`. Returns `RemoteCalendar{remote_id = href, name, color?, ctag?}`.
   - `fetch(cal, window) -> Vec<String>` (raw VCALENDAR blobs): a `calendar-query` REPORT with a `time-range` filter, plus `getctag` to allow skipping.
   - Use `quick-xml` for the XML.
4. **`ics`:**
   - `fetch(url, etag?, last_modified?) -> Fetched{body, etag?, last_modified?} | NotModified`. `webcal://` becomes `https://`.
   - `parse(body) -> Vec<ParsedComponent>`, using `icalendar`'s parser and keeping UID, RECURRENCE-ID, DTSTART/DTEND/DURATION with TZID or VALUE=DATE, RRULE, EXDATE, RDATE, SUMMARY, LOCATION, DESCRIPTION, CLASS, STATUS, and URL.
   - **Expansion is not here** (that is T-05). Keep the raw rule strings and TZIDs.
5. **`openmeteo`:** `forecast(base, lat, lon, tz) -> Forecast{current, hourly, daily}` requesting exactly the fields in Feeds.md §4 (metric, 48 h hourly, 7 days daily, `timezone=<tz>`). Keep the raw JSON for each section (stored as jsonb later) **and** typed accessors (hourly series by field name, as `Vec<(DateTime, Option<f64>)>`).
6. **DTOs:**
   - `RemoteEvent{uid, instance_key, title, description?, location?, start: EventTime, end: EventTime, private, cancelled, external_url?, etag?}`, where `EventTime = DateTime(Utc) | Date(NaiveDate)`.
   - `RemoteCalendar`, `TokenSet`, `Forecast`, `ParsedComponent`.
7. **Tests (wiremock only, no live network):**
   - every client's happy path;
   - pagination;
   - Google 401 → `Unauthorized`, 410 → `Gone`, `invalid_grant` → `Unauthorized`;
   - ICS 304;
   - the body cap;
   - `http://` rejected unless allowed;
   - a PROPFIND/REPORT XML round trip with Radicale-shaped responses (recorded fixture files under `tests/fixtures/`, fictional data);
   - Open-Meteo parsing from a recorded fixture response (built from a real response shape with fictional coordinates).

## Acceptance
- [ ] `cargo test -p fa-connectors` passes, with no network access needed
- [ ] `cargo clippy -p fa-connectors --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` is clean
- [ ] `cargo tree -p fa-connectors | grep -c ' fa-'` shows no internal dependencies

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
