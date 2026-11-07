---
id: T-20
title: "Additive filter chips everywhere; Calendar reopens the last view"
milestone: M4
wave: 5
depends_on: [T-19]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/ui/UiFilterChips.vue", "apps/web/src/ui/index.ts", "apps/web/src/ui/components.test.ts", "apps/web/src/ui/UiGallery.vue", "apps/web/src/views/BillsView.vue", "apps/web/src/views/CalendarView.vue", "apps/web/src/views/TasksView.vue", "apps/web/src/views/MailView.vue", "apps/web/src/views/*.test.ts", "apps/web/src/stores/tasks.ts", "apps/web/src/stores/tasks.test.ts", "apps/web/src/stores/bills.ts", "apps/web/src/stores/bills.test.ts", "apps/web/src/stores/calendar.ts", "apps/web/src/stores/calendar.test.ts", "apps/web/src/stores/mail.ts", "apps/web/src/stores/mail.test.ts", "apps/web/src/components/tasks/MemberPicker.vue", "apps/web/e2e/m1.spec.ts", "apps/web/e2e/m2.spec.ts", "apps/web/e2e/m3.spec.ts", "e2e/mail.spec.ts"]
decisions: [D-103, D-105]
---

# T-20: Additive filter chips; Calendar remembers its view

You are a sub-agent on milestone M4 (UI revamp) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. The user reviewed the R3 screens and decided two changes. Every filter row becomes the same additive multi-select (D-105), and the Calendar reopens the view a person last used (D-103).

## Read first
- `plan/_rules-common.md`, `plan/M4/_rules.md`
- `decisions/D-103-calendar-last-view.md`, `decisions/D-105-additive-filter-chips.md`
- `apps/web/src/ui/UiChip.vue`, `ui/composables`, and the filter rows in `BillsView`, `CalendarView`, `TasksView` (member filter, `stores/tasks.ts` `FILTER_KEY`) and `MailView`

## You own
The paths in `owns`. If a store named in `owns` does not exist, keep the state in the view. Anything else you need changed goes in the report.

## Work
1. **`ui/UiFilterChips.vue`:**
   - Props: `options: {value: string, label: string, icon?: string, avatar?: ...}[]`, `modelValue: string[]` (empty means All), `allLabel` (default 'All'), `label` (the group's accessible name).
   - It renders a `group` with filter chips (`aria-pressed`). The first chip is All.
   - Rules (D-105):
     - choosing All clears the others;
     - choosing any other chip clears All and toggles that chip;
     - clearing the last chosen chip returns to All (an empty array).
   - Wraps on phones; no horizontal page scroll.
   - Export it from `ui/index.ts`, add it to `UiGallery`, and add unit tests for the three rules.
2. **Use it** for:
   - Bills state (keep the grouping by state; show only the chosen states' groups);
   - Calendar members;
   - Tasks members;
   - every Mail filter row that is a chip set (category, member, source, state).
   Where the API takes one value, filter on the client or send the list when the API accepts it. Read the route in `crates/fa-api` to check, and do not change Rust; report any API change needed.
3. **Tasks:**
   - `fa.tasks.filter` stores a JSON array of member ids.
   - Default: the viewer's own member, which is 'Mine'.
   - Migrate an old string value: `'all'` becomes `[]`, a member id becomes `[id]`, and an unknown id becomes the default.
   - Update the store and view tests.
4. **Calendar (D-103):** store the chosen view in `localStorage` key `fa.calendar.view` (`agenda|week|month`; wrap reads and writes in try/catch). When nothing is stored, keep the per-width default: Agenda on compact, Week otherwise. There is no setting.
5. **e2e:** update the selectors in the owned specs if the filter rows changed role or name. Do not run `scripts/e2e.sh` (the main session does); run the web checks.

## Acceptance
- [ ] `corepack pnpm@9 run lint`, `typecheck`, `test`, `build` in `apps/web` pass (copy each summary line into the report)
- [ ] Prettier is clean on the files you touched
- [ ] Never run `scripts/dev-run.sh`, never bind 8080/5173/18091, and run no state-changing git commands
- [ ] Report: files changed, the selector changes for e2e, any API change needed, and "list what outside your paths needs changing"

## Outcome (T-20 report)
- New `ui/UiFilterChips.vue` (group of `aria-pressed` buttons, All first, the D-105 rules, 4 unit tests, in the gallery). It draws its own buttons, because `UiChip` uses checkbox semantics.
- Bills state, Calendar members, Tasks members and Mail category, member and source use it. `fa.tasks.filter` is a JSON array, with old string values migrated; the own-member chip reads "Mine". `fa.calendar.view` remembers the view.
- The APIs take a single value, so several values are fetched per member (Calendar, Tasks) or filtered client-side on loaded pages (Bills, Mail); no Rust change. Mail has no state chip row.
- The main session removed Calendar's "Everyone" label so every row reads "All" (D-105).
- Web checks: lint 0 errors, typecheck, test (569 passed), build. e2e: the m3 Mail member test was updated.
- Follow-ups: multi-value API filters for Mail (several values filter only the loaded page); a `CalendarView` mount test for the stored view.
