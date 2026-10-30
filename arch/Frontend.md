---
title: Frontend
scope: Vue PWA structure, routes, first-run wizard, settings structure, dashboard (kiosk) design and layout config, data flow (REST + SSE), offline and insecure-context behaviour
status: draft
updated: 2026-10-29
drawio: Frontend.drawio
drawio_src: src/Frontend.py
depends_on: [D-006, D-011, D-017, D-018, D-019, D-026, D-029, D-031, D-036, D-039, D-040, D-041, D-042, D-044, D-045, D-046, D-052, D-054, D-055, D-056, D-057, D-058, D-060, D-061, D-062, D-063, D-064, D-065, D-068, D-070, D-072, D-073, D-074, D-075, D-078, D-079, D-080, D-081]
milestone: M3.5
---

# Frontend (DRAFT v4)

This doc describes the Vue 3 PWA in `apps/web`: one app with three shells. The **setup shell** is the first-run wizard (`/setup`, D-065). The **app shell** is for signed-in people on phones and desktops. The **kiosk shell** is `/dashboard` on the Echo Show and other wall screens. v4 (M3.5) adds the wizard, the grouped Settings area with Settings, System, moves Members into Settings, makes the dashboard at-a-glance only, and makes the app work over plain HTTP. Backend data comes from [Feeds.md](Feeds.md), [Tasks.md](Tasks.md) and [Ingest.md](Ingest.md); the runtime settings and setup API are in [Platform.md](Platform.md).

