---
id: T-09
title: Web: login, members, devices, dashboard + keep-alive, generated client
milestone: M0
wave: 6
depends_on: [T-08]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/**"]
---

# T-09: Web: login, members, devices, dashboard + keep-alive, generated client

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- plan/M0.md (contracts)
- apps/web (scaffold from T-02)
- apps/web/src/gen/openapi.json and types (from T-03, T-08)
- decisions/D-006, D-011, D-019

## You own
- `apps/web/**`

## Work
1. **Generated client:** add `openapi-typescript` (types from `src/gen/openapi.json`) and a thin typed wrapper over `src/api/http.ts`. Add a `gen` script. Keep the ts-rs types and the OpenAPI types consistent, preferring OpenAPI for request and response shapes.
2. **Auth:**
   - A login page.
   - A Pinia `session` store (`/me`).
   - Route guards: user routes need a user actor, and `/dashboard` needs a device token (if missing, show "Pair this screen: ask an adult to open Settings → Devices").
   - Logout.
3. **Members page:** a list, with create and edit for adults. Children see a read-only view. Show conflicts (409) inline with a "reload" action.
4. **Settings → Devices** (adults): pair a device, which shows the pair URL as text **and** a QR code (a small MIT library such as `qrcode`), list devices, and revoke.
5. **Dashboard** (`/dashboard`, KioskLayout):
   - A clock, the date, the household name, and a member strip with avatars and colours.
   - A connection indicator.
   - Wire up `deviceToken`, `keepAlive`, and `watchdog` from T-02.
   - An SSE client (EventSource has no custom headers, so **use `fetch` streaming with the Authorization header**, or a small polyfill, and document which) with `Last-Event-ID` reconnect and backoff. Heartbeats reset the watchdog.
   - A **member picker + PIN pad** demo, behind a "Try elevation" button that calls `/device/elevate` and shows the remaining elevation time. This proves the D-019 flow.
6. A service worker (vite-plugin-pwa) caches the app shell. On API failure, the dashboard shows the last known state.
7. Tests: vitest for the stores, the SSE reconnect logic, and the PIN pad. A Playwright test file is set up for T-10 to run (write `e2e/smoke.spec.ts` covering login → members → pair device → open the dashboard with the token → see the heartbeat).

## Acceptance
- [ ] `corepack pnpm@9 run lint typecheck test build` passes
- [ ] Manual against `fa-server serve all` with the built dist: login works, and the dashboard shows the heartbeat (describe what you verified)

## Ground rules (all M0 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M0.md` (especially the contracts), and the arch sections listed under *Read first*.
- **Edit only the paths you own** (listed below). If you need a change somewhere else, don't make it: say so in your report.
- Code style: idiomatic Rust 2024 edition, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs. Keep comments sparse and useful.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com`.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups or requests for other tasks' paths.
