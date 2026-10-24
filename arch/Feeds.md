---
title: Feeds — calendar read & weather
scope: calendar sync (Google, CalDAV, ICS), Google OAuth accounts, weather fetch, WeatherRules, advisories, cross-process change events
status: draft
updated: 2026-10-24
drawio: Feeds.drawio
drawio_src: src/Feeds.py
depends_on: [D-004, D-006, D-008, D-014, D-017, D-018, D-019, D-023, D-034, D-035, D-036]
milestone: M1
---

# Feeds: calendar read & weather (DRAFT v1)

This doc covers the backend half of M1: pulling calendars and weather in, storing them, and turning weather into advisories. The UI half is in [Frontend.md](Frontend.md). Calendar **writes**, event extraction, and the approval inbox come in M3 (`Ingest.md`).

## 1. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-connectors` | Thin, stateless HTTP clients that return neutral DTOs: `google` (OAuth token exchange and refresh, CalendarList, Events), `caldav` (discovery, calendar list, `calendar-query` REPORT), `ics` (conditional GET, parse), `openmeteo` (forecast). **No DB access.** | (external crates only) |
| `fa-calendar` | Sources → calendars discovery, per-calendar sync, recurrence expansion, member linking, visibility masking, range queries. Jobs `calendar.sync`, `calendar.sync_due`, `calendar.prune` | fa-domain, fa-store, fa-jobs, fa-connectors |
| `fa-weather` | Fetch job, snapshots, the rule DSL (parser and evaluator), advisories, default rules, retention | fa-domain, fa-store, fa-jobs, fa-connectors |
| `fa-auth` (extended) | `OAuthService`: Google authorisation-code + PKCE flow, the `oauth_account` lifecycle, refresh tokens stored as `Secret`s. Shared with Gmail in M3 | fa-store, fa-connectors |
| `fa-store` (extended) | M1 migration and repositories, plus **`ChangeBus`** (§6) | fa-domain |

`fa-calendar` and `fa-weather` are feature crates. They depend on `fa-connectors` (a leaf client crate), never on each other.

**External crates** (checked on crates.io on 2026-10-24): `reqwest 0.13` (rustls; the Google token exchange is two hand-rolled form POSTs), `icalendar 0.17` (parser), `rrule 0.14`, `quick-xml 0.42`, and `wiremock 0.6` for tests.

## 2. Data model (page "Model")
This refines the Calendar and Weather packages of [Domain.md](Domain.md). All tables are household-scoped with RLS, and follow the persistence conventions there.

