---
id: T-14
title: "Migrate Settings II (calendars, mail, orgs, notifications, tasks, weather, system, status) to the design system"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/SettingsCalendarsView.vue", "apps/web/src/views/SettingsMailView.vue", "apps/web/src/views/SettingsOrgsView.vue", "apps/web/src/views/SettingsNotificationsView.vue", "apps/web/src/views/SettingsTasksView.vue", "apps/web/src/views/SettingsWeatherView.vue", "apps/web/src/views/SettingsSystemView.vue", "apps/web/src/views/SettingsStatusView.vue", "apps/web/src/stores/sources.ts", "apps/web/src/stores/mailSettings.ts", "apps/web/src/stores/mailSettings.test.ts", "apps/web/src/stores/rules.ts", "apps/web/src/stores/orgs.ts", "apps/web/src/stores/orgs.test.ts", "apps/web/src/stores/notifyPrefs.ts", "apps/web/src/api/mailSettings.ts", "apps/web/src/views/settings2.test.ts", "apps/web/src/views/parts/AbsencesPanel.vue", "apps/web/src/views/parts/AddSourceForm.vue", "apps/web/src/views/parts/DefinitionEditor.vue", "apps/web/src/views/parts/DefinitionsPanel.vue", "apps/web/src/views/parts/RoutinesPanel.vue", "apps/web/src/views/parts/RuleEditor.vue", "apps/web/src/views/parts/SourceCredentials.vue", "apps/web/src/views/parts/TaskListsPanel.vue", "apps/web/src/views/parts/definitionForm.ts", "apps/web/src/views/parts/definitionForm.test.ts", "apps/web/src/views/parts/listForm.ts", "apps/web/src/views/parts/routineForm.ts", "apps/web/src/views/parts/useDefinitionValidation.ts", "apps/web/src/views/parts/useRuleValidation.ts", "apps/web/src/views/parts/settings-mail/**", "apps/web/src/views/system/**", "apps/web/e2e/ui-shots.routes/settings2.ts"]
decisions: [D-095, D-096, D-052, D-054, D-058, D-061, D-075, D-072]
---

# T-14: Migrate Settings II (calendars, mail, orgs, notifications, tasks, weather, system, status) to the design system

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The design system (`apps/web/src/ui/`, T-02) and the new app shell with two reference pages (T-05) are in place and were reviewed by the user at checkpoints R1 and R2. You migrate the source, rule and system settings pages: **Calendars, Mail, Organizations, Notifications, Tasks, Weather, System and Status**. Five tasks migrate views in parallel in this wave, so stay strictly inside your paths.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Navigation*, *View ownership* (what is **not** yours), *Test files*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 and §1.1 (insecure contexts), §3 (source add flows, weather rule editor), §3a (Settings → Tasks, Notifications), §3b (Settings → Mail, Organizations), §3c (Settings, System)
- **The reference pages** from T-05: `apps/web/src/views/TasksView.vue` and `apps/web/src/views/SettingsHouseholdView.vue`, the "rules of the house" in T-05's report (quoted by the main session under *R2 feedback* below), and `apps/web/src/ui/` with `UiGallery.vue`
- `apps/web/e2e/*.spec.ts`: grep for the labels, roles and test ids your views use

## You own
The paths in `owns` above. `SettingsTasksView` uses the task components in `components/tasks/` (T-11 restyles them in parallel): use them as they are and report problems instead of editing them. **Not yours:** `ui/**`, `app/**`, `layouts/AppLayout.vue`, `router/**`, `gen/**`, `dashboard/**`, `kiosk/**`, and every view, component, store and test file of another wave-3 task (see *View ownership* in `plan/M4.md`).

## Work
1. **Sources (Calendars, Mail):** each connected source is a `UiCard` (outlined) with its state as a chip, last sync, and actions in a `UiMenu` (test, reconnect, remove with confirm). 'Add source' is a `UiFab` or top-bar action that opens a `UiSheet` choosing Google, CalDAV, ICS or IMAP, then the existing add flow as steps inside the sheet ('Test & discover', calendar picker). Credentials stay write-only. 'Google needs HTTPS' stays a fact line, not a warning (§1.1).
2. **Mail settings:** categories as a list with each category's behaviour as an instant `UiSelect` and notify and auto-action toggles as instant switches; rules with the condition builder and dry run in a sheet; the household mail settings as a form card with the Save bar.
3. **Organizations:** a list of orgs; an org opens in a sheet or detail with cards for Details, Contacts and Enrollments.
4. **Notifications:** per-member cards; trigger toggles instant with Undo; the minute and quiet-hours fields as a form; the subscribe URL with QR in a card; 'Send test' as a tonal button.
5. **Tasks settings:** sections for Lists, Definitions, Routines and Absences as cards with lists; editors in sheets (forms with Save).
6. **Weather:** the rule editor keeps its monospace field, debounce validation and the chip row (as `UiChip` assist chips).
7. **System:** each runtime setting in a card per group; fields locked by an environment variable show a lock icon and 'set by the server'; secrets 'set / not set' with replace and clear; restart-needed notes after save; the connection statement as plain text; the master-key backup card as a warning card. **Status:** cards per subsystem with state chips.
8. **Applying changes (D-096):** every switch, stand-alone single choice and list checkbox goes through `useInstantSave` (Undo snackbar). Every multi-field form goes through `useFormDraft` with `UiSaveBar`. Creating goes through a form in a `UiSheet`. Destructive actions confirm in a `UiDialog` unless they have Undo. Remove the page's own Save logic, toasts and banners in favour of these.
9. **Mobile-first layout:** a `UiPage` scaffold with `UiTopAppBar` (title, back on nested pages, actions). Collections are a `UiList` on compact and cards or a list in a card on wider screens. Each subject gets a `UiCard`. Empty, loading and error states use `UiEmptyState`, `UiSkeleton` and `UiErrorState`. Icons come from `ui/icons.ts` only.
10. **No page-local colours or ad-hoc kit classes:** only `--ui-*` tokens and `ui/` components. Remove uses of `PageHeader`, `StateBlock`, `ErrorBanner` and `Tabs` from your files (do not delete those components; the main session does after wave 3), and the old `base.css` kit classes (`.form`, `.card-list`, `.badge` and so on) from your templates.
11. **Keep behaviour:** guards, store calls, SSE invalidation, `If-Match` and 409 handling, and every existing feature of your views stay. This is a visual and interaction revamp, not a feature change. Keep the accessible names and test ids that `apps/web/e2e` uses; any change you cannot avoid goes in the report.
12. **Tests:** update your area's test file (`apps/web/src/views/settings2.test.ts`, split by the main session) and the tests next to the components and stores you own. Add at least one instant-save Undo test and one dirty-form test where your views have them.
13. Run `scripts/ui-shots.sh --all` for your routes (add them to the route constant in `apps/web/e2e/ui-shots.spec.ts` only if the main session made it data-driven per area; otherwise list the routes in the report).

## R2 feedback
(filled by main session before launch)

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
