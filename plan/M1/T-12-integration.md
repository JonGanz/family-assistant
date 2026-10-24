---
id: T-12
title: "Integration: e2e stack, specs, compose and env, README"
milestone: M1
wave: 6
depends_on: [T-10, T-11]
agent: {model: sonnet, effort: medium}
status: done
owns: ["e2e/**", "apps/web/e2e/**", "scripts/**", "deploy/**", ".github/workflows/**", "README.md", "justfile", ".env.dev"]
---

# T-12: Integration: e2e stack, specs, compose and env, README

You are a sub-agent implementing one task of the M1 milestone (Dashboard core) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- plan/M1.md: Goal (the definition of done), Process config, and Risks
- arch/Frontend.md §7 (Testing), arch/Feeds.md §3 and §4
- the M0 integration pieces: `scripts/e2e.sh`, `scripts/ci-local.sh`, `scripts/compose-smoke.sh`, `e2e/`, `apps/web/e2e/`, `deploy/compose.example.yaml`, `deploy/.env.example`, `.github/workflows/ci.yml`, and `README.md`
- the reports of T-01 to T-11 (the main session pastes the README changes they requested)

## You own
- `e2e/**`, `apps/web/e2e/**`, `scripts/**`, `deploy/**`, `.github/workflows/**`
- `README.md`, `justfile`, `.env.dev`

## Work
1. **e2e stack** (driven by `scripts/e2e.sh`):
   - a fresh throwaway Postgres;
   - `fa-server serve api` and `fa-server serve worker` as **separate processes** (never `serve all` here);
   - an **ICS fixture server** (static files under `e2e/fixtures/ics/`: a fictional school feed with a weekly RRULE, one EXDATE, one RECURRENCE-ID override, an all-day event, and an event across a DST change, all on `example.com`);
   - an **Open-Meteo stub** (a tiny static or Node server returning a fixture `/v1/forecast` JSON that triggers the "Frost tonight" and "Likely rain today" default rules);
   - **Radicale** `tomsquest/docker-radicale:3.8.3.0` (verified with `docker manifest inspect`; re-check it), seeded with one calendar and a few events through PUT.

   Env: `FA_OPEN_METEO_URL` points at the stub and `FA_ALLOW_HTTP_FEEDS=true`. The script seeds the example household plus accounts, a paired device, and the sources through the API, then waits for the first sync and fetch. It tears everything down on exit, including on failure.
