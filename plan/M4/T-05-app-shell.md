---
id: T-05
title: "App shell: adaptive navigation, snackbar host, Settings list-detail, two reference pages"
milestone: M4
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/app/**", "apps/web/src/layouts/AppLayout.vue", "apps/web/src/views/SettingsLayout.vue", "apps/web/src/views/TasksView.vue", "apps/web/src/views/SettingsHouseholdView.vue", "apps/web/src/router/index.ts", "apps/web/src/App.vue", "apps/web/src/app/*.test.ts"]
decisions: [D-095, D-096, D-097, D-063, D-070]
---

# T-05: App shell: adaptive navigation, snackbar host, Settings list-detail, two reference pages

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. T-02 built the design system in `apps/web/src/ui/`, and the user reviewed it at checkpoint R1. You rebuild the app shell on it and migrate two **reference pages**. Every wave-3 task will copy their patterns, so make them exemplary and keep them simple. The user reviews your screenshots at checkpoint R2.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership*, *Review checkpoints*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4, §2 (routes and guards), §3a (Tasks), §3c (Settings area)
- `apps/web/src/ui/` (everything T-02 built, including `UiGallery.vue` and `ui/nav.ts`), T-02's report and the R1 feedback appended below
- `apps/web/src/layouts/AppLayout.vue`, `apps/web/src/router/index.ts`, `apps/web/src/App.vue`, `apps/web/src/views/{SettingsLayout,SettingsIndexView,TasksView,SettingsHouseholdView}.vue`, `apps/web/src/views/settingsNav.ts`, `apps/web/src/stores/{session,inbox,household,tasks}.ts`
- `apps/web/e2e/*.spec.ts`: the selectors that touch the nav, Tasks and Settings → Household

## You own
The paths in `owns` above. **Not yours:** `ui/**` (report a missing component or variant; the main session adds it), `views/settingsNav.ts` and `SettingsIndexView.vue` (T-13 in wave 3; read them), every other view, store and component, `gen/**`, `dashboard/**`, `kiosk/**`. You may read stores but not edit them. If a reference page needs a store change, report it.

## Work
1. **`app/navigation.ts`**:
   - the destinations as data (`NavDestination` from `ui/nav.ts` plus `adultOnly` and `primary`), in the order of *Navigation* in `plan/M4.md`;
   - a computed `useNavigation()` that filters by the session's role and fills the Inbox badge from the inbox store's count;
   - a test for the filter (a child sees no Inbox, Mail or Bills, D-063) and for the More grouping.
2. **`AppLayout.vue`**:
   - `UiNavBar` plus a More `UiBottomSheet` on compact, `UiNavRail` on medium and `UiNavDrawer` on expanded (via `useBreakpoint`);
   - one `UiSnackbarHost` mounted for the app;
   - safe-area insets, and content that never hides behind the bottom bar;
   - the old text nav and its CSS removed;
   - the existing behaviours kept: active by path prefix, the Settings link and the inbox badge (with its `aria-label`).
3. **`SettingsLayout.vue`** list-detail (`arch/Frontend.md` §3c):
   - on compact and medium, `/settings` shows the grouped list and a child page fills the screen with a back arrow in its `UiTopAppBar`;
   - on expanded, the list is a left pane and the page is the right pane;
   - the list reads groups and pages from `settingsNav.ts` as it is (a field it lacks, such as an icon, is reported for T-13, with a fallback icon meanwhile).
4. **Reference page `TasksView.vue`:**
   - `UiPage` with `UiTopAppBar` (title, filter action);
   - `UiSegmented` or tabs for Now, Mine, Lists and Suggested;
   - tasks as a `UiList` on compact and cards in a grid on medium and expanded;
   - completion as an instant action with the Undo snackbar (reuse `useInstantSave` or `useSnackbar`; keep the 10-second undo window from §3a);
   - a `UiFab` "New task" that opens the create form in a `UiSheet`;
   - empty, loading (`UiSkeleton`) and error (`UiErrorState`) states;
   - "Load more" as a `UiButton` text.
   Keep using the existing `components/tasks/*` components inside it (T-11 restyles them in wave 3).
5. **Reference page `SettingsHouseholdView.vue`:** one `UiCard` per group (Household, Location, Units and currency, Calendar window, Now list, Quiet hours, Dashboard).
   - **Multi-field groups** use `useFormDraft` with `UiSaveBar`, one draft per card (or one per page if the API takes one PATCH; say which you chose and why).
   - **Single switches and choices** apply instantly with Undo through `useInstantSave` (e.g. the dashboard pending-approvals badge, units).
   - Field errors come from the server, and a 409 shows "changed elsewhere, reload".
6. **Router:**
   - move route meta for titles and back targets into the routes if the top app bar needs them (`meta.title`, `meta.back`);
   - keep every guard unchanged;
   - keep T-02's DEV `/ui-gallery` block.
7. **No page-local colours:** the two pages use only `--ui-*` tokens.
8. **Tests:**
   - `app/navigation.test.ts`;
   - a mount test of `AppLayout` at three widths (mock `useBreakpoint`) asserting bar, rail or drawer;
   - updated or added tests for the two reference pages (instant switch shows Undo; a dirty Household form shows the Save bar and blocks leaving).
9. Run `scripts/ui-shots.sh --all` for `/`, `/tasks`, `/settings`, `/settings/household` and the More sheet open (add those routes to the spec's route constant only if T-02 left it data-driven; otherwise report).

## R1 feedback
(filled by main session before launch)

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] e2e selectors: grep `apps/web/e2e` for every label or test id you touched; list any you had to change (do not run `scripts/e2e.sh`; the main session does)
- [ ] screenshots from `scripts/ui-shots.sh --all` listed (count from the script's output)
- [ ] report:
  - the files changed;
  - the patterns wave 3 should copy, as 5 to 10 bullet "rules of the house" (page scaffold, card per group, list versus cards by breakpoint, where actions go, empty, loading and error, instant versus form);
  - components or variants missing from `ui/`;
  - store changes needed;
  - **open questions for R2**: navigation order and the More contents, density (comfortable or compact lists), cards versus lists for Tasks on phones, the Save bar position;
  - "list what outside your paths needs changing"
