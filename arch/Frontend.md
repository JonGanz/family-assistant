---
title: Frontend
scope: Vue PWA structure, routes, dashboard (kiosk) design and layout config, settings screens, data flow (REST + SSE), offline behaviour
status: draft
updated: 2026-10-26
drawio: Frontend.drawio
drawio_src: src/Frontend.py
depends_on: [D-006, D-011, D-017, D-018, D-019, D-026, D-029, D-031, D-036, D-039, D-040, D-041, D-042, D-044, D-045, D-046, D-052, D-054, D-055, D-056, D-057, D-058, D-060, D-061, D-062, D-063, D-064]
milestone: M3
---

# Frontend (DRAFT v3)

This doc describes the Vue 3 PWA in `apps/web`: one app with two shells. The **app shell** is for signed-in people on phones and desktops. The **kiosk shell** is `/dashboard` on the Echo Show and other wall screens. This version covers what M1 builds. Later milestones add screens (tasks M2, inbox and labelling M3, pipelines and backends M4, meals M5). Backend data comes from [Feeds.md](Feeds.md).

## 1. Structure (page "Components")
```
apps/web/src/
├─ api/            http.ts (fetch wrapper, X-FA-Request, bearer opt-in), client.ts (typed calls over gen/)
├─ gen/            api.d.ts generated from openapi.json (`just gen`). The only source of API types
├─ stores/         Pinia: session, members, household, calendar, weather, sources, devices, dashboard
├─ composables/    useLiveStream (SSE → store invalidation), useUnits, useRange, useClock
├─ components/     shared UI: MemberChip, Avatar, PinPad, EventCard, WeatherIcon, DateRangeNav, ErrorBanner
├─ views/          app-shell screens (§3)
├─ dashboard/      kiosk panels + layout engine (§4)
├─ kiosk/          keepAlive, watchdog, deviceToken, sse (M0)
├─ layouts/        AppLayout (nav), KioskLayout (full-screen, no nav)
└─ router/         routes + guards (auth: user | adult | device)
```
- **API types:** one source only. `gen/api.d.ts` comes from the server's OpenAPI. The server derives it from the DTOs with `utoipa::ToSchema`, which removes M0's hand-written mirror schemas. The unused ts-rs output is dropped from the web app.
- **State:** stores fetch over REST and keep normalised data by ID. They never poll. They refresh when an SSE event invalidates their data (§5).
- **No UI framework.** Plain CSS with design tokens in `styles/base.css` (colour, spacing, type scale), plus a `kiosk` scale for viewing distance. That keeps the bundle small for Silk on Fire OS.
- **Dates:** `Intl.DateTimeFormat` in the household tz (from `/household`, not the browser's tz), with no date library. Recurrence is already expanded by the server.

## 2. Routes
| Path | Shell | Guard | M1 content |
|---|---|---|---|
| `/login` | app | none | M0 |
| `/` | app | user | **Today**: my agenda (or the whole household for adults), weather card, active advisories |
| `/calendar` | app | user | Agenda, week, and month views. Filter by member. Child users see only what §3.5 of Feeds.md allows |
| `/members` | app | user | M0 list + avatar upload and colour picker, plus interests, hobbies, and responsibilities editing |
| `/settings/household` | app | adult | name, tz, location (lat/lon + label, with a "use this device's location" button), units, calendar window, `private_events_as_busy` (D-036) |
| `/settings/calendars` | app | adult | sources (add Google / CalDAV / ICS, state, last sync, sync now, reconnect), calendars (enable, colour, members) |
| `/settings/weather` | app | adult | rules list, editor with live validation and preview, enable or disable |
| `/settings/devices` | app | adult | M0 pairing (plus a home-network link and QR code when the LAN dashboard is on, D-064) + **dashboard layout editor** (§4) |
| `/settings/status` | app | adult | System status: sources, last weather fetch, backup age, queue depth and dead jobs |
| `/oauth/done` | app | user | landing page after the Google callback. It shows the result and continues to `return_to` |
| `/dashboard` | kiosk | device | §4 |

Children see `/`, `/calendar`, and `/members` (public fields). Settings are adult-only in the nav, and the API enforces this as well.

## 3. App shell screens (M1)
- **Today:** a merged list of today's and tomorrow's events with member chips, the current weather plus the 7-day strip, and advisories at the top.
- **Calendar:**
  - The agenda view is the default on phones, and the week view on desktop. Month view is available.
  - Events are coloured by the first linked member's colour (or the calendar colour for household calendars), and multi-member events show stacked chips.
  - A busy-masked event shows as a hatched block labelled "Busy".
  - Each event has a 'from ↗' link to its source when the source gives a URL (Google `htmlLink`).
- **Source add flows:**
  - **Google:** the browser goes to `/api/v1/oauth/google/start`, which redirects to Google and back to `/oauth/done`. Then a calendar picker.
  - **CalDAV:** a form, then "Test & discover", then a calendar picker.
  - **ICS:** paste the URL, then "Test" (fetches once and shows the event count and the next 3 events).

  Credentials are write-only inputs and are never shown again (the URL is shown masked: `https://host/…`).
- **Weather rule editor:** a monospace condition field. The server validates it on input, with a 300 ms debounce, through `POST /weather-rules/validate`, which returns the error position or `{matches, value}` for the latest snapshot. A chip row inserts fields and windows.
- All mutating forms send `If-Match` versions and show a "changed elsewhere, reload" banner on a 409.

## 3a. App shell screens (M2, [Tasks.md](Tasks.md))
- **Tasks** (`/tasks`): tabs *Now* (D-042: "Mine" first, then "Household"), *Mine*, *Lists* (each list with its open tasks), and *Suggested* (adults, weather suggestions to accept or dismiss). Each item shows title, due (relative, overdue in red), assignee chips, a step count, and a claim button for pool tasks. Completing shows a 10-second **Undo** toast.
- **Task detail and editor:** title, notes, due, steps (add, reorder, tick), priority flag, and calendar display (`hidden`, `until done`, `always`; "key date" is the label for `always`, D-046). Children get the same editor for their own one-off tasks on lists that allow it (D-045).
- **Routines** (`/routines`): today's runs with a **progress bar** per member (D-041). Children see their own.
- **Settings → Tasks** (adults): lists (policy, flags, calendar default, claim time), definitions (schedule `every N days after done` or `repeat on…` with an RRULE builder for weekly/monthly, lead days, due time, missed behaviour, strategy and ordered candidates, rotation options, priority, season window and condition with the same validation as the weather rule editor, pause), routines (members, window, steps with target times, day filter, school calendar and keywords, dashboard toggle), and **absences** (member, from, until).
- **Settings → Notifications:** per member (self, plus children for adults): trigger toggles, due-soon and nudge minutes, quiet-hours override, copy-to adult, the subscribe URL with a QR code, and "Send test". When the server has no ntfy URL, the screen says notifications are not configured.
- **Settings → Household** gains the Now list size and weights, and the default quiet hours.
- **Calendar page:** tasks with `until_done` or `always` show as task chips (checkbox and member colour) on their due day; `always` stays after completion, shown as done (D-046).

## 3b. App shell screens (M3, [Ingest.md](Ingest.md))
New routes (adult guard unless noted): `/mail`, `/mail/:id`, `/mail/label`, `/inbox`, `/bills`, `/settings/mail`, `/settings/orgs`. Children get none of them (D-063); they see derived events and tasks in the existing screens.
- **Mail** (`/mail`): messages from visible sources (D-063), filtered by category, member, source and state; ignored categories hidden behind a toggle. Each row: sender, subject, category chip, member chips, time-sensitive marker.
- **Message** (`/mail/:id`): metadata, body (when content is kept), and for a child filter the relevant and general sections first with the others collapsed (D-055); the decisions with one-tap "Fix"; derived items; "Make task"; mail actions with an Undo list for 30 days (D-058; hidden on read-only sources); "open in Gmail ↗" when an external link exists.
- **Inbox** (`/inbox`): tabs for events, bills, tasks and contacts (D-056, D-057, D-060). Events show the suggested calendar with a picker and editable title and time; tasks show assignees and due; contacts show org, child and the triggering message. Approve and reject buttons, a nav badge with the count, and SSE `inbox.updated`.
- **Labelling** (`/mail/label`, D-062): phone-first, one message per screen, the four decision points pre-filled as chips (one tap confirms all, tapping a chip fixes it), "in eval sample" toggle, progress toward ~200, and "next" from the suggested balanced queue.
- **Bills** (`/bills`): open, proposed and paid bills with payee, amount, due date and the pay task's state (D-059).
- **Settings → Mail:** sources (add **Gmail** via Google sign-in or **IMAP** with host, port, username and app password plus a "Test connection" button, D-052; owners, visibility, read-only, backfill, filter, folders, poll, state and reconnect), categories (behaviour, notify, auto actions, email tasks; D-054, D-058, D-060), rules (condition builder with a dry run on recent mail), and household mail settings (event auto-write off by default with its thresholds, inbox expiry, gender term lists; D-055, D-057, D-061).
- **Settings → Organizations:** orgs (kind, domains, senders, ICS feed, default calendar, contact mode), contacts, and enrollments per child.
- **Member editor** gains aliases, and for children an optional gender and relevance tags (D-055).
- **Settings → Notifications** gains the `approval_needed` trigger (batched by default, immediate, digest only, off; D-061) and `mail_time_sensitive`.

## 4. Dashboard (kiosk shell)
### 4.1 Layout model
The layout is data held in `device.settings` and edited by adults in Settings → Devices. It is never edited on the dashboard itself (D-019).
```json
{ "orientation": "landscape" | "portrait",
  "theme": "auto" | "light" | "dark",       // auto = dark between sunset and sunrise (from weather)
  "agenda_days": 3,
  "members": "all" | ["<member-id>", ...],
  "panels": [ {"kind": "clock"}, {"kind": "weather", "hourly": true},
              {"kind": "advisories"}, {"kind": "agenda"}, {"kind": "week"}, {"kind": "members"} ] }
```
- **Panels in M2:** `now` (the Now list, D-042, with optional `members` and `lists` filters), `member_tasks` (one column per member with their open tasks for today), and `routine` (D-041: shown automatically during a routine window, one column per member with large step buttons and a progress bar; `pinned: true` keeps it visible outside the window). `agenda` also shows key-date tasks (`always`), and `week` shows `until_done` and `always` task chips (D-046). Completing anything on the dashboard opens the member picker ("who did it?", preselecting the assignee) and needs no PIN while the override is on (D-013, D-019).
- **Panel in M3:** `inbox` shows the pending count; when the device is elevated with an adult PIN (D-019), it lists the items with approve and reject.
- **Panels in M1:** `clock` (time, date), `weather` (now, the next 12 h, and 5–7 days), `advisories` (coloured by severity, dismissable without a PIN), `agenda` (today plus the next N days, grouped by day, with member chips), `week` (a 7-day column grid), `members` (avatars, which is also the entry point for M2 per-member task views and the PIN step-up).
- **Presets** ("Family hub", "Calendar wall", "Minimal") fill `panels`. Unknown panel kinds are ignored, so an older client survives a newer layout.
- **Landscape 1920×1080:** a 12-column grid. The left column (4) holds clock, weather, and advisories, and the right (8) holds agenda or week. **Portrait** stacks them. Type is sized for 2–3 m (base 28 px on the kiosk scale).
- The layout editor shows a live, scaled preview of the dashboard with the edited settings.

### 4.2 Data & resilience
- **One fetch:** `GET /api/v1/dashboard?days=N` returns household (name, tz, units), members (public fields + avatar URL), events in the window (already masked), the latest weather, active advisories, and the device settings. Refreshes reuse it.
- **M2 data:** `GET /api/v1/dashboard` gains `tasks[]` (open tasks visible now, plus calendar-displayed tasks in the window) and `routines[]` (today's runs with steps). SSE `tasks.updated` and `routines.updated` trigger a refetch of those parts.
- **Live:** SSE kinds update the snapshot: `calendar.updated` (refetch when its range overlaps), `weather.updated`, `advisory.updated`, `device.updated`, `member.updated`, `household.updated`, and `resync` (full refetch).
- **Offline:** the last snapshot is saved to localStorage (extending the M0 cache) with `savedAt`. While the stream is down, the dashboard shows the cache with a quiet "last updated hh:mm" badge. The service worker caches the app shell and avatar images.
- **Day rollover:** at midnight in the household tz, the dashboard refetches. The nightly reload (D-011) stays as a backstop.
- **Burn-in:** in dark theme the content shifts by a few pixels every 10 minutes. The M0 keep-alive is unchanged.

## 5. Live data flow (page "Live update")
1. The worker commits a change and calls `ChangeBus::publish` (`pg_notify`) inside the transaction.
2. The api's `PgListener` receives it and pushes it into the `Hub`, which gives it an SSE id.
3. `/api/v1/stream` sends it to every connected client of that household.
4. The `useLiveStream` composable maps the event kind to the affected stores. Each store refetches only if it holds overlapping data.
5. If the stream disconnects, the client reconnects with `Last-Event-ID`. If the hub can't replay that far, it sends `resync`.

## 6. Accessibility & i18n
- Every interactive element is reachable by keyboard, with visible focus rings. Touch targets are at least 48 px on the kiosk.
- Member colours keep at least 4.5:1 contrast for chip text: text colour is computed from the background.
- Strings are English and live in one `strings.ts` per area, which makes later i18n possible but isn't built now. Dates and numbers use `Intl` with the browser locale and the household tz.

## 7. Testing
- **vitest:**
  - stores (invalidation logic);
  - the layout engine (presets, unknown panels);
  - unit conversion;
  - day grouping across DST changes, using a fixed tz.
- **Playwright** against the e2e stack, which has a seeded household, an ICS fixture, and an Open-Meteo stub:
  - the dashboard renders the agenda, weather, and advisory;
  - an SSE update appears without a reload;
  - a child login sees only permitted events;
  - the settings flows for ICS and weather rules work.