## 1. Structure (page "Components")
```
apps/web/src/
├─ api/            http.ts (fetch wrapper, X-FA-Request, bearer opt-in), client.ts (typed calls over gen/)
├─ gen/            api.d.ts generated from openapi.json (`just gen`). The only source of API types
├─ stores/         Pinia: setup, session, members, household, calendar, weather, sources, devices, dashboard, tasks, mail, inbox, system
├─ composables/    useLiveStream (SSE → store invalidation), useUnits, useRange, useClock, useSecureContext
├─ components/     shared UI: PageHeader, StateBlock, MemberChip, Avatar, PinPad, EventCard, WeatherIcon, DateRangeNav, ErrorBanner, Tabs
├─ themes/         theme catalogue as data (D-079): one file per family, the token set, `applyTheme`, the device's saved choice (§3d)
├─ lib/            platform shims: ids (randomId), copy (clipboard fallback), pwa (conditional service worker)
├─ setup/          the first-run wizard: SetupLayout + one component per step (§2a)
├─ views/          app-shell screens (§3); settings/ holds SettingsLayout, SettingsIndexView and the settings pages
├─ dashboard/      kiosk panels + layout engine (§4)
├─ kiosk/          keepAlive, watchdog, deviceToken, sse (M0)
├─ layouts/        SetupLayout, AppLayout (nav), KioskLayout (full-screen, no nav)
└─ router/         routes + guards (setup | user | adult | device)
```
- **API types:** one source only. `gen/api.d.ts` comes from the server's OpenAPI, which the server derives from its DTOs (`utoipa::ToSchema`).
- **State:** stores fetch over REST and keep normalised data by ID. They never poll. They refresh when an SSE event invalidates their data (§5).
- **No UI framework.** Plain CSS with design tokens and the UI kit in `styles/base.css` (colour tokens come from the active theme, §3d; spacing, type scale, `.form`, `.card-list`, `.badge`), plus a `kiosk` scale for viewing distance. That keeps the bundle small for Silk on Fire OS.
- **Dates:** `Intl.DateTimeFormat` in the household tz (from `/household`, not the browser's tz), with no date library. Recurrence is already expanded by the server.

### 1.1 Secure and insecure contexts (D-072)
The whole app must work over plain HTTP on a LAN address, where the browser is in an **insecure context**:
- **Service worker** (vite-plugin-pwa) is registered only when `window.isSecureContext`. Without it the app loads normally from the network; the PWA install hint is hidden. The offline dashboard relies on the localStorage snapshot (§4.2), not on the service worker.
- **No `crypto.randomUUID`, no `crypto.subtle`:** IDs that the client makes come from `lib/ids` (`crypto.getRandomValues`, available in both contexts). The client never hashes or encrypts.
- **No async clipboard:** `lib/copy` uses `navigator.clipboard` when available, else a selected read-only text field with a "press Ctrl+C" hint.
- **No nag banners.** Plain HTTP is the hoster's choice. Settings, System states the connection type as a fact (§3c); the login page shows nothing about it.
- **Google sign-in needs HTTPS** (Google rejects `http://` redirect URIs except localhost). On an insecure context, Settings, Calendars and Settings, Mail show Google as **"needs HTTPS"**, with a one-line pointer to the Tailscale profile, instead of hiding it. IMAP, CalDAV and ICS work the same in both.
- e2e runs the UI suite over plain HTTP from a non-localhost address so that a secure-context-only API fails the build.

## 2. Routes
| Path | Shell | Guard | Content |
|---|---|---|---|
| `/setup` | setup | setup | First-run wizard (§2a). Only while setup is needed |
| `/login` | app | none | Username and password |
| `/` | app | user | **Today**: my agenda (or the whole household for adults), weather card, active advisories, my Now list |
| `/calendar` | app | user | Agenda, week, and month views. Filter by member. Child users see only what §3.5 of Feeds.md allows |
| `/tasks`, `/tasks/:id`, `/routines` | app | user | §3a |
| `/mail`, `/mail/:id`, `/mail/label`, `/inbox`, `/bills` | app | adult | §3b |
| `/settings` | app | user | **Settings index** (§3c): grouped cards; children see only the Personal group |
| `/settings/<page>` | app | per page | Child routes rendered inside `SettingsLayout` (§3c) |
| `/oauth/done` | app | user | Landing page after the Google callback. It shows the result and continues to `return_to` |
| `/dashboard` | kiosk | device | §4 |

- **Setup guard:** the router asks `GET /api/v1/setup` once at start. While `needed` is true, **every route redirects to `/setup`**, and any API call answering `409 setup_required` does the same. Once setup is done, `/setup` redirects to `/`.
- **No `/members` route** and no redirect from it (D-070): member management is `/settings/members`, adults only.
- Nested pages mark their parent nav item active by path prefix (`/tasks/:id` → Tasks, `/settings/*` → Settings).
- The API enforces every guard as well; the router guard only shapes navigation.

## 2a. First-run wizard (setup shell, D-065, D-073, D-074)
Shown when no household exists. It is **open to whoever opens the app first** (D-073): no setup code, no address check. Setup closes for good once the first adult account exists; the hoster is told to finish it right after `docker compose up`.

| Step | Content | Writes |
|---|---|---|
| 1. Welcome | The detected address (`location.origin`), whether it is HTTPS, whether the Tailscale sidecar is up and its tailnet address | nothing |
| 2. Household | Name, time zone (browser default), location (city search through the Open-Meteo geocoding API, or lat/lon by hand, or "use this device's location" when secure), units, currency | `POST /setup/household` |
| 3. You | The first adult: display name, colour, username, password (12+), optional PIN. Signs in; the rest of the wizard runs as that adult | `POST /setup/owner` |
| 4. Family | Add adults and children (role, display name, colour, optional grade and aliases), each with an optional login and PIN. Reuses the member editor components | existing `/api/v1/members` routes |
| 5. Access and notifications | Public address(es) (prefilled from step 1; LAN and tailnet both kept when the sidecar is up), notifications on or off with "Send a test" | `PATCH /system/settings` |
| 6. Master key backup | The key as text with **Copy** and **Download** buttons; continuing needs "I stored it somewhere safe" ticked (D-074) | `POST /system/master-key/confirm` |
| 7. Start | Starter chores and routines (D-047), or **Import a household file** (TOML upload with a dry-run preview, D-018). Then links to Calendars, Mail and Devices (pair the wall screen) | existing routes |

- Each step saves when you press Next; `GET /setup` returns the step to resume at, so a reload or a second device continues where it stopped.
- Back keeps values; steps 4–7 can be skipped and done later in Settings.
- A failed step shows the server's field errors inline (`.form` errors), as in Settings.

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
- **Inbox** (`/inbox`): tabs for events, bills, tasks and contacts (D-056, D-057, D-060). Events show the suggested calendar with a picker and editable title and time; tasks show assignees and due; contacts show org, child and the triggering message. Approve and reject buttons, a nav badge with the count, and SSE `inbox.updated`. Each adult sees only items from sources they may see (owners, or every adult for `household` sources); the server filters, the UI has no other view (D-078).
- **Labelling** (`/mail/label`, D-062): phone-first, one message per screen, the four decision points pre-filled as chips (one tap confirms all, tapping a chip fixes it), "in eval sample" toggle, progress toward ~200, and "next" from the suggested balanced queue.
- **Bills** (`/bills`): open, proposed and paid bills with payee, amount, due date and the pay task's state (D-059).
- **Settings → Mail:** sources (add **Gmail** via Google sign-in or **IMAP** with host, port, username and app password plus a "Test connection" button, D-052; owners, visibility, read-only, backfill, filter, folders, poll, state and reconnect), categories (behaviour, notify, auto actions, email tasks; D-054, D-058, D-060), rules (condition builder with a dry run on recent mail), and household mail settings (event auto-write off by default with its thresholds, inbox expiry, gender term lists; D-055, D-057, D-061).
- **Settings → Organizations:** orgs (kind, domains, senders, ICS feed, default calendar, contact mode), contacts, and enrollments per child.
- **Member editor** gains aliases, and for children an optional gender and relevance tags (D-055).
- **Inbox** is an app screen only: approvals never appear on the dashboard (D-068).
- **Settings → Notifications** gains the `approval_needed` trigger (batched by default, immediate, digest only, off; D-061) and `mail_time_sensitive`.

## 3c. Settings area (M3.5, U-08, D-070, D-075)
`/settings` is a parent route with `SettingsLayout`: a sidebar on desktop, a chip row on phones. Its index (`SettingsIndexView`) shows grouped cards. Every page keeps its own guard.

| Group | Pages (path under `/settings/`) | Guard |
|---|---|---|
| Personal | `account` (password, sessions), `appearance` (this device's theme, §3d), `notifications` (own, plus children's for adults; §3a) | user |
| Household | `household` (name, tz, location, units, currency, calendar window, Now list weights, quiet hours, dashboard pending-approvals badge), **`members`** (add, edit, avatar, hue, role, account and password reset, PIN, aliases, gender and relevance tags, interests, hobbies and responsibilities), `tasks` (lists, definitions, routines, absences) | adult |
| Sources and rules | `calendars`, `weather`, `mail`, `orgs` | adult |
| System | `system` (below), `devices` (pairing + dashboard layout editor, §4), `status`, `about` (server version, commit and build time, next to the same for this page's bundle; a banner when the two commits differ, which means the browser is showing a stale cached app) | adult |

- **Members** (D-070): adults only, since other features rely on member details. Children see members on the dashboard's `members` strip, not in the app. A child sees only the Personal group.
- **Settings, System** (D-075, D-074, D-072):
  - **Runtime settings:** public address(es), ntfy URL and token, Google client ID and secret, plain-HTTP feeds, private hosts for feeds, the home-network dashboard listener (D-064), trusted proxies (D-069). Each field shows its source: editable when stored in the DB; **locked with "set by the server"** when an environment variable overrides it. Secrets are write-only ("set" / "not set", replace, clear). Fields that need a restart (listeners) say so after saving.
  - **Connection:** a plain statement such as "This page is served over HTTP" or "HTTPS via Tailscale (`https://family.<tailnet>.ts.net`)". No warning styling.
  - **Master key backup:** a warning card until an adult has confirmed a backup; it reopens the step-6 Copy/Download flow (needs a recent login or elevation).
  - **Household file:** **Export** downloads TOML (no secrets). **Import** uploads TOML, shows a dry-run diff (created, changed, skipped, errors with positions), then applies on confirmation (D-018).

## 3d. Themes (D-079, D-080, D-081)
**Catalogue.** `themes/` holds one data file per family: Catppuccin, Nord, Dracula, Gruvbox, Solarized, Tokyo Night, Rosé Pine, Kanagawa. A family lists the light and/or dark variants its project publishes (Catppuccin has Latte and three dark flavours). Every variant fills the same tokens:
- **neutrals:** `bg`, `surface`, `surface-2`, `border`, `muted`, `text`, `on-accent` (text on a hue-coloured fill);
- **hues:** `red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `pink`. A palette without a native hue supplies its nearest colour.

`applyTheme` writes the tokens as CSS custom properties on `<html>` (`--color-bg`, `--color-surface`, `--color-text`, `--hue-red`…). The existing names in `base.css` stay as aliases, so components keep working: `--color-accent` is the `blue` hue, and `--color-danger`, `--color-warning` and `--color-success` are `red`, `yellow` and `green`. Source and licence of each palette are recorded next to its file.

**Mode.** A device's choice is `{family, variant?, mode}` with `mode` one of `auto | light | dark`.
- A family with **both** light and dark variants follows the mode. In the app, `auto` follows the device's `prefers-color-scheme`. On the kiosk, `auto` is dark between sunset and sunrise (from the weather data).
- A family with **one** variant always uses it, at any time of day. The mode picker is hidden for it.
- A family with several variants in one mode (Catppuccin dark) lets the user pick the variant.

**Where the choice lives** (D-081; there is no household setting):
- **App devices:** `localStorage` key `fa.theme`, edited in **Settings → Personal → Appearance** by any signed-in member. It does not follow a person to another device. Until a choice is made the app uses its shipped default (a technical choice: Catppuccin, `auto`).
- **Kiosk and dashboard devices:** `device.settings.theme`, set by an adult in Settings → Devices (the layout editor's live preview uses it). The kiosk uses the same catalogue and never picks a theme itself (D-019).

**Member colours** (D-080). A member's `color` is a hue name. The client uses `var(--hue-<name>)` for chips, the agenda and task items. `eventColors` falls back from the event's own calendar colour (still hex, from outside) to the member's hue. The member picker shows the current theme's swatches.

**Contrast** (build-time unit test over every variant): `text` and `muted` on `bg` and on both surfaces at least 4.5:1; each hue on `bg` at least 3:1 (stripes, chips, accents); `on-accent` on `blue`. A failing value is corrected in the theme data, never adjusted at run time.

## 4. Dashboard (kiosk shell)
The dashboard is **at-a-glance only** (D-068): date and time, weather with advisories, agenda, tasks, routines, and the household's members. **Nothing derived from email** appears on it: no inbox panel, no titles, senders or approvals, elevated or not. The one exception is a **pending-approvals count badge per adult** on the `members` strip (D-078), which the household setting `dashboard_pending_approvals` (default on) can turn off.

### 4.1 Layout model
The layout is data held in `device.settings` and edited by adults in Settings → Devices. It is never edited on the dashboard itself (D-019).
```json
{ "orientation": "landscape" | "portrait",
  "theme": { "family": "<id>", "variant": "<id>?", "mode": "auto" | "light" | "dark" },  // §3d; auto = dark between sunset and sunrise (from weather)
  "agenda_days": 3,
  "members": "all" | ["<member-id>", ...],
  "panels": [ {"kind": "clock"}, {"kind": "weather", "hourly": true},
              {"kind": "advisories"}, {"kind": "agenda"}, {"kind": "week"}, {"kind": "members"} ] }
```
- **Panels:**
  - `clock` (time, date);
  - `weather` (now, the next 12 h, and 5–7 days);
  - `advisories` (coloured by severity). **Not dismissable** by anyone: an advisory disappears when its `valid_until` passes or when the server finds its rule no longer matches the forecast (D-068), arriving as SSE `advisory.updated`;
  - `agenda` (today plus the next N days, grouped by day, with member chips and key-date tasks, D-046);
  - `week` (a 7-day column grid, with `until_done` and `always` task chips);
  - `members` (avatars and names of the household, D-070; the entry point for per-member task views and the PIN step-up; adults carry a count-only "to approve" badge when `dashboard_pending_approvals` is on, D-078);
  - `now` (the Now list, D-042, with optional `members` and `lists` filters), `member_tasks` (one column per member with today's open tasks), `routine` (D-041: shown automatically during a routine window with large step buttons and a progress bar; `pinned: true` keeps it visible).
- Completing anything on the dashboard opens the member picker ("who did it?", preselecting the assignee) and needs no PIN while the override is on (D-013, D-019). PIN elevation remains for adult actions on tasks (D-019); it is not used for approvals.
- **Removed in v4:** the `inbox` panel (D-068). A layout that still names it is ignored like any unknown kind, so older saved layouts keep working.
- **Presets** ("Family hub", "Calendar wall", "Minimal") fill `panels`. Unknown panel kinds are ignored, so an older client survives a newer layout.
- **Landscape 1920×1080:** a 12-column grid. The left column (4) holds clock, weather, and advisories, and the right (8) holds agenda or week. **Portrait** stacks them. Type is sized for 2–3 m (base 28 px on the kiosk scale, one kiosk font system).
- The layout editor shows a live, scaled preview of the dashboard with the edited settings.

### 4.2 Data & resilience
- **One fetch:** `GET /api/v1/dashboard?days=N` returns household (name, tz, units), members (public fields + avatar URL), events in the window (already masked), the latest weather, active advisories, `tasks[]` (open tasks visible now, plus calendar-displayed tasks in the window), `routines[]` (today's runs with steps), and the device settings. Refreshes reuse it. It carries no mail-derived data.
- **Live:** SSE kinds update the snapshot: `calendar.updated` (refetch when its range overlaps), `weather.updated`, `advisory.updated`, `tasks.updated`, `routines.updated`, `device.updated`, `member.updated`, `household.updated`, and `resync` (full refetch).
- **Offline:** the last snapshot is saved to localStorage with `savedAt`. While the stream is down, the dashboard shows the cache with a quiet "last updated hh:mm" badge. Where a service worker runs (secure contexts only, §1.1) it also caches the app shell and avatar images; over plain HTTP the snapshot is the offline story.
- **Access paths:** the dashboard works on the main listener (HTTP or HTTPS, D-072) and on the optional restricted home-network listener (D-064); pairing shows a link and QR code for each available address.
- **Day rollover:** at midnight in the household tz, the dashboard refetches. The nightly reload (D-011) stays as a backstop.
- **Burn-in:** whenever the active variant is dark the content shifts by a few pixels every 10 minutes. The M0 keep-alive is unchanged.

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
  - the settings flows for ICS and weather rules work;
  - **first run** (M3.5): an empty database redirects every route to `/setup`; the wizard creates the household, owner, a child and confirms the key backup; `/setup` is then closed;
  - the whole suite runs over plain HTTP from a non-localhost address (insecure context, §1.1);
  - Settings, System shows env-set fields as locked; TOML import dry-run then apply;
  - the dashboard has no advisory dismiss control and no inbox panel; a child sees no `/settings/members`.
