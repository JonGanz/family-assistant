---
id: T-11
title: "Web dashboard: panels, layout editor, live updates, offline"
milestone: M1
wave: 5
depends_on: [T-09, T-03]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/dashboard/** (except dashboard/layout/**)", "apps/web/src/kiosk/**", "apps/web/src/layouts/KioskLayout.vue", "apps/web/src/stores/dashboard.ts", "apps/web/src/views/DashboardView.vue", "apps/web/src/views/DevicesView.vue", "apps/web/vite.config.ts"]
---

# T-11: Web dashboard: panels, layout editor, live updates, offline

You are a sub-agent implementing one task of the M1 milestone (Dashboard core) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Frontend.md §4 (Dashboard), §5, §7
- arch/Feeds.md §3.5 (what busy means for display; D-036)
- plan/M1.md: `/dashboard`, `/devices/{id}`, the SSE kinds, and the *Web ownership split*
- `apps/web/src/gen/api.d.ts`, the T-03 layout engine (`dashboard/layout/`), and the T-03 shared components and utilities
- the M0 kiosk code (`kiosk/`, `stores/dashboard.ts`, `views/DashboardView.vue`, `views/DevicesView.vue`)

## You own
- `apps/web/src/dashboard/**` **except** `dashboard/layout/**` (T-03)
- `apps/web/src/kiosk/**`, `apps/web/src/layouts/KioskLayout.vue`
- `apps/web/src/stores/dashboard.ts`
- `apps/web/src/views/DashboardView.vue`, `apps/web/src/views/DevicesView.vue`

Import API calls from `api/client.ts` (owned by T-10). If you need a call that T-10 hasn't added yet, add a local wrapper in `dashboard/api.ts` that uses `gen/` types, and report it so the main session can merge the two.

## Work
1. **Dashboard store:**
   - Hold one `DashboardView` snapshot from `GET /api/v1/dashboard?days=N` (N from `settings.agenda_days`).
   - Extend the M0 localStorage cache to the full snapshot with `savedAt`.
   - Apply SSE kinds (Frontend.md §4.2):
     - `calendar.updated` refetches only if the range overlaps the shown window;
     - `weather.updated`, `advisory.updated`, `device.updated` (for this device), `member.updated`, and `household.updated` refetch;
     - `resync` does a full refetch.
   - Refetch at midnight in the household tz.
2. **Panels** (`dashboard/panels/`): `clock`, `weather` (now, the next 12 h when `hourly`, and 5–7 days), `advisories` (severity colours; dismiss without a PIN through `/advisories/{id}/dismiss`), `agenda` (today + N days grouped by day, member chips, busy blocks labelled "Busy"), `week` (a 7-column grid), and `members` (avatars; tapping one opens the M0 PIN elevation flow, kept as is). Unknown panel kinds render nothing.
3. **DashboardView:** render `settings.panels` through the T-03 layout engine:
   - landscape 12-column grid (clock, weather, and advisories on the left in 4 columns; agenda or week on the right in 8);
   - portrait stack;
   - the kiosk type scale.

   Theme `auto` uses sunrise and sunset from the weather. Burn-in shift: in dark theme, translate the content by a few px every 10 min. The "last updated hh:mm" badge shows while the stream is down or the data is stale. Keep the M0 keep-alive, watchdog, and pairing behaviour unchanged.
4. **Settings → Devices** (`DevicesView.vue`): keep the M0 pairing list and add a **layout editor** for each device:
   - orientation, theme, `agenda_days`, the member filter, presets ("Family hub", "Calendar wall", "Minimal" from T-03), and panel add, remove, and reorder;
   - a **live scaled preview** that renders the real panels with the current household data (fetched through the user-session endpoints, not the device token) under the edited settings.

   Save via `PATCH /devices/{id}` with `If-Match`.
5. Service worker: make sure avatars and the app shell are cached (vite-plugin-pwa runtime caching for `/api/v1/members/*/avatar`). You own `vite.config.ts` for this. Also point the maskable PWA icon at `icon-maskable-512.png` (added by T-03). Kiosk panels must sit under the `.kiosk` class to get the 28 px base. Put user-visible strings in `dashboard/strings.ts`.
6. **Tests** (vitest):
   - the store applies each SSE kind correctly (range overlap, own device only);
   - cache read and write, and the stale badge;
   - unknown panels are ignored;
   - a busy event renders without its title;
   - the layout editor emits valid `DashboardSettings`.

## Acceptance
- [ ] `corepack pnpm@9 --dir apps/web lint` passes
- [ ] `corepack pnpm@9 --dir apps/web typecheck` passes
- [ ] `corepack pnpm@9 --dir apps/web test` passes
- [ ] `corepack pnpm@9 --dir apps/web build` passes
- [ ] Manual: `serve api` and `serve worker` run as separate processes. Pair a browser at 1920×1080. The dashboard shows weather, the agenda, and advisories. Change the layout in Settings → Devices in a second browser, and the dashboard updates without a reload. Stop the api, and the cached snapshot and stale badge show

## Wave 3 outcomes (read before starting)
- `WeatherView.hourly` and `daily` are **column-oriented** objects keyed by Open-Meteo field names (`{time: [...], temperature_2m: [...], ...}`), trimmed to 48 h and 7 days, all metric. Convert them with T-03's `lib/units.ts`.

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
