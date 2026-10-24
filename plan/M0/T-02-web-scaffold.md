---
id: T-02
title: Vue app scaffold
milestone: M0
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/**"]
---

# T-02: Vue app scaffold

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §1, §4
- decisions/D-011 (dashboard keep-alive)
- decisions/D-006, D-019

## You own
- `apps/web/**`

## Work
1. Scaffold `apps/web` with Vite and **Vue 3 + TypeScript** (`<script setup>`). Add Pinia, vue-router, `vite-plugin-pwa` (manifest named "Family Assistant", with placeholder icons), ESLint (flat config) with the Vue/TS rules, Prettier, and vitest with @vue/test-utils. Set `"license": "AGPL-3.0-or-later"` in package.json.
2. Package manager: **pnpm via corepack** (`corepack pnpm@9 install`). Commit `pnpm-lock.yaml`, and set `packageManager` in package.json.
3. Vite dev server proxies `/api`, `/healthz`, and `/readyz` to `http://127.0.0.1:8080`.
4. Routes, as placeholders only: `/login`, `/` (app home), `/members`, `/settings/devices`, `/dashboard` (fullscreen kiosk layout, no app chrome). Make a layout split: `AppLayout` for users and `KioskLayout` for the dashboard.
5. Write `src/api/http.ts`, a small fetch wrapper. It always sends `credentials: 'include'`, adds `X-FA-Request: 1` on mutating requests, adds `Authorization: Bearer` when a device token is present, and parses `application/problem+json` errors into a typed `ApiError`. Leave `src/gen/` as a placeholder (README) for the generated types and client (T-09 fills it).
6. Write `src/kiosk/` utilities with unit tests:
   - `deviceToken.ts`: read the token from `location.hash` (`#device=…`), save it to localStorage (wrapped in try/catch), and clear the hash.
   - `keepAlive.ts`: Screen Wake Lock with re-acquire on `visibilitychange`, a fallback muted looping 1-px inline video (generated in code, or a tiny bundled asset), and a periodic DOM tick.
   - `watchdog.ts`: reload if there's been no heartbeat for 120 s, plus a nightly reload at 03:30 local time.
7. Base styling: CSS custom properties for light and dark, and a large-type dashboard scale (readable at 1920×1080 from across a room). No UI framework.

## Acceptance
- [ ] `corepack pnpm@9 install && corepack pnpm@9 run lint && corepack pnpm@9 run typecheck && corepack pnpm@9 run test && corepack pnpm@9 run build` all pass (add those scripts)
- [ ] The kiosk utility unit tests cover the hash → localStorage → hash-cleared flow and the watchdog timeout (using fake timers)

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
