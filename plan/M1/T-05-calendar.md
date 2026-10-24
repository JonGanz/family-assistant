---
id: T-05
title: "fa-calendar: discovery, sync, expansion, queries, jobs"
milestone: M1
wave: 3
depends_on: [T-02, T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-calendar/**"]
---

# T-05: fa-calendar: discovery, sync, expansion, queries, jobs

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §2–3 (all of it), §6, §7
- plan/M1.md: *Jobs*, *HTTP API v1 additions* (the events and calendar-source rows), *Risks*
- crates/fa-connectors (T-02), crates/fa-store repos (T-04: `CalendarSourceRepo`, `CalendarRepo`, `EventRepo`, `ChangeBus`, `SecretRepo`), `fa_domain::calendar::view` (T-01), crates/fa-jobs (`JobHandler`)

## You own
- `crates/fa-calendar/**` (W0 created the empty crate)

## Work
1. **`CalendarService`**, constructed with `Store`, `MasterKey`, `ConnectorConfig{allow_http, google endpoints}`, and an `Arc<dyn fa_domain::AccessTokenProvider>` for Google access tokens (the trait is defined by T-01, implemented by T-07). `TokenError::NeedsReauth` sets the source to `needs_reauth`.
   - Define the trait here: `async fn access_token(&self, account: OAuthAccountId) -> Result<String, TokenError{Reauth|Other}>`. T-07 implements it, and tests use a fake.
   - Methods used by T-09:
     - `test_source(NewSource) -> TestResult{calendars, sample_events (≤ 3), error?}` (no writes);
     - `create_source(actor_household, owner, NewSource) -> (SourceView, Vec<CalendarView>)`: stores the credential as a Secret (the CalDAV password, or the **whole ICS URL**), discovers synchronously, and inserts the calendars disabled (an ICS source gets exactly one calendar, enabled);
     - `update_source` (a credential change resets the state to `ok` and enqueues a sync), `archive_source`, `enqueue_sync(source)`;
     - `update_calendar(patch)` (members, colour, enabled), which publishes `calendar.updated` over the full window;
     - `events(viewer, flags, from, to, member: Option) -> Vec<EventView>`: `EventRepo::range` + `view()` + mask (D-036); the `member` filter keeps events linked to that member; a child viewer only gets their own and household calendars; the range is capped at 400 days (error above).
2. **Sync** (`sync_source(source_id)`), following Feeds.md §3.3 exactly:
   1. Discover: upsert calendars (new ones disabled) and archive the ones that went missing.
   2. For each enabled calendar, fetch with the right mode:
      - **Google:** a full sync with a window or an incremental one with `syncToken`. On `Gone`, clear the token, run `delete_calendar_events`, and do a full sync.
      - **CalDAV:** skip when the ctag is unchanged.
      - **ICS:** skip on 304.
   3. Expand locally (CalDAV/ICS) with `rrule` in the event's TZID:
      - apply EXDATE and RDATE;
      - a RECURRENCE-ID override replaces its occurrence;
      - a `STATUS:CANCELLED` override removes it;
      - `instance_key` = the RECURRENCE-ID as UTC ISO (`''` for non-recurring);
      - expansion is clipped to `[now − past_days, now + future_days]` from the household.
   4. Timed events become UTC instants. **All-day** events keep `start_date`/`end_date` (end exclusive; a missing DTEND means 1 day) and also get `start_at`/`end_at` computed at midnight in the household tz.
   5. Unknown TZIDs: map Windows names to IANA (a static table in `tz_windows.rs` covering at least the 50 most common Outlook zones). Otherwise use the household tz and a warning recorded in the source's `last_error` (the state stays `ok`).
   6. Floating times use the household tz.
   7. In one transaction per calendar: upsert, `delete_unseen_in_window` (on full fetches), save `sync_state`, then `ChangeBus::publish("calendar.updated", {from, to})`.
   8. Errors: `Unauthorized`/`TokenError::Reauth` → `needs_reauth` (and the oauth_account too, through the provider's error). Others → `error` + `last_error` (scrubbed of URLs and credentials), and return Err so the job retries. Always publish `source.updated{id, state}`.
3. **Jobs** (`handlers()` + `default_schedules()`, per the plan's Jobs table):
   - `calendar.sync_due`: `CalendarSourceRepo::due`, then enqueue `calendar.sync{source}` with the key `calendar.sync:{source}:{minute slot}`.
   - `calendar.sync{source}`
   - `calendar.prune` (`prune_before(now − past_days)` per household)
4. **Fixture corpus** (`tests/fixtures/`, all fictional, `example.com`):
   - the RFC 5545 §3.8.5.3 RRULE examples, as many as `rrule` supports (daily COUNT, weekly BYDAY, monthly BYMONTHDAY, yearly);
   - a Google-style export with an overridden instance;
   - an Outlook export with a Windows TZID (e.g. "Eastern Standard Time") and an EXDATE;
   - a fictional school feed: all-day multi-day events, a floating time, and a CANCELLED override;
   - a sports-app-style feed with no DTEND, using DURATION;
   - a feed with an unknown TZID.
5. **Tests:**
   - expansion: count, EXDATE, RECURRENCE-ID move, cancelled override, COUNT + EXDATE, a weekly event across the **DST change** (local wall time stays fixed, UTC shifts), an all-day event spanning a DST change, and window clipping;
   - Windows TZID mapping and the unknown-TZID fallback warning;
   - against a test DB with wiremock-served sources (reuse the T-02 fixture style):
     - an ICS full sync, then a 304 skip, then a changed feed removing an event (`delete_unseen`);
     - CalDAV ctag skip;
     - a Google incremental sync with a cancelled item, then `410` → full resync;
     - 401 → `needs_reauth`;
   - the `due()` → enqueue idempotency;
   - `events()` applying `view()` for an adult, a child (own + household calendars only), and a device (an adult-owned private event is busy, a child-owned private event is full, and with the flag off everything is full);
   - `calendar.updated` published on commit.

## Acceptance
- [ ] `cargo test -p fa-calendar` passes (a DB is needed for the sqlx tests; no live network)
- [ ] `cargo clippy -p fa-calendar --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` is clean
- [ ] If you needed new store queries: list them in your report (T-04 owns `fa-store`; don't put SQL in this crate)

## Wave 1 outcomes (read before starting)
- **fa-connectors API (T-02):**
  - Every function takes `&fa_connectors::Http` and a base URL.
  - Google: `google::calendar::{list_calendars, list_events}` with `EventQuery::{Window, SyncToken}`.
  - CalDAV: `CaldavClient::new(http, base, user, pass)` with `.discover()`, `.ctag(&cal)`, and `.fetch(&cal, &Window)`.
  - ICS: `ics::fetch` → `IcsFetch::{Fetched, NotModified}`, and `ics::parse` gives raw `IcsTime{value, tzid, is_date}` plus the raw RRULE, EXDATE, and RDATE strings. You do the TZID resolution and expansion.
- **Google mapping:** `RemoteEvent.uid` = `recurringEventId` or `id`, and `instance_key` = `originalStartTime` as UTC ISO (`…Z`) or `YYYY-MM-DD` for all-day. Cancelled stubs have `cancelled=true` and an epoch placeholder time, so delete them by `(uid, instance_key)`.
- **Use the same instance-key format** for ICS and CalDAV `RECURRENCE-ID`s: UTC ISO with a `Z` suffix, or `YYYY-MM-DD` for all-day, with `''` for non-recurring events.
- fa-domain: `calendar::{view, mask, EventMeta, CalendarMeta, Viewer, HouseholdFlags}` and `AccessTokenProvider` / `TokenError` exist. Read the actual signatures in `crates/fa-domain/src`.

## Wave 2 outcomes (fa-store, read before starting)
- Read the real signatures in `crates/fa-store/src/repo/*.rs` and `src/change.rs`. Don't assume them from the brief.
- **Transactions:** `WeatherSnapshotRepo::insert`, `WeatherRuleRepo::ensure_default`, and `AdvisoryRepo::upsert_by_dedupe` take the household from the transaction, so they must run in `tx_for`. `CalendarSourceRepo::due()` returns `(HouseholdId, CalendarSourceId)` pairs and must run in `tx_system`.
- `ChangeBus::publish` rejects payloads over 1 KB (`StoreError::PayloadTooLarge`). Exports: `fa_store::{CHANNEL, ChangeMessage}`.
- **If a repo method you need is missing:** add it in a **new file** `crates/fa-store/src/repo/ext_<your task id>.rs` (plus a one-line `mod`/`pub use` in `repo/mod.rs`), using the dev DB (migrated) for the sqlx macros. **Don't run `cargo sqlx prepare`**; the main session does that after the wave. List every such addition in your report. Don't edit other fa-store files.
- **Extra columns** beyond Feeds.md: `calendar_source.enabled` and `version`; `calendar.archived_at` and `version`.
- `EventRepo::upsert_synced` returns the number of rows inserted or changed (0 when nothing changed, and the version is not bumped), so publish `calendar.updated` only when the delete or upsert count is above 0. `delete_unseen_in_window` deletes only synced rows of that calendar overlapping `[from, to)` that aren't in `seen`.
- `EventRepo::range` returns `EventRow{event, color, members: Vec<(MemberId, Role)>, owner_role}`, and already excludes cancelled, proposed, and rejected events, plus disabled or archived calendars and sources. **You** apply the viewer scope and `view()`.
- `CalendarRepo::archive_missing` returns the archived ids, and `set_members`/`members` handle linking. Secrets are set with `SecretRepo`, then `CalendarSourceRepo::set_secret`.

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
