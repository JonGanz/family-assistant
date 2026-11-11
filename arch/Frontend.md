---
title: Frontend
scope: Vue PWA structure, design system and navigation, routes, Food views (M5), first-run wizard, settings structure, dashboard (kiosk) design and layout config, data flow (REST + SSE), offline and insecure-context behaviour
status: draft
updated: 2026-11-08
drawio: Frontend.drawio
drawio_src: src/Frontend.py
depends_on: [D-006, D-011, D-017, D-018, D-019, D-026, D-029, D-031, D-036, D-039, D-040, D-041, D-042, D-044, D-045, D-046, D-052, D-054, D-055, D-056, D-057, D-058, D-060, D-061, D-062, D-063, D-064, D-065, D-068, D-070, D-072, D-073, D-074, D-075, D-078, D-079, D-080, D-081, D-093, D-094, D-095, D-096, D-097, D-098, D-099, D-100, D-101, D-102, D-103, D-104, D-105, D-106, D-109, D-110, D-111, D-112, D-115, D-116, D-117, D-118, D-120, D-121, D-122, D-123]
milestone: M5
---

# Frontend (DRAFT v5.2, M5 Food views)

This doc describes the Vue 3 PWA in `apps/web`: one app with three shells. The **setup shell** is the first-run wizard (`/setup`, D-065). The **app shell** is for signed-in people on phones and desktops. The **kiosk shell** is `/dashboard` on the Echo Show and other wall screens. v4 (M3.5) adds the wizard, the grouped Settings area with Settings, System, moves Members into Settings, makes the dashboard at-a-glance only, and makes the app work over plain HTTP. v5 (M4, D-095 to D-097) adds the design system (`ui/`, §1.2), the adaptive navigation (§1.3), one rule for applying changes (§1.4), list-detail Settings, and the Settings → AI screen with the inbox's AI provenance (§3e). v5.2 (M5, D-109 to D-118) adds the **Food** destination (meal plan, shopping list, recipes, templates, §3f), Settings → Meals, the kiosk Food view and `meals` panel (§4), and the grocery offline queue (§1.1). Backend data comes from [Feeds.md](Feeds.md), [Tasks.md](Tasks.md), [Ingest.md](Ingest.md) and [Meals.md](Meals.md); the runtime settings and setup API are in [Platform.md](Platform.md).

