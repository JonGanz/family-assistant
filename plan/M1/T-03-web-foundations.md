---
id: T-03
title: "Web foundations: tokens, shared components, units, tz date utils, layout engine, PWA icons"
milestone: M1
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/styles/**", "apps/web/src/components/**", "apps/web/src/composables/**", "apps/web/src/lib/**", "apps/web/src/dashboard/layout/**", "apps/web/public/**"]
---

# T-03: Web foundations: tokens, shared components, units, tz date utils, layout engine, PWA icons

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Frontend.md §1, §3, §4.1, §4.2 (day rollover, burn-in), §6
- plan/M1.md: *Web ownership split*
- apps/web/src (the M0 code: styles/base.css, components/PinPad.vue, kiosk/)

## You own
- `apps/web/src/{styles,components,composables,lib}/**`
- `apps/web/src/dashboard/layout/**`
- `apps/web/public/**`

## Work
This is **pure TypeScript with local types**. The generated API types don't exist yet, so define minimal local interfaces in `src/lib/types.ts` (for example `EventLike{start_at, end_at, all_day, start_date?, end_date?, members: string[], color, visibility}`, `MemberLike{id, display_name, color, avatar_url?}`). T-10 and T-11 will map the generated types onto these.

Don't touch the router, views, stores, api/, or kiosk/.

1. **Design tokens** (`styles/base.css`):
   - colour, spacing, radius, and the type scale as CSS custom properties;
   - light and dark themes (`[data-theme]` + `prefers-color-scheme`);
   - a `.kiosk` scale (base 28 px) for viewing distance;
   - visible focus rings.

   Keep the existing M0 styles working.
2. **`lib/time.ts`** (no date library, `Intl` only, and every function takes the household `tz` explicitly):
   - `dayKey(instant, tz)`
   - `groupByDay(events, tz, from, days)`. All-day events use `start_date`/`end_date` (end exclusive) and span several days.
   - `startOfDay(tz, date)`
   - `msUntilNextMidnight(tz, now)` (for day rollover)
   - `formatTime`, `formatDayHeading`, and `formatRange` using the browser locale.
   - Must be correct across DST changes: test America/New_York in March and November, and Europe/London.
3. **`lib/units.ts`** + `composables/useUnits.ts`: convert metric to imperial (°C→°F, mm→in, cm→in, km/h→mph) and format with a unit suffix. Units come from a ref passed in.
4. **`lib/weatherCodes.ts`:** a WMO weather-code → `{label, icon, night_icon?}` table (all codes Open-Meteo returns: 0, 1–3, 45, 48, 51–57, 61–67, 71–77, 80–82, 85–86, 95–99). Icons are inline SVG components or a small sprite in `components/icons/`, with no external icon font.
5. **`lib/contrast.ts`:** `textColorFor(bg)` returns black or white for ≥ 4.5:1 contrast. Test it.
6. **Components:**
   - `MemberChip` (colour + name, with computed text colour)
   - `Avatar` (image or initials fallback, colour ring)
   - `EventCard` (title, time range, location, member chips, a hatched "Busy" style when `visibility === 'busy'`, and a 'from ↗' link when present)
   - `WeatherIcon`
   - `DateRangeNav`
   - `ErrorBanner` (including a "changed elsewhere, reload" variant for 409)

   Keep PinPad as it is. Each component has a vitest test.
7. **Composables:** `useClock(tickMs)`, `useRange(tz)` (current range + prev/next for day, week, and month), and `useLiveStream` **as an interface only**: export a `LiveEventKind` union matching Feeds.md §6 and an `invalidates(kind, data, heldRange)` pure function (the rules in Frontend.md §5 step 4). Wiring to the SSE client is T-11.
8. **Layout engine** (`dashboard/layout/`):
   - The `DashboardSettings` TS type mirrors Frontend.md §4.1.
   - Presets: "Family hub" (default), "Calendar wall", "Minimal".
   - `resolveLayout(settings, orientation) -> GridArea[]` uses a 12-column landscape grid (left 4: clock, weather, advisories; right 8: agenda or week) and stacks for portrait. Unknown panel kinds are dropped from rendering but kept in the data.
   - `burnInOffset(now)` gives a few-pixel shift every 10 min.
9. **PWA icons** (M0 follow-up): replace the placeholder `icon-192.png`/`icon-512.png` with a simple original icon (generate it from an SVG you write, and commit the SVG as well). Add a maskable variant.

## Acceptance
- [ ] `corepack pnpm@9 -C apps/web lint` passes
- [ ] `corepack pnpm@9 -C apps/web typecheck` passes
- [ ] `corepack pnpm@9 -C apps/web test` passes, including the DST cases
- [ ] `corepack pnpm@9 -C apps/web build` passes
- [ ] `corepack pnpm@9 -C apps/web format` has been run

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