2. **Specs:**
   - **API level (`e2e/`):**
     - ICS and CalDAV sync produce the expected occurrences (the EXDATE is missing, the override has moved, the DST event's local time is correct);
     - `/weather` returns the fixture;
     - the advisories exist and are deduped across two fetches;
     - a child sees only permitted events, and the device sees masked busy events;
     - `config export` contains no secrets.
   - **UI level (`apps/web/e2e/`), from Frontend.md §7:**
     - the dashboard renders the agenda, weather, and an advisory;
     - an SSE update from the **worker** (trigger `fa-server weather fetch` or `calendar sync`) appears without a reload;
     - a child login sees only permitted events;
     - the settings flows: add an ICS source and enable its calendar; create a weather rule with an invalid condition (error shown), then fix it (preview shown).
3. **Deploy:**
   - `deploy/compose.example.yaml` and `deploy/.env.example` gain `FA_GOOGLE_CLIENT_ID`, `FA_GOOGLE_CLIENT_SECRET`, `FA_PUBLIC_URL`, `FA_OPEN_METEO_URL`, and `FA_ALLOW_HTTP_FEEDS` (commented, with the defaults);
   - `.env.dev` gets dev values;
   - `deploy/README.md` gets a "Google OAuth client" section (matching H-05) and notes that the api and the worker both need outbound HTTPS;
   - `compose-smoke.sh` still passes.
4. **CI:** make sure `ci.yml` runs anything new that the other tasks added (for example new crates are covered by `--workspace`). Add the e2e job only if it fits the CI runtime (under 10 min); otherwise keep e2e local, and say so in the README. `scripts/ci-local.sh` keeps mirroring `ci.yml`.
5. **README** (you own it, and must keep it current):
   - setup and prerequisites;
   - the new env vars and ports;
   - **Connecting calendars**: Google via OAuth (link to the deploy README's OAuth section, and mention the 7-day Testing-mode re-consent), the Google secret iCal address as an alternative, CalDAV, and ICS;
   - setting the household location and units;
   - configuring the dashboard layout;
   - the new CLI commands (`calendar sync`, `weather fetch`);
   - the e2e command;
   - the repository layout (new crates);
   - Roadmap: M1 → "code-complete" (once everything here passes).

   **The user's explicit requirement:** when M1 is done, the README must have two clear, complete, top-level sections:
   - **"Run it locally"**: a copy-pasteable path from a fresh clone to a working app with the example household, a weather fetch (live Open-Meteo, or the stub), an ICS calendar showing on the dashboard, and the dashboard opened in a browser. Include the separate-processes mode (`serve api` + `serve worker`).
   - **"Deploying"**: guidelines covering the compose example and its profiles, the env vars, the master key and its offline backup, `tailscale serve` (HTTPS on the tailnet), the Google OAuth client (H-05), backups to the NAS and restore, updating to a new version, and pairing the Echo Show. Keep the details in `deploy/README.md` and the runbooks, and link to them from the README.

   **Run every step you document**, on a clean checkout state (fresh dev DB, dev key, example import, serve, pair, see the dashboard), and fix the docs where reality differs.
6. Add `just e2e` and `just serve-split` (api + worker as separate processes) recipes, as thin wrappers.

## Acceptance
- [ ] `scripts/e2e.sh` passes: report the API and UI spec counts, computed from the output
- [ ] `scripts/ci-local.sh` passes
- [ ] `scripts/compose-smoke.sh` passes
- [ ] `actionlint` passes (if `ci.yml` changed)
- [ ] `scripts/check-example-data.sh` passes (fixtures are fictional, on `example.com`)
- [ ] Every README command was run, and the report lists them with pass or fail

## Waves 3–5 outcomes: README and e2e inputs (read before starting)
**State:** `scripts/ci-local.sh` is green on a fresh Postgres: Rust 208 passed, web 91 passed. The web app is complete: Today, Calendar, Members with avatars, Settings (household, calendars, weather, devices with the layout editor, status), `/oauth/done`, and the dashboard.

**README items the tasks reported:**
- **New CLI:**
  - `fa-server calendar sync <source-id>`.
  - `fa-server weather fetch`.
  - `config import` now validates weather-rule conditions (it names the rule and position on error) and seeds the 8 default rules.
  - `user add`/`reset-password` and `device pair` are unchanged in use.
- **Processes:**
  - `serve api` needs the master key and runs the `fa_change` LISTEN bridge.
  - `serve worker` runs the jobs.
  - Both can run as separate processes (that's how compose runs them). `serve all` is both in one.
- **Env vars:**
  - `FA_GOOGLE_CLIENT_ID`/`FA_GOOGLE_CLIENT_SECRET` (when unset, Google is hidden in the UI).
  - `FA_PUBLIC_URL` (required with Google: https, the OAuth redirect `${FA_PUBLIC_URL}/api/v1/oauth/google/callback`).
  - `FA_OPEN_METEO_URL` (default `https://api.open-meteo.com`).
  - `FA_ALLOW_HTTP_FEEDS` (default false; needed for LAN CalDAV over http and for the e2e fixtures).
- **Household TOML:**
  - `[household]` gains `units`, `calendar_past_days`, `calendar_future_days`, `weather_retention_days`, and `private_events_as_busy`.
  - New sections `[[calendar_sources]]` (`[[calendar_sources.calendars]]`) and `[[weather_rules]]`.
  - Secrets are never exported; imported sources come in disabled with `needs_reauth`.
- **UI routes:** `/calendar`, `/settings/{household,calendars,weather,devices,status}`, `/oauth/done`, and `/dashboard`. The dashboard layout is edited under Settings → Devices → Edit layout.
- **Visibility (D-036):** adults' private events show as Busy on the dashboard and to children. The household toggle is in Settings → Household.

**Known limitations to note in the README:**
- A saved household location can't be cleared from the UI (a null `location` in the PATCH means "unchanged").
- The Google Testing-mode refresh token expires after 7 days.

**e2e notes:**
- The dashboard e2e test expects `data-testid="household"`, which sits inside the clock panel.
- Avatars on the kiosk load as blob URLs fetched with the device token.
- `GET /api/v1/dashboard` is device-only.
- Don't use the dev DB, and don't kill pid 2679601 (an unrelated, older `serve all`).

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