## 1. Structure (page "Components")
```
apps/web/src/
├─ api/            http.ts (fetch wrapper, X-FA-Request, bearer opt-in), client.ts (typed calls over gen/)
├─ gen/            api.d.ts generated from openapi.json (the gen commands in the README). The only source of API types
├─ stores/         Pinia: setup, session, members, household, calendar, weather, sources, devices, dashboard, tasks, mail, inbox, system, mealPlan, grocery, recipes, mealsSettings
├─ composables/    useLiveStream (SSE → store invalidation), useUnits, useRange, useClock, useSecureContext
├─ ui/             the design system (D-095, §1.2): tokens.css, Ui* components, icons.ts, composables (useBreakpoint, useSnackbar, useInstantSave, useFormDraft), dev-only UiGallery
├─ app/            navigation.ts: the destinations list every navigation component renders (§1.3)
├─ components/     domain components built on ui/: MemberChip, Avatar, PinPad, HuePicker, EventCard, WeatherIcon, DateRangeNav, tasks/, mail/, food/ (MealPlanGrid, MealCell, GroceryList, RecipeCard, RatingStars, TemplateGrid; shared by app and kiosk)
├─ themes/         theme catalogue as data (D-079): one file per family, the token set, `applyTheme`, the device's saved choice (§3d)
├─ lib/            platform shims: ids (randomId), copy (clipboard fallback), pwa (conditional service worker), offlineQueue (grocery writes, §1.1)
├─ setup/          the first-run wizard: SetupLayout + one component per step (§2a)
├─ views/          app-shell screens (§3); settings/ holds SettingsLayout, SettingsIndexView and the settings pages
├─ dashboard/      kiosk panels + layout engine (§4)
├─ kiosk/          keepAlive, watchdog, deviceToken, sse (M0)
├─ layouts/        SetupLayout, AppLayout (adaptive navigation, snackbar host), KioskLayout (full-screen, no nav)
└─ router/         routes + guards (setup | user | adult | device)
```
- **API types:** one source only. `gen/api.d.ts` comes from the server's OpenAPI, which the server derives from its DTOs (`utoipa::ToSchema`).
- **State:** stores fetch over REST and keep normalised data by ID. They never poll. They refresh when an SSE event invalidates their data (§5).
- **No UI framework** (D-095). The app shell uses the home-grown component library in `ui/` (§1.2), whose colours all come from the active theme (§3d). `styles/base.css` keeps the theme aliases, resets and the `kiosk` scale for viewing distance. That keeps the bundle small for Silk on Fire OS.
- **Dates:** `Intl.DateTimeFormat` in the household tz (from `/household`, not the browser's tz), with no date library. Recurrence is already expanded by the server.

### 1.0 Picking up a new version (D-123)
After a deploy, open clients must not keep running the old app, in either context:
- **Caching:** the server sends `Cache-Control: no-cache` for `index.html`, `sw.js`, `registerSW.js` and `manifest.webmanifest`; `public, max-age=31536000, immutable` for the hashed `/assets/*`; `max-age=3600` for other static files (Platform.md). A reload therefore always gets the current shell, with or without a service worker.
- **Detection:** every API response carries `X-FA-Build: <commit>`, and `/api/v1/stream` sends a `build` event `{commit}` on connect. `lib/appUpdate.ts` compares it with `__APP_COMMIT__`. An unknown build on either side, a prefix match (short and long hashes), and the Vite dev server all mean "same".
- **App shell:**
  - a newer build shows a snackbar "A new version is available · Reload";
  - the app reloads by itself on the next return to the foreground (`visibilitychange` to visible) or the next route change, unless a form has unsaved changes (`useFormDraft` keeps a global dirty count);
  - before reloading in a secure context, it asks the service worker to update (`registration.update()`) and waits for the new worker to take control (at most 5 s).
- **Kiosk:** reloads by itself when no dialog is open and the device is not elevated, or as soon as both are true.
- **Loop guard:** an automatic reload is tried once per server build and tab (`sessionStorage` `fa.update.tried`). If the mismatch survives it (a web build older than the server, say), only the snackbar's manual Reload remains.
- **Checks:** in a secure context the service worker is asked to `update()` on every return to the foreground and hourly.
- **Escape hatch:** Settings → About has **Reload app**. It unregisters nothing, deletes the Workbox precache and runtime caches except `fa-grocery`, and reloads. The offline queue (`fa.grocery.queue`) is kept.

### 1.1 Secure and insecure contexts (D-072)
The whole app must work over plain HTTP on a LAN address, where the browser is in an **insecure context**:
- **Service worker** (vite-plugin-pwa) is registered only when `window.isSecureContext`. Without it the app loads normally from the network; the PWA install hint is hidden. The offline dashboard relies on the localStorage snapshot (§4.2), not on the service worker. In a secure context the service worker also caches the app shell and the last shopping list, so the Food view opens with no connection (D-118).
- **Grocery offline queue (D-118), both contexts:** an open Shopping List keeps working without a connection. Adds and check-offs go to `lib/offlineQueue` (localStorage `fa.grocery.queue`) and replay in order when the connection returns. A queued add carries a client-made item id from `lib/ids`, so a replay is idempotent; a check-off wins over a concurrent edit; a 404 (item deleted elsewhere) drops that item's queued changes. A note "Offline, N changes waiting" shows above the list until the queue is empty. Deleting needs a connection.
- **No `crypto.randomUUID`, no `crypto.subtle`:** IDs that the client makes come from `lib/ids` (`crypto.getRandomValues`, available in both contexts). The client never hashes or encrypts.
- **No async clipboard:** `lib/copy` uses `navigator.clipboard` when available, else a selected read-only text field with a "press Ctrl+C" hint.
- **No nag banners.** Plain HTTP is the hoster's choice. Settings, System states the connection type as a fact (§3c); the login page shows nothing about it.
- **Google sign-in needs HTTPS** (Google rejects `http://` redirect URIs except localhost). On an insecure context, Settings, Calendars and Settings, Mail show Google as **"needs HTTPS"**, with a one-line pointer to the Tailscale profile, instead of hiding it. IMAP, CalDAV and ICS work the same in both.
- e2e runs the UI suite over plain HTTP from a non-localhost address so that a secure-context-only API fails the build.

### 1.2 Design system (D-095)
A home-grown Vue component library in `apps/web/src/ui/`, modelled on Material 3 patterns and used by every app-shell and setup screen. The kiosk dashboard keeps its own look (§4) and reuses a `ui/` component only where nothing changes visually.
- **Tokens** (`ui/tokens.css`), loaded after `styles/base.css`:
  - **Colour** is never a literal. Every colour token derives from the theme variables from §3d. The surface and its five container tones (`--ui-surface-container-{lowest,low,mid,high,highest}`) map onto the theme's **four layers** (D-106, T-21): page = `--color-layer-0` (`bg`), card = `--color-layer-1`, raised = `--color-layer-2`, dialog or sheet = `--color-layer-3`. A variant without extended slots gets the layers from `color-mix` steps of `bg` toward `text` (5, 10 and 15 %; 3, 6 and 9 % for Seoul256). Secondary text on a container uses that layer's `--color-on-layer-N-subtle` (§3d). `--color-comment` is used only for placeholders and disabled text. `::selection` and list hover use `--color-selection` and `--color-highlight`. The rest stay `color-mix()` of the theme variables: on-surface and its variant, outline and its variant, primary (the `blue` hue), on-primary, primary container, error, and the state layers (hover 8 %, focus and pressed 12 %). Every theme in the catalogue therefore gets the same tonal structure.
  - **Spacing** is a 4 px grid (`--ui-space-1..8`).
  - **Shape** is `--ui-radius-xs..xl` plus `full` (4, 8, 12, 16, 28 px, pill), with `none` (0) for **events inside calendar grids, which are square** (R1 review, 2026-11-04). Cards default to `filled`.
  - **Density** is per device (`data-density` on `<html>`, `localStorage` `fa.density`), with **comfortable** (Material 3 sizes) as the default and **compact** as the option (R1). It is set in Appearance next to the theme.
  - **Icons** are Material Symbols Rounded; the active navigation destination shows the filled glyph (R1).
  - **Elevation** is `--ui-elev-0..3`: shadows plus a surface tint, readable in dark themes.
  - **Type scale** has display, headline, title, body and label, each in lg, md and sm.
  - **Motion** has short and medium durations with one easing; the durations drop to 0 under `prefers-reduced-motion`.
- **Breakpoints** follow the Material window classes: **compact** < 600 px, **medium** 600–839 px, **expanded** ≥ 840 px (`useBreakpoint()`). Layouts are written mobile-first: compact is the base and wider classes add columns.
- **Components** (`Ui*.vue`, exported from `ui/index.ts`):
  - **Layout:** `UiPage` (scaffold with top-bar slot and a content max-width), `UiTopAppBar`, `UiSection`.
  - **Surfaces:** `UiCard` (elevated, filled or outlined; media, header, body and actions slots; the whole card can be a link), `UiDivider`.
  - **Lists:** `UiList` and `UiListItem` (one to three lines, leading and trailing slots), the main way to show collections on phones.
  - **Actions:** `UiButton` (filled, tonal, outlined, text), `UiIconButton`, `UiFab`.
  - **Inputs:** `UiTabs` (every page-level view switcher: Food, Calendar, Inbox, Tasks; segmented on compact, tab row otherwise, links when an option has `href`), `UiTextField`, `UiSelect`, `UiSwitch`, `UiCheckbox`, `UiRadioGroup`, `UiSegmented` (value pickers in forms and settings), `UiChip` (assist, filter, input), `UiFilterChips` (the shared filter row, §1.5).
  - **Overlays:** `UiDialog`, `UiBottomSheet`, `UiSheet` (bottom sheet on compact, dialog otherwise), `UiMenu`, `UiSnackbarHost`.
  - **Feedback:** `UiBadge`, `UiProgress`, `UiSkeleton`, `UiEmptyState`, `UiErrorState`, `UiSaveBar`.
  - **Navigation:** `UiNavBar`, `UiNavRail`, `UiNavDrawer`.
- **Cards separate content.** A screen is a stack of cards or lists on the surface. Each card holds one subject (a settings group, an event, a bill) with its own actions; there are no free-floating forms.
- **Icons:** Material Symbols (rounded) as tree-shaken SVG components (R-005), coloured with `currentColor`. Views use semantic names from `ui/icons.ts` (`home`, `calendar`, `tasks`, `inbox`, `ai`, …) and never import an icon set directly, so the set can be swapped in one file. Icon-only buttons carry an `aria-label`.
- **Gallery:** `/ui-gallery` (development builds only) shows every component in every state with a theme switcher. `scripts/ui-shots.sh` captures it and the app pages at 360, 800 and 1280 px in a light and a dark theme, for design review.

### 1.3 Navigation (D-095)
The destinations are data in `app/navigation.ts` (`id`, label, icon key, route, optional badge, adult-only, primary). Each navigation component renders that one list:
| Window class | Navigation |
|---|---|
| compact | **top app bar with a menu button** (hamburger) that opens a **modal navigation drawer** with every destination, grouped as on expanded. There is no bottom bar, so the top bar carries each screen's **contextual actions** (R2 review, 2026-11-04) |
| medium | **navigation rail** on the left with the primary destinations; More becomes a menu; Settings sits at the rail's foot |
| expanded | **navigation drawer** (standard) with every destination, grouped: Plan (Today, Calendar, Tasks, Routines), Food (Meal plan, Shopping list, Recipes, Templates; Templates adults only, M5), Mail (Inbox, Mail, Bills), Settings. The user can **collapse it to the rail** (a toggle at the drawer's top), which is remembered per device (`localStorage` `fa.nav.collapsed`) (R2) |
- Children see only the destinations their guards allow (no Inbox, Mail or Bills, D-063; no Templates, D-110).
- **Food (M5, D-117)** is one primary destination `food` (`/food`, icon key `food` in `ui/icons.ts`, the Material Symbol `restaurant`) on the rail; on the drawer it expands into its four entries, which are the tabs of §3f.
- Each screen has a `UiTopAppBar` with its title, a back arrow on nested pages (`/tasks/:id`, `/settings/*` on compact), and its actions. On compact, the menu button sits at the start of every top-level screen's top bar, and a nested screen shows a back arrow instead. The primary create action of a list screen is a `UiFab` on compact and a top-bar button otherwise.
- A parent destination is active for its nested paths (§2).

### 1.4 Applying changes (D-096)
One rule everywhere outside the dashboard:
- **A single control applies instantly.** A switch, a stand-alone single choice (radio group, segmented button, select) and a checkbox in a list save on change through `useInstantSave`: an optimistic update, then a snackbar "Saved" with **Undo** (about 5 s), which saves the old value. A failure reverts the control and shows the error in the snackbar.
- **A multi-field form saves together.** It edits a draft from `useFormDraft` (`draft`, `dirty`, `save`, `discard`). While it is dirty, a `UiSaveBar` (Discard, Save) shows, **sticky at the bottom of the viewport on every window class** (R2). Leaving the page asks first (router guard and `beforeunload`). Success shows a snackbar; field errors from the server show under their fields, and a 409 shows "changed elsewhere, reload" in the bar.
- **Creating** something (a task, a source, a member) is a form in a `UiSheet` with its own Save, never instant.
- **Destructive** actions confirm in a dialog unless they offer Undo (task completion keeps its Undo snackbar, §3a). **Inbox Approve** applies instantly; **Reject** asks first, since there is no undo endpoint yet (D-104, Q-089).
- **Settings → System** switches are part of the page draft and save with the Save bar, because each save re-reads the server config (D-104, Q-094). **Settings → Account** changes the password through the Save bar. **Members** are edited in a sheet on every width, accepted for now (D-104, Q-093).

### 1.5 Filter rows (D-105)
Every filter row is one shared component, `UiFilterChips` (`ui/`; props `options`, `modelValue: string[]` where empty means All, `allLabel`, `label`), a `group` of filter chips with `aria-pressed` that wraps on phones:
- The first chip is **All**. Choosing All clears every other chip.
- Choosing any other chip clears All and toggles that chip on or off independently, so Bills can show Proposed and Open together.
- Clearing the last chosen chip returns to All.
- It is used by Bills (state; the grouping by state stays and only the chosen states' groups show), Calendar (members), Tasks (members) and Mail (category, member, source, state). The Inbox **AI only** chip stays a separate toggle and is not remembered (§3e).
- A remembered filter stores the chosen set. Where the API takes a single value, the client filters.

## 2. Routes
| Path | Shell | Guard | Content |
|---|---|---|---|
| `/setup` | setup | setup | First-run wizard (§2a). Only while setup is needed |
| `/login` | app | none | Username and password |
| `/` | app | user | **Today**: my agenda (or the whole household for adults), weather card, active advisories, my Now list |
| `/calendar` | app | user | Agenda, week, and month views. Filter by member. Child users see only what §3.5 of Feeds.md allows |
| `/tasks`, `/tasks/:id`, `/routines` | app | user | §3a |
| `/food` | app | user | Redirects to the Food tab last used on this device (`localStorage` `fa.food.tab`), else `/food/plan` (§3f) |
| `/food/plan`, `/food/list`, `/food/recipes`, `/food/recipes/:id` | app | user | Meal plan, Shopping list, Recipes and a recipe (§3f). Children read; they add and check items and rate recipes (D-110) |
| `/food/recipes/new`, `/food/recipes/:id/edit`, `/food/templates`, `/food/templates/:id` | app | adult | Recipe import and editor, templates (§3f) |
| `/mail`, `/mail/:id`, `/mail/label`, `/inbox`, `/bills` | app | adult | §3b |
| `/settings` | app | user | **Settings index** (§3c): grouped cards; children see only the Personal group |
| `/settings/<page>` | app | per page | Child routes rendered inside `SettingsLayout` (§3c) |
| `/oauth/done` | app | user | Landing page after the Google callback. It shows the result and continues to `return_to` |
| `/dashboard` | kiosk | device | §4 |
| `/ui-gallery` | app | none, development builds only | The component gallery (§1.2) |

- **Setup guard:** the router asks `GET /api/v1/setup` once at start. While `needed` is true, **every route redirects to `/setup`**, and any API call answering `409 setup_required` does the same. Once setup is done, `/setup` redirects to `/`.
- **No `/members` route** and no redirect from it (D-070): member management is `/settings/members`, adults only.
- Nested pages mark their parent nav item active by path prefix (`/tasks/:id` → Tasks, `/food/*` → Food, `/settings/*` → Settings).
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
| 7. Start | Nothing is added for you (D-047). Links to Calendars, Mail and Devices (pair the wall screen) | existing routes |

- Each step saves when you press Next; `GET /setup` returns the step to resume at, so a reload or a second device continues where it stopped.
- Back keeps values; steps 4–7 can be skipped and done later in Settings.
- Each step is a card; a failed step shows the server's field errors under their fields, as in Settings (§1.4).

## 3. App shell screens (M1)
- **Today:** a merged list of today's and tomorrow's events with member chips, the current weather plus the 7-day strip, and advisories at the top. M5 adds a **Today's meals** card (the visible slots; tapping opens `/food/plan`, D-117).
- **Calendar:**
  - The Agenda, Week and Month switch is a row under the top bar (D-103). The first visit opens Agenda on compact widths and Week on medium and expanded. After that the Calendar reopens in the view last used on that device (`localStorage` key `fa.calendar.view`: `agenda|week|month`; the per-width default applies only when nothing is stored). There is no setting for it.
  - The member filter is a `UiFilterChips` row (§1.5).
  - Events are coloured by their linked members (several members: equal bands in the stripe, D-086), and multi-member events show stacked chips.
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
- **Tasks** (`/tasks`): the member filter is a `UiFilterChips` row (§1.5), remembered per device in `fa.tasks.filter` as a list of member ids (default the viewer's own member, 'Mine'; an old string value is migrated). Tabs *Now* (D-042: "Mine" first, then "Household"), *Mine*, *Lists* (each list with its open tasks), and *Suggested* (adults, weather suggestions to accept or dismiss). Each item shows title, due (relative, overdue in red), assignee chips, a step count, and a claim button for pool tasks. Completing shows a 10-second **Undo** snackbar.
- **Task detail and editor:** title, notes, due, steps (add, reorder, tick), priority flag, and calendar display (`hidden`, `until done`, `always`; "key date" is the label for `always`, D-046). Children get the same editor for their own one-off tasks on lists that allow it (D-045).
- **Routines** (`/routines`): today's runs with a **progress bar** per member (D-041). Children see their own.
- **Settings → Tasks** (adults): lists (policy, flags, calendar default, claim time), definitions (schedule `every N days after done` or `repeat on…` with an RRULE builder for weekly/monthly, lead days, due time, missed behaviour, strategy and ordered candidates, rotation options, priority, season window and condition with the same validation as the weather rule editor, pause), routines (members, window, steps with target times, day filter, school calendar and keywords, dashboard toggle), and **absences** (member, from, until).
- **Settings → Notifications:** per member (self, plus children for adults): trigger toggles, due-soon and nudge minutes, quiet-hours override, copy-to adult, the subscribe URL with a QR code, and "Send test". When the server has no ntfy URL, the screen says notifications are not configured.
- **Planned after M4:**
  - **Settings → Calendars**, on a CalDAV source: a **Trust other hosts** toggle (`trust_other_hosts`, default off; D-099, [Feeds.md](Feeds.md)).
  - **Settings → Notifications**: ntfy **generic text** (`ntfy_generic_text` `auto|on|off`, default `auto`: on when `ntfy_url` resolves outside the LAN and tailnet ranges; D-102, [Tasks.md](Tasks.md)).
- **Settings → Household** gains the Now list size and weights, and the default quiet hours.
- **Calendar page:** tasks with `until_done` or `always` show as task chips (checkbox and member colour) on their due day; `always` stays after completion, shown as done (D-046).

## 3b. App shell screens (M3, [Ingest.md](Ingest.md))
New routes (adult guard unless noted): `/mail`, `/mail/:id`, `/mail/label`, `/inbox`, `/bills`, `/settings/mail`, `/settings/orgs`. Children get none of them (D-063); they see derived events and tasks in the existing screens.
- **Mail** (`/mail`): messages from visible sources (D-063), filtered by category, member, source and state; ignored categories hidden behind a toggle. Each row: sender, subject, category chip, member chips, time-sensitive marker. Category, member, source and state are `UiFilterChips` rows (§1.5).
- **Message** (`/mail/:id`): metadata, body (when content is kept), and for a child filter the relevant and general sections first with the others collapsed (D-055); the decisions with one-tap "Fix"; derived items; "Make task"; mail actions with an Undo list for 30 days (D-058; hidden on read-only sources); "open in Gmail ↗" when an external link exists.
- **Inbox** (`/inbox`): tabs for events, bills, tasks and contacts (D-056, D-057, D-060). Events show the suggested calendar with a picker and editable title and time; tasks show assignees and due; contacts show org, child and the triggering message. Approve and reject buttons (Reject confirms first, §1.4), a nav badge with the count, and SSE `inbox.updated`. Each adult sees only items from sources they may see (owners, or every adult for `household` sources); the server filters, the UI has no other view (D-078).
- **Labelling** (`/mail/label`, D-062): phone-first, one message per screen, the four decision points pre-filled as chips (one tap confirms all, tapping a chip fixes it), "in eval sample" toggle, progress toward ~200, and "next" from the suggested balanced queue.
- **Bills** (`/bills`): open, proposed and paid bills with payee, amount, due date and the pay task's state (D-059), grouped by state with a `UiFilterChips` state row (§1.5, D-105).
- **Settings → Mail:** sources (add **Gmail** via Google sign-in or **IMAP** with host, port, username and app password plus a "Test connection" button, D-052; owners, visibility, read-only, backfill, filter, folders, poll, state and reconnect), categories (behaviour, notify, auto actions, email tasks; D-054, D-058, D-060), rules (condition builder with a dry run on recent mail), and household mail settings (event auto-write off by default with its thresholds, inbox expiry, gender term lists; D-055, D-057, D-061).
  - **Planned after M4 (D-098, D-100, D-101):**
    - per source, **task visibility** for mail-derived tasks (`task_visibility`: `follow_until_accepted` default, `follow_until_done`, `household`);
    - on an IMAP source, an optional **PEM** (`imap_pem`, a server certificate or CA bundle; write-only like other secrets);
    - the household **max message size** (`mail.max_message_bytes`, default 40 MB) and, per source, the list of **too-large messages** (stored as headers only).
- **Settings → Organizations:** orgs (kind, domains, senders, ICS feed, default calendar, contact mode), contacts, and enrollments per child.
- **Member editor** gains aliases, and for children an optional gender and relevance tags (D-055).
- **Inbox** is an app screen only: approvals never appear on the dashboard (D-068).
- **Settings → Notifications** gains the `approval_needed` trigger (batched by default, immediate, digest only, off; D-061) and `mail_time_sensitive`.

## 3c. Settings area (M3.5, U-08, D-070, D-075)
`/settings` is a parent route with `SettingsLayout`, a **list-detail** layout (D-095). On compact and medium screens the index (`SettingsIndexView`) is a grouped list (icon, title, one-line summary) and each page opens full-screen with a back arrow. On expanded screens the list stays as the left pane and the page shows in the right pane. Each page is a stack of cards, one per setting group, following §1.4. Every page keeps its own guard.

| Group | Pages (path under `/settings/`) | Guard |
|---|---|---|
| Personal | `account` (password, sessions), `appearance` (this device's theme, §3d), `notifications` (own, plus children's for adults; §3a) | user |
| Household | `household` (name, tz, location, units, currency, calendar window, Now list weights, quiet hours, dashboard pending-approvals badge), **`members`** (add, edit, avatar, hue, role, account and password reset, PIN, aliases, gender and relevance tags, interests, hobbies and responsibilities), `tasks` (lists, definitions, routines, absences) | adult |
| Household (M5) | `meals` (§3f: slots, stores, aisles, ingredient catalogue and staples, plan horizon) | adult |
| Sources and rules | `calendars`, `weather`, `mail`, `orgs` | adult |
| AI | `ai` (§3e) | adult |
| System | `system` (below), `devices` (pairing + dashboard layout editor, §4), `status`, `about` (server version, commit and build time, next to the same for this page's bundle; a banner when the two commits differ, which means the browser is showing a stale cached app) | adult |

- **Members** (D-070): adults only, since other features rely on member details. Children see members on the dashboard's `members` strip, not in the app. A child sees only the Personal group.
- **Settings, System** (D-075, D-074, D-072):
  - **Runtime settings:** public address(es), ntfy URL and token, Google client ID and secret, plain-HTTP feeds, private hosts for feeds, the home-network dashboard listener (D-064), trusted proxies (D-069). Each field shows its source: editable when stored in the DB; **locked with "set by the server"** when an environment variable overrides it. Secrets are write-only ("set" / "not set", replace, clear). Fields that need a restart (listeners) say so after saving.
  - **Connection:** a plain statement such as "This page is served over HTTP" or "HTTPS via Tailscale (`https://family.<tailnet>.ts.net`)". No warning styling.
  - **Master key backup:** a warning card until an adult has confirmed a backup; it reopens the step-6 Copy/Download flow (needs a recent login or elevation).

**Second adult (D-088).** In Settings → Members an adult can create another adult with a username and initial password (no PIN, no grade) and edit them until they claim the account; a claimed adult is read-only to others. A session whose account still has the initial password is redirected to Account with a banner until the password is changed.

## 3d. Themes (D-079, D-080, D-081)
**Catalogue.** `themes/` holds one data file per family: Catppuccin, Nord, Dracula, Gruvbox, Solarized, Tokyo Night, Rosé Pine, Kanagawa, seoul256 (D-083; dark 233–239 and light 252–256 as variants). Nord also has a light Snow Storm variant (D-083). A family lists the light and/or dark variants its project publishes (Catppuccin has Latte and three dark flavours). Every variant fills the same tokens:
- **neutrals:** `bg`, `surface`, `surface-2`, `border`, `muted`, `text`, `on-accent` (text on a hue-coloured fill);
- **extended slots** (optional, D-106, taken verbatim from the variant's Neovim port, R-006): `layers` (4 steps: page = `bg`, card, raised, dialog), `textStrong`, `textSubtle`, `comment`, `selection`, `highlight`, and extra named `accents`;
- **hues:** `red`, `orange`, `yellow`, `green`, `cyan`, `blue`, `purple`, `pink`. A palette without a native hue supplies its nearest colour.

`applyTheme` writes the tokens as CSS custom properties on `<html>` (`--color-bg`, `--color-surface`, `--color-text`, `--hue-red`…). The existing names in `base.css` stay as aliases, so components keep working: `--color-accent` is the `blue` hue, and `--color-danger`, `--color-warning` and `--color-success` are `red`, `yellow` and `green`. When `extended` is present `applyTheme` also writes `--color-layer-0..3`, `--color-text-strong`, `--color-text-subtle`, `--color-comment`, `--color-selection` and `--color-highlight`; when it is absent it writes derived values (layers by `color-mix` of `bg` toward `text`, the text slots falling back to `text` and `muted`). Variants without a Neovim port (Dracula Alucard, Nord Snow Storm, Seoul256) have no `extended` and keep the derivation, Seoul256 with smaller mix steps.

**Per-layer secondary text** (D-106). Palette values are never altered for contrast. For each layer `applyTheme` picks the secondary-text colour and writes it as `--color-on-layer-N-subtle`: `textSubtle` where it reaches 4.5:1 on that layer, else `muted`, else `text`. `comment` is for placeholders and disabled text only. Source and licence of each palette are recorded next to its file.

**Mode.** A device's choice is `{family, variant?, mode}` with `mode` one of `auto | light | dark`.
- A family with **both** light and dark variants follows the mode. In the app, `auto` follows the device's `prefers-color-scheme`. On the kiosk, `auto` is dark between sunset and sunrise (from the weather data).
- An optional `dark: {family, variant?}` gives a separate **night-time theme** (stored in the same `device.settings` jsonb, so no migration; absent means the daytime family, and the legacy string and objects without it still read). When the effective mode is dark (`mode` dark, or `auto` at night or with a dark system scheme) the `dark` family and variant are used, otherwise the daytime ones. Settings → Devices shows "Daytime theme" and "Night-time theme" (default "Same as daytime"). Both slugs are validated like `family`.
- A family with **one** variant always uses it, at any time of day. The mode picker is hidden for it.
- A family with several variants in one mode (Catppuccin dark) lets the user pick the variant.

**Where the choice lives** (D-081; there is no household setting):
- **App devices:** `localStorage` key `fa.theme`, edited in **Settings → Personal → Appearance** by any signed-in member. It does not follow a person to another device. Until a choice is made the app uses its shipped default (a technical choice: Catppuccin, `auto`).
- **Kiosk and dashboard devices:** `device.settings.theme`, set by an adult in Settings → Devices (the layout editor's live preview uses it). The kiosk uses the same catalogue and never picks a theme itself (D-019).

**Member colours** (D-080). A member's `color` is a hue name. The client uses `var(--hue-<name>)` for chips, the agenda and task items. `eventColors` returns one hue per linked member (an event for two members shows both), and a neutral for an event with none; calendars have no colour (D-086). In the Week view an event takes its column's member colour. The member picker shows the current theme's swatches.

**Contrast** (build-time unit test over every variant): `text` and `muted` on `bg` and on both surfaces at least 4.5:1; `text` and the chosen subtle token on every layer at least 4.5:1; each hue on `bg` at least 3:1 (stripes, chips, accents); `on-accent` on `blue`. Neovim palette values are **not altered** (D-106); where a slot fails, the fallback chain above applies. The earlier R-002 corrections to `text` and `muted` stay. The three light themes (Solarized light, Tokyo Night Day, Rosé Pine Dawn) keep their published accents, so **accent-as-text below 4.5:1 is accepted** there (D-106, reverting the M4 darkening) and no test forbids it.

## 3e. Settings → AI and AI provenance (M4, D-093, D-094; detail in [AI.md](AI.md))
`/settings/ai`, adults only, is a stack of cards:
- **LLM endpoint:** base URL, model, dialect (OpenAI-compatible by default, Ollama native as a fallback), API key (write-only: "set" / "not set", replace, clear), timeout, and an enabled switch. It is a form with the Save bar (§1.4), except the enabled switch, which applies instantly.
  - **Test connection** sends the unsaved values and shows the latency and whether the model was found, or the error kind.
  - The model field is a select filled from the server's model list when one is available, otherwise free text.
  - A one-line hint says the LLM server should listen only on the home network or tailnet.
- **Mail processing (D-107):** a radio group, "Use the AI for every message (rules as fallback)" (`llm_first`, default) or "Use rules first, AI only when unsure" (`rules_first`); instant, with Undo. In `llm_first` it also shows "Catch up on mail from the last N days" (0 to 365, 0 is "Off"; a form with the Save bar). A line under the endpoint card shows the backlog from `GET /ai/backlog` ("12 messages waiting for the AI · 3 queued"), hidden at zero, refreshed on endpoint save or toggle and every 30 s.
- **Email decisions:** one row per decision point (`email.category`, `email.time_sensitive`, `email.member_link`, `email.is_bill`), each with an LLM switch (instant) and an accept threshold slider (saves on release, with Undo). In `rules_first` each row also has a rules threshold slider (50 to 99 %, same behaviour).
- **Placement:** Settings → AI sits under "Sources and rules", after Mail. A newly saved endpoint starts **enabled** (D-104, Q-096).
- **Extraction:** in `rules_first`, `mail_extract_mode` as a segmented button (Off, When rules find nothing, Always; instant) and, for "When rules find nothing", the fallback cutoff slider (50 to 99 %, saves on release, with Undo). In `llm_first` the mode does not apply and is hidden. The max input size (form) is always shown.
- **Activity:** a stats card for the last 7 days. It shows LLM calls per purpose (count, success share, p50 and p95 latency) and, per decision point, the share decided by rules, the LLM and a person. It is read-only.

**AI provenance in the inbox:** items whose `extracted_by` is `llm` carry an **"AI" chip** (the `ai` icon) with the confidence as a percentage. Its tooltip, or a tap on compact screens, explains "suggested by the local language model". Approval works exactly as for rule-made items (D-057, D-060). Tasks carry just "AI". Wide screens show a tooltip and phones a sheet (D-104, Q-097). The Inbox has an **"AI only" filter chip**, a separate toggle that is not remembered. The message screen's decision list marks decisions with `stage_kind = llm` the same way.

## 3f. Food (M5, D-109 to D-118; detail in [Meals.md](Meals.md))
`/food` is one destination with tabs (`UiTabs`: a segmented control under the top bar on compact, a tab row otherwise): **Meal plan**, **Shopping list**, **Recipes**, and **Templates** for adults. The chosen tab is remembered per device (`fa.food.tab`). All edits follow §1.4.

**Meal plan** (`/food/plan`, adults edit, everyone reads; D-110, D-111, D-112, D-116):
- A grid of **week rows**: seven day columns starting on the household's `week_start`; the first row is the **previous week**, then the current week, then following weeks, scrolling on to the end of the horizon (`meal_plan_horizon_weeks`); scrolling up shows past weeks read-only. Today's cell is drawn **inverted** (text and background swapped), the same marker as today on the calendar. On compact the grid becomes a day list (one card per day, the same order).
- Each cell shows the **visible slots** in slot order (only Dinner by default, D-111), each with a recipe title (thumbnail on expanded) or free text; a slot may hold **several entries**, listed in order, with a "+" for adults to add another (D-120).
- Adults add an entry from a sheet (recipe search or free text, servings defaulting to the recipe's), edit it, delete it with Undo, and **move or swap** entries by drag on pointer devices and by "Move to…" (date and slot picker) everywhere; dropping an entry on a slot appends it there, and dropping it on another entry swaps the two (D-120).
- **Apply template:** pick a template and a start date. The sheet previews the slots it fills and lists every **conflict** (a slot that already has entries, shown with them) with a **Replace / Add** choice per slot. Slots holding any free-text entry are preselected **Add**, others **Replace** (D-120). Applying is one change with Undo (D-096). Applied entries are detached from the template (D-112).
- **Make grocery list:** choose a date range (default the coming week) or pick meals in the grid (selection mode); a summary shows items to add and the Staples check; applying shows Undo.
- **Plan change with list impact (D-116):** when an edit, delete, move, swap, servings change or template apply touches an entry that contributed to unchecked list items, the server returns its `grocery_impact` and a dialog lists the items that would be reduced or removed, with **Update the list** and **Leave the list**. Nothing is asked when there is no impact.
- **Save as template** (adults): pick a date range and the slots to include, name it, save. Every entry in those slots becomes a template item (D-122).

**Shopping list** (`/food/list`, D-021, D-110, D-115, D-118):
- An **add box** at the top (text; suggestions from the ingredient catalogue as you type; the item takes the ingredient's default store).
- Items grouped **by store** (store order, "Any store" last), then **by aisle** in **that store's** aisle order ("Any store" uses the household order). Aisles a store hides, and unmatched items, go to that store's "Other" (D-121). A household with no stores sees aisles only. Each row: text, quantity and unit in household units, a source hint ("for Tue dinner, Thu dinner") when generated.
- **Store filter** as a `UiFilterChips` row (§1.5) to shop one store; remembered per device (`fa.grocery.stores`).
- **Check off** is instant with Undo; checked items fall to a collapsed "Checked" group until archived (default 24 h, D-021).
- The **Staples check** section (D-021) sits below, listing staples with the meals that need them and a **Low** button that moves one onto the list.
- Adults edit an item (text, quantity, unit, store) in a sheet and **delete** it with Undo; children add and check only (D-110).
- Offline behaviour as in §1.1.

**Recipes** (`/food/recipes`, D-109):
- **Library:** search (title and ingredient), tag chips (`UiFilterChips`), a favourites chip, cards with image, title, total time, the household's average rating, and "last planned".
- **Recipe page:** image, servings, times, ingredients (each line with its matched ingredient on wide screens), steps, the household note, **ratings** (every signed-in member sets their own 1 to 5 stars, instant with Undo; the list of members' ratings shows under it), "Add to plan", and for adults Edit and Delete.
- **New recipe** (adults): a sheet with three ways in, **From a link**, **Paste text**, **Type it in**; the first two show progress and open the **editor** on the returned draft (nothing is stored until Save). Errors (no recipe found, LLM not available) are shown with the fallback "Type it in".
- **Editor:** a form with the Save bar: title, servings, times, tags, note, image (keep the imported one, upload, or remove), ingredient lines, steps. **Unmatched lines** are highlighted; a line with an LLM proposal shows it as a chip ("tomato, canned?") with Accept, Pick another (catalogue search) and New ingredient; a picked match becomes an alias (D-113).

**Templates** (`/food/templates`, adults, D-112): a list of templates (name, length, slots used), and an editor: name, length in days, and a grid of days × slots where each cell holds a recipe, free text or nothing; a cell can hold several items (D-120). Days are numbered (Day 1…) and shown in week rows when the length is a multiple of 7. **Duplicate** copies a template under a new name (D-122). Delete asks first.

**Settings → Meals** (`/settings/meals`, adults): cards for
- **Slots:** name, `"HH:MM"` time, shown switch (instant), order; add and rename. A new household has Breakfast, Lunch, Dinner and Snack with only Dinner shown (D-111);
- **Plan horizon:** `meal_plan_horizon_weeks`;
- **Stores:** name and order, add, delete (items move to "Any store"). Per store, an **aisle order** editor (drag to reorder, a switch to hide an aisle the store does not have), starting from the household order (D-121);
- **Aisles:** name and the household order (used for "Any store" and new stores);
- **Ingredients:** the catalogue with search; per ingredient the canonical name, aliases, aisle, default store, default unit and the **staple** switch (instant); add and merge.

## 4. Dashboard (kiosk shell)
**Navigation (D-082, D-084, D-090):** a bar along the shorter edge of the screen as detected (left in landscape, top in portrait; same button size in both) switches between **Home** (the configured panels below), **Week** (a table: a row per day for the calendar week that starts on the household's `week_start` (Monday or Sunday) and holds today, a column per person; each cell lists that person's events, drawn like the calendar page's, and the tasks due that day; the day cell carries the forecast and the day's visible meals (M5); it fills the view and never scrolls: a cell that cannot fit its items in one column splits them into two half-width columns when each half stays readable, then ends with a "+N" slot that opens a modal listing everything in that cell, D-087). A bottom row has previous and next arrows, a box showing "Current" or the date range of the shown week, and a "Hide past events" switch (ended events go; a task goes only once completed). The view fetches its own week with `GET /api/v1/dashboard?from=`) **Food** (M5, D-117, below) and **Timers** (below). The view is local state. The Home view scrolls, so no panel is cropped when another needs more room: in landscape each of its two columns (left 4/12, right 8/12) scrolls on its own, in portrait the clock and weather share a fixed top row 50/50, a thin members strip follows, then two columns that each scroll on their own: "right now" (now, routines, member tasks) on the left and "coming up" (agenda, week) on the right, a lone column taking the full width (the groups are `PORTRAIT_*` in `layout/index.ts`); the Week view does not scroll (D-087). The connection status (Connected only while heartbeats arrive: "No data" after 45 s of silence, D-091), the stale badge and the adult unlock button sit at the end of the bar (D-085).

**Timers view:** a bar button (timer icon) after Food, always shown. An "Add timer" button along the top opens a modal (minutes and seconds with steppers, preset chips 5/10/15/30 min, optional name). Each timer is a card in a responsive grid with a progress ring, the remaining time (`mm:ss`, `h:mm:ss` from one hour) and the actions +1 min, Pause/Resume, Cancel (reset) and Remove. Timers store an absolute `endsAt` (`Date.now`), so throttled rendering cannot drift; paused ones store the remaining time. They live in a page-wide store (`dashboard/timers/useTimers.ts`), persist per device in `localStorage` (`fa.dashboard.timers`) and keep running while another view is open; one that ended while the page was away rings on load. A ringing card pulses and the bar button shows a dot; a synthesised chime (Web Audio, no audio files) repeats every 1.5 s until every ringing timer is dismissed (Dismiss button or a tap on the card). The audio context is unlocked by the tap that adds a timer.

**Food view (M5, D-117):** a third bar button, shown unless the device setting `food_view` is off (default on). Two tabs:
- **Meal plan:** the §3f grid read-only, week rows from the previous week with today inverted, as many rows as fit the screen: it **never scrolls** (previous and next arrows page by a week, like the Week view).
- **Shopping list:** the §3f list grouped by store then aisle, with the add box and check-off, both **without a PIN** (D-015, D-019, D-110); the member picker is not asked. Editing and deleting an item need the adult PIN step-up (D-085 unlock button); Staples **Low** needs no PIN. The store filter chips are local state.

**Screensaver (D-092):** after `screensaver_minutes` (default 15) without a touch, key or pointer event, a black full-screen overlay shows the app icon bouncing off the screen edges like a DVD logo. A tap dismisses it. Data keeps updating underneath.

The dashboard is **at-a-glance only** (D-068): date and time, weather with advisories, agenda, tasks, routines, meals and the shopping list (M5), and the household's members. **Nothing derived from email** appears on it: no inbox panel, no titles, senders or approvals, elevated or not. The one exception is a **pending-approvals count badge per adult** on the `members` strip (D-078), which the household setting `dashboard_pending_approvals` (default on) can turn off.

### 4.1 Layout model
The screen's orientation is not part of the layout: the dashboard detects it (portrait when taller than wide) and picks the panel arrangement and menu edge itself (D-090).
The layout is data held in `device.settings` and edited by adults in Settings → Devices. It is never edited on the dashboard itself (D-019).
```json
{ "theme": { "family": "<id>", "variant": "<id>?", "mode": "auto" | "light" | "dark" },  // §3d; auto = dark between sunset and sunrise (from weather)
  "agenda_days": 3,
  "screensaver_minutes": 15,   // 0 = off, max 240; absent = 15 (D-092)
  "food_view": true,           // M5, D-117: the Food bar button; absent = true
  "members": "all" | ["<member-id>", ...],
  "panels": [ {"kind": "clock"}, {"kind": "weather", "hourly": true},
              {"kind": "agenda"}, {"kind": "week"}, {"kind": "members"} ] }
```
- **Panels:**
  - `clock` (time, date);
  - `weather` (a compact panel: icon, now, feels like and wind, advisories on a second row, and before noon household time today's high, low and rain chance; tapping it opens a modal with today by the hour and the upcoming days, D-085). It carries the **advisories** (coloured by severity, beside "feels like / wind") and a derived notice for a thunderstorm in the forecast (D-082). Advisories are **not dismissable** by anyone: an advisory disappears when its `valid_until` passes or when the server finds its rule no longer matches the forecast (D-068), arriving as SSE `advisory.updated`;
  - `agenda` (today plus the next N days, grouped by day, with key-date tasks, D-046; each event shows its time with the member chips under it and its title in an aligned column, and opens a modal with the location and details, D-085);
  - `week` (a 7-day column grid, with `until_done` and `always` task chips);
  - `members` (avatars and names of the household, D-070; the entry point for per-member task views and the PIN step-up; adults carry a count-only "to approve" badge when `dashboard_pending_approvals` is on, D-078);
  - `meals` (M5, D-117: today's entries in the visible slots, in slot order with the slot time, recipe image on wide panels; free text as written; "Nothing planned" when empty; tapping opens the Food view's meal plan);
  - `now` (the Now list, D-042, with optional `members` and `lists` filters), `member_tasks` (one column per member with today's open tasks), `routine` (D-041: shown automatically during a routine window with large step buttons and a progress bar; `pinned: true` keeps it visible).
- Completing anything on the dashboard opens the member picker ("who did it?", preselecting the assignee) and needs no PIN while the override is on (D-013, D-019). PIN elevation remains for adult actions on tasks (D-019); it is not used for approvals.
- **Removed in v4:** the `inbox` panel (D-068). A layout that still names it is ignored like any unknown kind, so older saved layouts keep working.
- **Presets** ("Family hub", "Calendar wall", "Minimal") fill `panels`; Family hub includes `meals` (M5). Unknown panel kinds are ignored, so an older client survives a newer layout.
- **Landscape 1920×1080:** a 12-column grid. The left column (4) holds clock and weather, and the right (8) holds agenda or week. **Portrait** stacks them. Type is sized for 2–3 m (base 28 px on the kiosk scale, one kiosk font system).
- The layout editor shows a live, scaled preview of the dashboard with the edited settings.

### 4.2 Data & resilience
- **One fetch:** `GET /api/v1/dashboard?days=N` returns household (name, tz, units), members (public fields + avatar URL), events in the window (already masked), the latest weather, active advisories, `tasks[]` (open tasks visible now, plus calendar-displayed tasks in the window), `routines[]` (today's runs with steps), `meals[]` (M5: entries in the visible slots for the window), and the device settings. The Food view fetches the plan and the list on its own (`GET /meal-plan`, `GET /grocery`). Refreshes reuse it. It carries no mail-derived data.
- **Live:** SSE kinds update the snapshot: `calendar.updated` (refetch when its range overlaps), `weather.updated`, `advisory.updated`, `tasks.updated`, `routines.updated`, `device.updated`, `member.updated`, `household.updated`, `mealplan.updated` and `grocery.updated` (M5), and `resync` (full refetch).
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
6. On connect the stream first sends `build {commit}` (D-123, §1.0); it is not a data change and no store refetches.
7. M5 kinds: `mealplan.updated {from, to}` (the plan stores and the dashboard refetch when the range overlaps), `grocery.updated` (the grocery store refetches the list; queued offline writes replay first, §1.1), `recipes.updated` (library).

## 6. Accessibility & i18n
- Every interactive element is reachable by keyboard, with visible focus rings (a `ui/` token, never removed). Touch targets are at least 48 px in the app shell and on the kiosk. Icon-only buttons have an accessible name; snackbars use `role=status`; dialogs and sheets trap focus and close on Escape.
- Member colours keep at least 4.5:1 contrast for chip text: text colour is computed from the background.
- Strings are English and live in one `strings.ts` per area, which makes later i18n possible but isn't built now. Dates and numbers use `Intl` with the browser locale and the household tz.

## 7. Testing
- **vitest:**
  - `ui/` components and composables (`useInstantSave` undo and revert, `useFormDraft` dirty and leave guard, breakpoints);
  - stores (invalidation logic);
  - the layout engine (presets, unknown panels);
  - unit conversion;
  - (M5) the meal-plan grid (week rows from the previous week on `week_start`, today inverted, visible slots only), several entries per slot, the template conflict list (Add preselected for slots with free text), save as template, the grocery grouping (store, then that store's aisle order, hidden aisles to Other, Any store last), the offline queue (order, idempotent replay, 404 drop), the plan-change impact dialog, the rating control;
  - day grouping across DST changes, using a fixed tz.
- **Playwright** against the e2e stack, which has a seeded household, an ICS fixture, and an Open-Meteo stub:
  - the dashboard renders the agenda, weather, and advisory;
  - an SSE update appears without a reload;
  - a child login sees only permitted events;
  - the settings flows for ICS and weather rules work;
  - **first run** (M3.5): an empty database redirects every route to `/setup`; the wizard creates the household, owner, a child and confirms the key backup; `/setup` is then closed;
  - the whole suite runs over plain HTTP from a non-localhost address (insecure context, §1.1);
  - Settings, System shows env-set fields as locked;
  - the dashboard has no advisory dismiss control and no inbox panel; a child sees no `/settings/members`;
  - (M4) navigation adapts at 360, 800 and 1280 px; an instant switch shows Undo and Undo restores the value; a dirty form blocks navigation until saved or discarded;
  - (M5) an adult imports a recipe from a fixture page, saves it, plans it, applies a template with one kept conflict, makes the grocery list and sees the staples check; a child adds and checks an item but sees no delete and no Templates; the kiosk Food view adds and checks an item without a PIN and its meal plan does not scroll; the `meals` panel shows today's dinner; an item checked while offline (route blocked) syncs when the route returns.
- **Screenshots** (`scripts/ui-shots.sh`, tagged `@shots`, not part of the default run) feed the design reviews; they are not compared pixel by pixel.
