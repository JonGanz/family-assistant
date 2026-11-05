---
id: T-10
title: "Migrate Today, Calendar and Routines to the design system"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/HomeView.vue", "apps/web/src/views/CalendarView.vue", "apps/web/src/views/RoutinesView.vue", "apps/web/src/components/EventCard.vue", "apps/web/src/components/DateRangeNav.vue", "apps/web/src/components/MemberChip.vue", "apps/web/src/components/Avatar.vue", "apps/web/src/components/WeatherIcon.vue", "apps/web/src/components/icons/**", "apps/web/src/components/components.test.ts", "apps/web/src/stores/calendar.ts", "apps/web/src/stores/calendar.test.ts", "apps/web/src/stores/routines.ts", "apps/web/src/stores/weather.ts", "apps/web/src/views/today.test.ts", "apps/web/src/views/parts/AdvisoryList.vue", "apps/web/src/views/parts/AgendaList.vue", "apps/web/src/views/parts/WeatherCard.vue", "apps/web/e2e/ui-shots.routes/today.ts"]
decisions: [D-095, D-096, D-086, D-080, D-046, D-041]
---

# T-10: Migrate Today, Calendar and Routines to the design system

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The design system (`apps/web/src/ui/`, T-02) and the new app shell with two reference pages (T-05) are in place and were reviewed by the user at checkpoints R1 and R2. You migrate **Today** (`/`), **Calendar** (`/calendar`) and **Routines** (`/routines`), plus the shared event and member components they use. Five tasks migrate views in parallel in this wave, so stay strictly inside your paths.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership* (what is **not** yours), *Test files*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 and §3 (Today, Calendar), §3a (Routines, tasks in the calendar), §3d (member colours)
- **The reference pages** from T-05: `apps/web/src/views/TasksView.vue` and `apps/web/src/views/SettingsHouseholdView.vue`, the "rules of the house" in T-05's report (quoted by the main session under *R2 feedback* below), and `apps/web/src/ui/` with `UiGallery.vue`
- `apps/web/e2e/*.spec.ts`: grep for the labels, roles and test ids your views use

## You own
The paths in `owns` above. `MemberChip`, `Avatar`, `EventCard` and `WeatherIcon` are also used by the kiosk dashboard: change their look only behind a prop or by wrapping (e.g. a `variant="app"` default for the app and the current look for the kiosk). Run the dashboard tests to prove the kiosk is unchanged. **Not yours:** `ui/**`, `app/**`, `layouts/AppLayout.vue`, `router/**`, `gen/**`, `dashboard/**`, `kiosk/**`, and every view, component, store and test file of another wave-3 task (see *View ownership* in `plan/M4.md`).

## Work
1. **Today:** cards for weather (current plus the 7-day strip as a horizontally scrolling row on compact), advisories as a prominent card at the top (colour by severity through tokens and `var(--hue-*)`), today's and tomorrow's events as a `UiList` grouped by day with member chips, and the Now list as a card with instant completion and Undo.
2. **Calendar:** the agenda is the default on compact, and the week view on medium and expanded; month is available through a `UiSegmented` in the top bar. `DateRangeNav` is restyled on `UiIconButton` and `UiButton`. The member filter uses `UiChip` filter chips. Event colour bands per member (D-086), busy blocks hatched, task chips (D-046), and an event detail in a `UiSheet` with the 'from ↗' link.
3. **Routines:** one card per member with a `UiProgress` bar and the steps as a list with checkboxes (instant, with Undo).
4. `EventCard`, `MemberChip` and `Avatar` are restyled for the app on tokens, with the kiosk look kept (see *You own*). `WeatherIcon` keeps its SVGs.
5. **Applying changes (D-096):** every switch, stand-alone single choice and list checkbox goes through `useInstantSave` (Undo snackbar). Every multi-field form goes through `useFormDraft` with `UiSaveBar`. Creating goes through a form in a `UiSheet`. Destructive actions confirm in a `UiDialog` unless they have Undo. Remove the page's own Save logic, toasts and banners in favour of these.
6. **Mobile-first layout:** a `UiPage` scaffold with `UiTopAppBar` (title, back on nested pages, actions). Collections are a `UiList` on compact and cards or a list in a card on wider screens. Each subject gets a `UiCard`. Empty, loading and error states use `UiEmptyState`, `UiSkeleton` and `UiErrorState`. Icons come from `ui/icons.ts` only.
7. **No page-local colours or ad-hoc kit classes:** only `--ui-*` tokens and `ui/` components. Remove uses of `PageHeader`, `StateBlock`, `ErrorBanner` and `Tabs` from your files (do not delete those components; the main session does after wave 3), and the old `base.css` kit classes (`.form`, `.card-list`, `.badge` and so on) from your templates.
8. **Keep behaviour:** guards, store calls, SSE invalidation, `If-Match` and 409 handling, and every existing feature of your views stay. This is a visual and interaction revamp, not a feature change. Keep the accessible names and test ids that `apps/web/e2e` uses; any change you cannot avoid goes in the report.
9. **Tests:** update your area's test file (`apps/web/src/views/today.test.ts`, split by the main session) and the tests next to the components and stores you own. Add at least one instant-save Undo test and one dirty-form test where your views have them.
10. Run `scripts/ui-shots.sh --all` for your routes (add them to the route constant in `apps/web/e2e/ui-shots.spec.ts` only if the main session made it data-driven per area; otherwise list the routes in the report).

## R1 rule (user, 2026-11-04)
Calendar events **inside calendar grids** (month, week and day grids in `CalendarView` and its components) are square: use `--ui-radius-none` (added by T-05). Event cards in lists, such as the Today agenda, keep the normal card rounding. The kiosk `EventCard` look is unchanged.

## R2 feedback
The user answered R2 on 2026-11-04 and 2026-11-05. These rules apply to every wave-3 page:
- **Shell (T-18, done):** phones get a menu button in the top bar that opens a modal drawer, with no bottom bar. Put contextual actions in `UiTopAppBar`'s `#actions` slot (at most two icon buttons plus an overflow `UiMenu` at 360 px). The desktop drawer collapses to the rail. Your page must use `UiPage` + `UiTopAppBar` (Settings children: `SettingsPage`), or phones get no menu button.
- **Save bar:** `UiSaveBar` is sticky at the bottom on every width. Use it for every multi-field form (D-096).
- **Density:** comfortable is the default; build for it, and check the compact density in the gallery does not break your page.
- **Collections on phones:** **cards on every width** (not a plain list on compact). `ResponsiveItems` should render a single-column card stack on compact.
- **Log out** stays at the foot of the navigation drawer. Do not add it to Settings → Account.
- Follow T-05's "rules of the house" and T-18's outcome (`plan/M4/T-05-app-shell.md`, `plan/M4/T-18-shell-r2.md`).

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] `grep -nE "var\(--color-|#[0-9a-fA-F]{3,6}\b" <your .vue files>` shows no page-local colours (theme hues for member colours through `var(--hue-*)` are allowed)
- [ ] e2e selectors you changed are listed (do not run `scripts/e2e.sh`; the main session does)
- [ ] screenshots taken (count from the script's output) and the routes covered
- [ ] report:
  - the files changed;
  - components or variants missing from `ui/` (and the workaround used meanwhile, inside your files);
  - store changes made;
  - questions for the user's R3 review;
  - "list what outside your paths needs changing"
