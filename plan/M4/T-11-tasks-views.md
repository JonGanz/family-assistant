---
id: T-11
title: "Migrate Tasks and Task detail to the design system"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/views/TasksView.vue", "apps/web/src/views/TaskDetailView.vue", "apps/web/src/components/tasks/**", "apps/web/src/stores/tasks.ts", "apps/web/src/stores/tasks.test.ts", "apps/web/src/stores/lists.ts", "apps/web/src/stores/definitions.ts", "apps/web/src/stores/taskMap.ts", "apps/web/src/stores/taskMap.test.ts", "apps/web/src/stores/away.ts", "apps/web/src/views/tasks.test.ts", "apps/web/src/views/parts/TaskAddForm.vue", "apps/web/src/views/parts/dateInput.ts", "apps/web/e2e/ui-shots.routes/tasks.ts"]
decisions: [D-095, D-096, D-042, D-045, D-046]
---

# T-11: Migrate Tasks and Task detail to the design system

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The design system (`apps/web/src/ui/`, T-02) and the new app shell with two reference pages (T-05) are in place and were reviewed by the user at checkpoints R1 and R2. You finish **Tasks** (`/tasks`, already migrated by T-05 as the reference; you restyle its child components) and migrate **Task detail and editor** (`/tasks/:id`) with the task components. Five tasks migrate views in parallel in this wave, so stay strictly inside your paths.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership* (what is **not** yours), *Test files*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 and §3a (Tasks)
- **The reference pages** from T-05: `apps/web/src/views/TasksView.vue` and `apps/web/src/views/SettingsHouseholdView.vue`, the "rules of the house" in T-05's report (quoted by the main session under *R2 feedback* below), and `apps/web/src/ui/` with `UiGallery.vue`
- `apps/web/e2e/*.spec.ts`: grep for the labels, roles and test ids your views use

## You own
The paths in `owns` above. `TasksView.vue` is T-05's reference page: keep its structure, adjust only what the task components need. **Not yours:** `ui/**`, `app/**`, `layouts/AppLayout.vue`, `router/**`, `gen/**`, `dashboard/**`, `kiosk/**`, and every view, component, store and test file of another wave-3 task (see *View ownership* in `plan/M4.md`).

## Work
1. **Task components** (`components/tasks/*`): `TaskItem` becomes a `UiListItem`-based row (leading checkbox with instant completion and the 10-second Undo, headline title, supporting due date relative with overdue in `--ui-error`, trailing assignee avatars, step count and the claim button for pool tasks). `MemberPicker` uses `UiChip` filter chips or a `UiSheet` list. `ProgressBar` becomes `UiProgress` (or wraps it). `StepList` uses `UiList` with checkboxes and drag handles (keep the current reorder behaviour). `RruleBuilder` uses `UiSegmented`, `UiSelect` and `UiTextField`. **`UndoToast` is replaced by `useSnackbar`**: remove its uses; delete the file only if nothing outside your paths imports it (otherwise report).
2. **Task detail:** the title as a large field in a card; then cards for Notes, Due and display (calendar display `hidden`, `until done`, `always` as a `UiSegmented` labelled 'key date' for `always`), Steps, and Assignment. Multi-field cards use `useFormDraft` with `UiSaveBar`; the priority flag and the display mode apply instantly. Children get the same editor where the list allows it (D-045).
3. **Create:** the 'New task' sheet T-05 added uses the same form fields as the detail (extract a shared form component inside `components/tasks/` if it helps).
4. **Applying changes (D-096):** every switch, stand-alone single choice and list checkbox goes through `useInstantSave` (Undo snackbar). Every multi-field form goes through `useFormDraft` with `UiSaveBar`. Creating goes through a form in a `UiSheet`. Destructive actions confirm in a `UiDialog` unless they have Undo. Remove the page's own Save logic, toasts and banners in favour of these.
5. **Mobile-first layout:** a `UiPage` scaffold with `UiTopAppBar` (title, back on nested pages, actions). Collections are a `UiList` on compact and cards or a list in a card on wider screens. Each subject gets a `UiCard`. Empty, loading and error states use `UiEmptyState`, `UiSkeleton` and `UiErrorState`. Icons come from `ui/icons.ts` only.
6. **No page-local colours or ad-hoc kit classes:** only `--ui-*` tokens and `ui/` components. Remove uses of `PageHeader`, `StateBlock`, `ErrorBanner` and `Tabs` from your files (do not delete those components; the main session does after wave 3), and the old `base.css` kit classes (`.form`, `.card-list`, `.badge` and so on) from your templates.
7. **Keep behaviour:** guards, store calls, SSE invalidation, `If-Match` and 409 handling, and every existing feature of your views stay. This is a visual and interaction revamp, not a feature change. Keep the accessible names and test ids that `apps/web/e2e` uses; any change you cannot avoid goes in the report.
8. **Tests:** update your area's test file (`apps/web/src/views/tasks.test.ts`, split by the main session) and the tests next to the components and stores you own. Add at least one instant-save Undo test and one dirty-form test where your views have them.
9. Run `scripts/ui-shots.sh --all` for your routes (add them to the route constant in `apps/web/e2e/ui-shots.spec.ts` only if the main session made it data-driven per area; otherwise list the routes in the report).

## R2 feedback
The user answered R2 on 2026-11-04 and 2026-11-05. These rules apply to every wave-3 page:
- **Shell (T-18, done):** phones get a menu button in the top bar that opens a modal drawer, with no bottom bar. Put contextual actions in `UiTopAppBar`'s `#actions` slot (at most two icon buttons plus an overflow `UiMenu` at 360 px). The desktop drawer collapses to the rail. Your page must use `UiPage` + `UiTopAppBar` (Settings children: `SettingsPage`), or phones get no menu button.
- **Save bar:** `UiSaveBar` is sticky at the bottom on every width. Use it for every multi-field form (D-096).
- **Density:** comfortable is the default; build for it, and check the compact density in the gallery does not break your page.
- **Collections on phones:** **cards on every width** (not a plain list on compact). `ResponsiveItems` should render a single-column card stack on compact.
- **Log out** stays at the foot of the navigation drawer. Do not add it to Settings → Account.
- Follow T-05's "rules of the house" and T-18's outcome (`plan/M4/T-05-app-shell.md`, `plan/M4/T-18-shell-r2.md`).
- **Tasks filter (user, 2026-11-05):**
  - **Chips:** a row of filter chips under the top bar: **All**, **Mine**, then one chip per member (`MemberChip` colour dot plus name, in member order). It is single-select and scrolls horizontally on phones.
  - **Default is Mine.** Mine means the signed-in member. On an elevated device it is the elevating adult; with no actor member, the default falls back to All.
  - **Server-side:** the filter is applied by the server. `GET /api/v1/tasks` already takes `member=<uuid>` (`TaskListQuery.member` in `crates/fa-api/src/tasks.rs`), so no API change is needed. Pass it from `stores/tasks.ts` and reset paging (`next_before`) when the chip changes.
  - **Per device:** the choice is remembered per device in `localStorage` key `fa.tasks.filter` (`"all"`, `"mine"` or a member id), with try/catch. An unknown member id falls back to Mine.
  - **Tests:** the default is Mine; picking a member sends `member=`; All sends none; the choice persists.

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

## Outcome (T-11 report)
- Built: Tasks with the R2 filter (All, Mine, members; default Mine; `fa.tasks.filter`; `member=` server-side), tabs Now, Lists and Suggested (the Mine tab is dropped); TaskDetail on `useFormDraft` + Save bar, with priority and display instant; TaskAddForm sheet; restyled `components/tasks/*`, new `StepEditor`; the store gains filter.
- Defaults applied: Mine tab dropped; up/down step reorder; separate create and edit forms.
- e2e: m2 tabs → radio (T-19). UndoToast stays for the kiosk.