| Table | Key columns | Notes |
|---|---|---|
| `household` (+cols) | `units: metric\|imperial` (default `metric`), `calendar_past_days` (30), `calendar_future_days` (400), `weather_retention_days` (7), `private_events_as_busy` (true, D-036) | Editable in the UI (Settings → Household) |
| `oauth_account` | provider: `google`, owner_member, email, scopes[], secret (refresh token), state: `ok\|needs_reauth\|error`, last_error? | One per connected Google account. Gmail (M3) reuses it with extra scopes (incremental consent) |
| `oauth_pending` | state (PK), session, member, pkce_verifier, return_to, expires_at (10 min) | Short-lived flow state, cleaned hourly by the `oauth.cleanup` job |
| `calendar_source` | kind: `google\|caldav\|ics`, name, owner_member?, oauth_account? (google), url? (caldav base), username? (caldav), secret? (caldav password, **or the whole ICS URL**), writable=false, poll_minutes (default 15, ICS 60), state, last_synced_at?, last_error?, archived_at? | `owner_member = null` means a household source. ICS URLs often embed a secret token (Google's "secret address", school portals), so the URL is a `Secret` and the API shows it masked |
| `calendar` | source, remote_id, name, color, enabled, is_default_target (M3), sync_state (Google `syncToken` / CalDAV `getctag` / ICS `ETag`), last_full_sync_at | Unique `(source, remote_id)`. An ICS source has exactly one calendar |
| `calendar_member` | calendar ↔ member (M:N) | No members means a **household calendar** |
| `event` | calendar?, remote_uid?, instance_key (`''` or the RECURRENCE-ID as UTC ISO), title, description?, location?, start_at, end_at, all_day, start_date?/end_date? (all-day), source_tz?, is_private, cancelled, origin: `synced\|extracted\|manual`, status? (`null` for synced; `proposed\|approved\|written\|rejected` from M3), confidence?, source_ref, dedupe_key, remote_etag?, version | **One row per occurrence** (see §3.3). Unique `(calendar, remote_uid, instance_key)`. Index on `(household_id, start_at)` |
| `event_member` | event ↔ member | Used by extracted events (M3). Synced events take their members from the calendar |
| `weather_snapshot` | fetched_at, provider: `open-meteo`, current jsonb, hourly jsonb, daily jsonb | Always metric. Hard-deleted after `weather_retention_days` |
| `weather_rule` | name, condition (DSL text), action: `alert\|task`, severity: `info\|warning\|severe`, message, task_definition? (M2), status: `active\|suggested\|disabled`, suggested_by: `default\|s2\|user`, archived_at?, version | `task` rules can be saved in M1 but only fire from M2 |
| `advisory` | rule?, text, severity, valid_from, valid_until, source: `rule\|s2`, dedupe_key (unique per household), dismissed_at?, dismissed_by? | Shown on the dashboard until `valid_until` or dismissed |
| `member_avatar` | member (PK), mime, bytes (≤ 512 KB), sha256 | Served with an ETag. Only images (png, jpeg, webp) are accepted |
| `device` (+col) | `settings jsonb` | Dashboard layout (Frontend.md §4). An opaque UI blob, never queried |

## 3. Calendar sync
### 3.1 Connecting sources
| Kind | How an adult adds it | Auth |
|---|---|---|
| `google` | Settings → Calendars → "Connect Google account" → OAuth (scope `calendar.readonly` in M1) → pick calendars from CalendarList | `oauth_account` refresh token (Secret). Client ID and secret are process config (`FA_GOOGLE_CLIENT_ID`/`_SECRET`), with the redirect `https://<tailnet-host>/api/v1/oauth/google/callback` |
| `caldav` | Base URL + username + app password → discovery → pick calendars | Basic auth, password as Secret |
| `ics` | Paste a URL (`webcal://` becomes `https://`) | The URL itself is the Secret |

- A Google calendar can also be added **without OAuth** as `ics` using its "secret address in iCal format". This avoids Google's 7-day re-consent in Testing mode, at the cost of slower updates (Google refreshes those feeds slowly). The UI explains both choices.
- **Ownership (D-006):** an adult can add and edit household sources, their own, and children's. An adult can't see another adult's credentials or edit their sources. Anyone allowed to read calendars can still see the events, subject to visibility (§3.5).

### 3.2 Scheduling
- `calendar.sync_due` runs every minute. It enqueues `calendar.sync{source}` for each enabled source whose `last_synced_at + poll_minutes` has passed, using the idempotency key `calendar.sync:{source}:{slot}`.
- An adult can force a sync (`POST /calendar-sources/{id}/sync`).
- `calendar.prune` (daily) deletes synced occurrences that ended before `calendar_past_days`.

### 3.3 One sync run
1. **Discover** the calendar list (Google CalendarList, CalDAV PROPFIND `calendar-home-set`). New remote calendars are created **disabled**, and an adult enables them. The exception is an ICS source: its single calendar is created **enabled**, because the adult just chose that one feed. Remote deletions archive the calendar.
2. **Fetch** each enabled calendar, within the window `[now − past_days, now + future_days]`:
   - **Google:** `events.list?singleEvents=true` (Google expands recurrences). The first call is bounded by `timeMin`/`timeMax`, and later calls pass `syncToken` (incremental). A `410 Gone` clears the token and does a full resync.
   - **CalDAV:** if `getctag` has not changed, skip. Otherwise run a `calendar-query` REPORT with a `time-range` filter, and expand locally.
   - **ICS:** a conditional GET (`If-None-Match`/`If-Modified-Since`). On `304`, skip. Otherwise parse and expand locally.
3. **Expand locally** (CalDAV and ICS) with `rrule`, in the event's own TZID. Apply `EXDATE`, and let overriding `RECURRENCE-ID` components replace the matching occurrence. Each occurrence is a row with `instance_key` = its RECURRENCE-ID.
4. **Upsert** the occurrences by `(calendar, remote_uid, instance_key)`, then delete rows for this calendar inside the window that this run didn't see. For Google incremental syncs, `status=cancelled` deletes the row.
5. Commit, then publish `calendar.updated{from, to}` on the ChangeBus (§6).

**Freshness:** every source also does a full (non-incremental) fetch when `last_full_sync_at` is missing or older than 24 h. A `304`, an unchanged ctag, or a Google `syncToken` (which only covers its original `timeMin`/`timeMax`) would otherwise let the rolling window go stale. Recurrences expand on wall-clock time in the event's zone, so local times stay fixed across DST. Unknown TZIDs go through a Windows→IANA table, and otherwise fall back to the household tz, with a warning in `last_error` (the state stays `ok`).

**Errors:** a `401`/`invalid_grant` sets the source (and its `oauth_account`) to `needs_reauth`. Other errors set `error` with `last_error`, and the job retries with backoff. The System status page shows each source's state.

**Times:** timed events are stored as UTC `start_at`/`end_at`. All-day events store `start_date`/`end_date` (end exclusive) and also a UTC `start_at`/`end_at` computed in the household tz, so range queries work for both.

### 3.4 Member linking (M1)
An event's members are the members of its calendar. A calendar with no members is a household calendar. Linking events by content (attendee e-mail, title mentions) is a System One decision point, `event.member_link`, in M3/M4 (D-027).

### 3.5 Visibility (D-036)
`fa-calendar` applies one pure function, `fa_domain::calendar::view(event, calendar, viewer, flags)`, to every query. The result is **full** or **busy** (title, location, and description are replaced by "Busy"; the time block and member colour stay). Scope rules (which events a viewer gets at all) are applied in the query.

**Adult-owned private event:** the event is private at its source (`visibility=private\|confidential`, `CLASS:PRIVATE\|CONFIDENTIAL`) **and** at least one linked member is an adult. A calendar with no members uses its source owner instead, and with no owner either it counts as an adult's.

| Viewer | Rule |
|---|---|
| Adult user | Everything, in full |
| Child user | Events on calendars linked to them, plus household calendars. Adult-owned private events are busy when `private_events_as_busy` is on |
| Dashboard device | All enabled calendars. Adult-owned private events are busy when `private_events_as_busy` is on |
| Elevated device | Same as the dashboard device. Elevation is only for actions |

## 4. Weather
- `weather.fetch` runs on the cron `*/30 * * * *` (editable in the schedule settings). It skips when the household has no location. It calls Open-Meteo `/v1/forecast` with `current`, `hourly` (48 h), and `daily` (7 days) in metric units and the household tz, then stores a `weather_snapshot`.
- **Fields used:** current: `temperature_2m`, `apparent_temperature`, `weather_code`, `wind_speed_10m`, `wind_gusts_10m`, `precipitation`, `is_day`. Hourly: `temperature_2m`, `apparent_temperature`, `precipitation_probability`, `precipitation`, `snowfall`, `wind_speed_10m`, `wind_gusts_10m`, `uv_index`, `weather_code`. Daily: `weather_code`, `temperature_2m_max`/`_min`, `precipitation_probability_max`, `precipitation_sum`, `sunrise`, `sunset`, `uv_index_max`.
- After each fetch, `weather.evaluate` runs every `active` rule against the snapshot. A rule that matches upserts an `advisory` with the dedupe key `rule:{id}:{window start date}`, so a rule fires at most once per window. It then publishes `weather.updated` (and `advisory.updated` if anything changed).
- `weather.retention` (daily) deletes snapshots older than `weather_retention_days`.
- **Units:** storage and the DSL are always metric. The API returns metric, and the UI converts using `household.units`.

### 4.1 Rule DSL (deterministic, D-017)
```
rule      := cond ( ("and" | "or") cond )*
cond      := agg "(" field "," window ")" op number
agg       := min | max | sum | avg | any
field     := temp_c | feels_c | precip_prob | precip_mm | snow_cm | gust_kmh | wind_kmh | uv | code
window    := "next" INT "h" | today | tonight | tomorrow
op        := <= | < | >= | > | == | !=
```
- `tonight` means 18:00 to 08:00 and `today` means now until midnight, both in the household tz. `any(code, today) == 95` is true if any hour has that WMO code.
- `and` binds tighter than `or`. No parentheses (kept simple on purpose).
- The parser returns precise errors, with positions, for the settings UI. `POST /weather-rules/validate` also evaluates the rule against the latest snapshot as a preview.
- The message is a template with `{value}`, `{when}`, and `{field}`, for example "Frost tonight, low of {value}". `{value}` is formatted in the household's units.

**Default rules** (`suggested_by: default`, seeded active, and all editable or disableable):

| Name | Condition | Severity |
|---|---|---|
| Frost tonight | `min(temp_c, tonight) <= 0` | warning |
| Heavy rain | `sum(precip_mm, next 12h) >= 15` | warning |
| Likely rain today | `max(precip_prob, today) >= 70` | info |
| Snow | `sum(snow_cm, next 24h) >= 2` | warning |
| Strong wind | `max(gust_kmh, next 12h) >= 60` | warning |
| Heat | `max(feels_c, today) >= 32` | warning |
| High UV | `max(uv, today) >= 8` | info |
| Thunderstorm | `any(code, today) >= 95` | severe |

The S2-suggested rules and LLM advisories of D-017 come in M4. Weather-driven tasks come in M2.

## 5. Configuration (D-018)
- The household TOML gains `[household] units`, `calendar_past_days`, `calendar_future_days`, `weather_retention_days`, `private_events_as_busy`, plus `[[calendar_sources]]` and `[[weather_rules]]`.
- Calendar sources are **exported without secrets**: Google sources export as a placeholder that needs reconnecting, CalDAV without the password, and ICS without the URL. On import, these sources come in **disabled** with the state `needs_reauth`, and the UI asks for the missing credential.
- `calendar_sources[].calendars[]` carries `remote_id`, `enabled`, `members` (member keys), and `color`, so linking survives a round trip.

## 6. Cross-process change events (ChangeBus)
The api and the worker are separate processes (Platform.md §3), so the worker can't use the api's in-process SSE `Hub`. The bridge works like this:
- `fa_store::ChangeBus::publish(tx, household, kind, data)` calls `pg_notify('fa_change', {household, kind, data})` **inside the transaction**, so the event goes out only on commit. The payload stays small (IDs and ranges, ≤ 1 KB, well below the 8 KB NOTIFY limit).
- `fa-api` runs one `PgListener` on `fa_change` and pushes each message into the `Hub`, which assigns the SSE id and keeps the replay ring. It reconnects with backoff, and after a reconnect it broadcasts `resync`, which tells clients to refetch.
- In `serve all`, the same path is used, so there is only one code path to test.
- New SSE kinds in M1: `calendar.updated{from, to}`, `weather.updated`, `advisory.updated`, `household.updated`, `device.updated{id}`, `source.updated{id, state}`, `resync`.

## 7. Security notes
- Secrets (refresh tokens, CalDAV passwords, ICS URLs) use the existing envelope encryption, and are never returned by the API or written to logs. The `reqwest` client logs only the host.
- The OAuth `state` is a random value bound to the user's session and stored in a short-lived `oauth_pending` row (10 min, with the PKCE verifier). The callback rejects a missing or mismatched state.
- Outbound fetches have a 20 s timeout and a 10 MB body limit, and accept `https` only (plus `http` when `FA_ALLOW_HTTP_FEEDS=true`, for LAN CalDAV servers and tests).
