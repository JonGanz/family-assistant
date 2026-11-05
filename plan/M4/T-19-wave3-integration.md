---
id: T-19
title: "Wave-3 integration: e2e selectors, ui/ gaps, Settings list icons, dead components"
milestone: M4
wave: 4
depends_on: [T-10, T-11, T-12, T-13, T-14, T-18, T-15, T-16]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/e2e/m1.spec.ts", "apps/web/e2e/m2.spec.ts", "apps/web/e2e/m3.spec.ts", "apps/web/e2e/m35.spec.ts", "apps/web/e2e/m36.spec.ts", "apps/web/e2e/smoke.spec.ts", "apps/web/e2e/wizard.spec.ts", "apps/web/e2e/ui-shots.routes/**", "apps/web/src/ui/UiSelect.vue", "apps/web/src/ui/UiTextField.vue", "apps/web/src/ui/UiButton.vue", "apps/web/src/ui/icons.ts", "apps/web/src/ui/index.ts", "apps/web/src/ui/components.test.ts", "apps/web/src/views/SettingsLayout.vue", "apps/web/src/components/PageHeader.vue", "apps/web/src/components/StateBlock.vue", "apps/web/src/components/ErrorBanner.vue", "apps/web/src/components/Tabs.vue", "apps/web/src/components/Tabs.test.ts", "apps/web/src/components/pageShell.test.ts", "apps/web/src/components/components.test.ts", "apps/web/src/layouts/AppLayout.vue", "apps/web/src/app/**", "apps/web/src/views/MessageView.vue", "apps/web/e2e/ui-shots.spec.ts"]
decisions: [D-095, D-096]
---

# T-19: Wave-3 integration

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. Wave 3 (T-10 to T-14) moved every view onto the `ui/` design system. Their reports list e2e selectors that no longer match and small gaps in `ui/`. You fix those in one pass, so the e2e suite is green again.

## Read first
- `plan/_rules-common.md`, `plan/M4/_rules.md`, `plan/M4.md`
- The `## Outcome` sections of `plan/M4/T-10-*.md` to `T-14-*.md`, `T-05-app-shell.md` and `T-18-shell-r2.md`. Each lists the selector changes ("e2e selectors changed") and the `ui/` gaps.
- `apps/web/e2e/*.spec.ts`, `scripts/e2e.sh`

## You own
The paths in `owns`. **Not yours:** the views, stores and parts (wave-3 tasks are done, but leave their files alone unless a test id is missing, and then report it), `SettingsAiView`, `settingsNav.ts`, `router/**`, `InboxView`, `MessageView` and `components/mail/**` (T-15 and T-16 run in parallel).

## Work
1. **e2e specs:** update every selector the wave-3 outcomes list. Prefer roles and labels, and use the test ids the views kept. Do not weaken an assertion; if a behaviour was removed, say so in the report. Notable changes:
   - Segmented controls are `radio` (Calendar views, Inbox tabs, Tasks tabs, Appearance mode).
   - Create forms open from a FAB into a sheet (calendars, mail sources, definitions, absences, weather rules, members in the wizard, device pairing).
   - The Account password uses the Save bar.
   - Inbox Reject opens a confirm dialog.
   - Labelling uses new test ids and an eval switch.
   - Mail source actions are in an "Actions for <name>" menu.
   - The Absences submit button is "Save".
2. **Run `scripts/e2e.sh`** (Docker; it builds the server and starts its own stack on the 180xx ports, never 8080 or 5173). Fix the specs until it passes. If a failure is a real bug in a view, fix it only if the change is a test id or an accessible name in one line, and list it. Otherwise report it with the failing step.
3. **`ui/` gaps:**
   - `UiSelect` and `UiTextField` forward `name`, `id` and other attrs (`inheritAttrs: false` with `v-bind="$attrs"` on the native control, and `class` on the wrapper).
   - `UiButton` takes `to` (a RouterLink) and `href` (an `<a>`).
   - `icons.ts` adds `arrow_upward`, `arrow_downward` and `menu_open` (rounded, both styles).
   - `ui/index.ts` re-exports `navControlKey`.
   - Add tests for each.
4. **`SettingsLayout`:** read `icon` and `summary` from `settingsNav.ts` (T-13 added them; drop its own `iconFor` fallback). Do not edit `settingsNav.ts`; T-15 adds the AI entry to it in parallel.
5. **Dead components:**
   - Delete `PageHeader.vue`, `StateBlock.vue`, `ErrorBanner.vue`, `Tabs.vue`, their tests and their test cases in `components.test.ts`. First grep that no `.vue`/`.ts` outside tests imports them.
   - Keep `components/tasks/UndoToast.vue`: the kiosk `dashboard/TaskFlow.vue` uses it.
   - Remove the `:deep` padding fallbacks for unmigrated pages in `AppLayout.vue` (every page is migrated now).
6. **Screenshot routes:** add `/login` (signed out), the setup wizard first step if reachable in the e2e stack, a task detail (open the first task from `/tasks` in a `before` hook), a message detail (first message on `/mail`), and the opened create sheets: Settings → Calendars "Add a source", Tasks FAB and Members FAB. Use `before` hooks in the area files.

7. **AI chips on derived items:** `DerivedEvent` and `DerivedTask` now carry `extracted_by` (main session, regenerated in `gen/`). In `MessageView.vue`, show `components/mail/AiChip.vue` on derived events and tasks when `extracted_by === 'llm'`, as T-16 did for derived bills. `TaskView` has no confidence, so the chip shows "AI".
8. **Screenshots of the AI pieces with mocked data:** the e2e stack has no LLM server and no LLM-made items. In the area files (or a new `ui-shots.routes/ai.ts` registered in `ui-shots.spec.ts`), use Playwright `page.route` to answer with fictional data:
   - `/inbox` with one event and one bill where `extracted_by: 'llm'`;
   - `/settings/ai` with a configured endpoint (`http://gpu-pc.example:11434/v1`, `qwen3:8b`, enabled), a successful test result, the four pipeline points (one enabled), and stats with a few calls.
   Use the generated DTO shapes. Name the shots `ai-inbox` and `ai-settings`.

## Acceptance
- [ ] `scripts/e2e.sh` passes (quote its summary line)
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test`, `build` pass; `format` run (prettier on your paths only)
- [ ] `scripts/ui-shots.sh --all` runs to completion (it may take a while; quote the file count)
- [ ] Report (`plan/_rules-common.md` format): the selectors changed per spec, view bugs found, anything left

## Outcome (T-19 report)
- `scripts/e2e.sh` green (API and UI specs, "e2e OK"); `scripts/ui-shots.sh --all` wrote 190 files (33 routes, 3 widths, light and dark). Web lint 0 errors, typecheck, test and build pass.
- ui/ gaps closed: `UiSelect`/`UiTextField` forward attrs to the native control, `UiButton` takes `to`/`href`, icons `arrow_upward`, `arrow_downward`, `menu_open`, `navControlKey` exported. `SettingsLayout` reads `icon`/`summary`; `:deep` padding rules removed.
- Deleted `PageHeader`, `StateBlock`, `ErrorBanner`, `Tabs` and their tests; `UndoToast` kept for the kiosk.
- `MessageView` shows `AiChip` on derived events and tasks with `extracted_by === 'llm'`.
- New shot routes: `ai.ts` (mocked `ai-inbox`, `ai-settings`), `login`, `tasks-detail`, `tasks-new-sheet`, `mail-message`, `settings-calendars-add-source`, `settings-members-add-sheet`.
- Outside owns (accepted by the main session): `ui/UiListItem.vue` passed `:href="undefined"` to RouterLink, so Settings nav rows had no link role; fixed with a conditional `v-bind`. Root `e2e/mail.spec.ts` reads `.items` pages.
- Follow-ups: `MembersManager` shows a member row after the first POST while the login is created by a second one; a reload in between leaves a child without a login (the wizard spec now waits for both). No wizard screenshot (separate LAN-bound stack). Detail shots depend on seeded data.
