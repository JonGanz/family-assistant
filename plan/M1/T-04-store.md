---
id: T-04
title: "fa-store: M1 migration, repos, ChangeBus, .sqlx"
milestone: M1
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-store/**", ".sqlx/**"]
---

# T-04: fa-store: M1 migration, repos, ChangeBus, .sqlx

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §2 (every table and column), §3.3 step 4, §6 (ChangeBus)
- arch/Domain.md *Persistence conventions*
- plan/M1.md: *Migrations*, *Domain types*, *Jobs*
- crates/fa-store (the M0 migrations, `store.rs` `tx_for`/`tx_system`, `repo/*` patterns, `SecretRepo`)

## You own
- `crates/fa-store/**`
- `.sqlx/**` (the offline query data, wherever M0 keeps it)

## Work
1. **Migration** `migrations/20261025000001_m1_feeds.{up,down}.sql`, holding exactly the tables and columns of Feeds.md §2:
   - `household` gains its columns, with defaults and CHECKs;
   - `device.settings jsonb NOT NULL DEFAULT '{}'`;
   - new tables: `member_avatar`, `oauth_account`, `oauth_pending`, `calendar_source`, `calendar`, `calendar_member`, `event`, `event_member`, `weather_snapshot`, `weather_rule`, `advisory`.

   Rules:
   - Enums are `text` + CHECK, using the values from T-01's enums.
   - Unique constraints: `calendar(source_id, remote_id)`, `event(calendar_id, remote_uid, instance_key)`, `advisory(household_id, dedupe_key)`.
   - Indexes: `event(household_id, start_at)`, `event(calendar_id, start_at)`, `weather_snapshot(household_id, fetched_at DESC)`.
   - `event`: `CHECK ((origin = 'synced') = (status IS NULL))` and `CHECK (end_at >= start_at)`.
   - RLS and grants to `fa_app` exactly as in migration 0002 (the `tenant` policy with `app_is_system()`), on every new table.
   - **Never edit the M0 migrations.** The down migration drops everything in reverse.
2. **Repositories** (`repo/*.rs`, associated functions taking `&mut PgConnection` and returning `Versioned<T>` where an entity is versioned; optimistic `expected_version` → `StoreError::Conflict`). These are **the exact methods later tasks rely on**. Add more if you need them, but don't rename these:
   - **HouseholdRepo** (extend): `update_settings(conn, id, &HouseholdPatch-equivalent, expected_version) -> i32`. `get` returns the new fields.
   - **MemberRepo** (extend): `unarchive(conn, id)` (M0 follow-up); `list_including_archived`.
   - **MemberAvatarRepo:** `put(conn, member, mime, bytes, sha256)`, `get(conn, member) -> Option<(mime, bytes, sha256)>`, `delete(conn, member)`.
   - **DeviceRepo** (extend): `get_settings(conn, id) -> serde_json::Value`, `set_settings(conn, id, value, expected_version) -> i32`.
   - **OAuthAccountRepo:**
     - `upsert(conn, household, provider, owner_member, email, scopes, secret_id) -> Versioned<OAuthAccount>`, keyed by (household, provider, email);
     - `get`, `list(conn, owners: Option<&[MemberId]>)`;
     - `set_state(conn, id, state, last_error)`, `set_secret(conn, id, secret_id)`.
   - **OAuthPendingRepo:** `insert(conn, state, session_id, member, pkce_verifier, return_to, expires_at)`; `take(conn, state) -> Option<Pending>` (delete-returning, so it is single-use); `delete_expired(conn) -> u64`. It has `household_id` and the standard `tenant` policy (`app_household() OR app_is_system()`), like every other table.
   - **CalendarSourceRepo:**
     - `insert`, `get`, `list(conn, include_archived)`, `update(conn, &patch, expected_version)`, `archive`, `set_secret(conn, id, secret_id)`;
     - `set_sync_result(conn, id, state, last_error, synced_at)`;
     - `due(conn, now) -> Vec<CalendarSourceId>` (enabled, not archived, `last_synced_at` null or `+ poll_minutes` ≤ now). This one **runs through `tx_system`** across households, returning `(household_id, source_id)`.
   - **CalendarRepo:**
     - `upsert_discovered(conn, source, remote_id, name, color) -> (Versioned<Calendar>, created: bool)`; new calendars are inserted `enabled = false`;
     - `list(conn, source: Option)`, `get`, `update(conn, &patch, expected_version)`, `archive_missing(conn, source, seen_remote_ids)`;
     - `set_sync_state(conn, id, sync_state: Option<String>, full_sync_at: Option<DateTime>)`;
     - `set_members(conn, id, &[MemberId])`, `members(conn, id)`.
   - **EventRepo:**
     - `upsert_synced(conn, calendar, &[NewOccurrence]) -> u64`, keyed by (calendar, remote_uid, instance_key); it updates the content fields and `version`;
     - `delete_unseen_in_window(conn, calendar, from, to, seen: &[(String, String)]) -> u64`;
     - `delete_by_uid(conn, calendar, uid, instance_key: Option<&str>)` (Google cancellations);
     - `delete_calendar_events(conn, calendar)` (on resync or archive);
     - `prune_before(conn, before) -> u64` (synced only);
     - `range(conn, from, to, calendars: Option<&[CalendarId]>) -> Vec<EventRow>`. An `EventRow` carries the event fields **plus** its calendar's `color`, `dashboard_visibility`, and member IDs (an aggregated array), so fa-calendar can apply `view()` without N+1 queries.
   - **WeatherSnapshotRepo:** `insert(conn, provider, current, hourly, daily) -> id`, `latest(conn) -> Option<WeatherSnapshot>`, `delete_older_than(conn, before) -> u64`.
   - **WeatherRuleRepo:** `insert`, `get`, `list(conn, statuses: Option<&[RuleStatus]>)`, `update(conn, &rule, expected_version)`, `archive`, `ensure_default(conn, name, condition, action, severity, message) -> bool` (inserts only if no rule with that name and `suggested_by = default` exists, even an archived one).
   - **AdvisoryRepo:** `upsert_by_dedupe(conn, rule, text, severity, valid_from, valid_until, source, dedupe_key) -> (id, changed: bool)` (doesn't resurrect a dismissed advisory); `active(conn, now) -> Vec<Advisory>` (not dismissed, `valid_until > now`); `dismiss(conn, id, by_member: Option)`; `delete_expired(conn, before) -> u64`.
   - **JobRepo** (extend, for StatusView): `counts(conn) -> {ready, running, dead}`.
3. **ChangeBus** (`change.rs`):
   - `pub struct ChangeBus;` with `publish(conn, household: HouseholdId, kind: &str, data: &serde_json::Value)`, which runs `SELECT pg_notify('fa_change', $1)` with `{household, kind, data}`. It is called inside the writer's transaction, so it is delivered on commit.
   - Reject payloads over 1 KB with an error (don't truncate).
   - `pub const CHANNEL: &str = "fa_change"` and `pub struct ChangeMessage{household, kind, data}` (serde) for the listener side, which T-09 owns.
4. **Secrets:** no secret values in any new column. Sources and accounts reference `secret.id`. Use `SecretRepo` with `owner_type` values `oauth_account` and `calendar_source`.
5. **Tests** (`#[sqlx::test]`):
   - the migration applies and reverts;
   - RLS isolates two households on every new table;
   - the event upsert is idempotent and `delete_unseen_in_window` only touches its calendar and window;
   - the range query returns color, visibility, and members;
   - `due()` respects `poll_minutes` and archive;
   - advisory dedupe and no resurrection after dismiss;
   - `ensure_default` is idempotent;
   - `take` on oauth_pending is single-use;
   - version conflicts;
   - ChangeBus delivers on commit and not on rollback (use a `PgListener` in the test);
   - `MemberRepo::unarchive`.
6. Run `cargo sqlx prepare --workspace` (against the migrated dev DB) and commit the offline data. Update the dev DB's `_sqlx_migrations` only by running the migration, never by editing it.

## Acceptance
- [ ] `cargo test -p fa-store` passes on the dev DB
- [ ] It also passes on a **fresh** throwaway Postgres: `docker run -d --rm --name fa-t04 -e POSTGRES_PASSWORD=fa -p 55499:5432 pgvector/pgvector:pg16`, then `SQLX_OFFLINE=true DATABASE_URL=postgres://postgres:fa@127.0.0.1:55499/postgres cargo test -p fa-store`, then `docker stop fa-t04`
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-store --all-targets -- -D warnings` passes
- [ ] `cargo fmt --check` is clean

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
