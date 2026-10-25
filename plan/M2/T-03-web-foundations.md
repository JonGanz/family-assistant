---
id: T-03
title: "Web: task components, progress bar, member picker, RRULE helpers, layout kinds"
milestone: M2
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/components/tasks/**", "apps/web/src/lib/tasks.ts", "apps/web/src/lib/tasks.test.ts", "apps/web/src/lib/rrule.ts", "apps/web/src/lib/rrule.test.ts", "apps/web/src/dashboard/layout/**"]
---

# T-03: Web: task components, progress bar, member picker, RRULE helpers, layout kinds

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue 3 + TypeScript frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Domain types* (the DTO shapes), *Web ownership split*
- `arch/Frontend.md` §3a and §4.1 (M2 panels), `arch/Tasks.md` §4.4, §4.6, §4.7
- `apps/web/src/components/*` (Avatar, MemberChip, PinPad patterns and tests), `apps/web/src/lib/*` (time.ts, contrast.ts), `apps/web/src/dashboard/layout/index.ts` and its test, `apps/web/src/styles/base.css` (tokens)

## You own
Only the paths in the front-matter `owns` list.

## Work
The API types are generated after Wave 4, so components take **view-model props** defined in `lib/tasks.ts` as small TS interfaces that are structural subsets of the plan's DTOs (`TaskItemModel {id, title, dueAt?, status, priorityFlag, assignees: MemberRef[], stepsDone, stepsTotal, claimedBy?, origin}`, and so on). Wave 5 maps generated DTOs onto them. Use the plan's field meanings exactly.

1. `lib/tasks.ts`: `dueLabel(dueAt, now, tz)` (e.g. "today 18:00", "tomorrow", "Sat", "overdue 2 d"), `isOverdue`, `calendarDisplayLabel` ("Hidden", "Until done", "Key date"), `scheduleSummary(schedule)` ("Every 7 days after done", "Every Saturday", "1st of every month"), `seasonValid("MM-DD")`, `progress(done, total)`. Unit tests.
2. `lib/rrule.ts`: a small builder for the UI (weekly with weekdays, every N weeks, monthly by day-of-month, monthly by nth weekday) ↔ RRULE body strings (no DTSTART), with `parseSimple()` returning `null` for anything it can't represent (the UI then shows the raw text field). Unit tests, round-trip.
3. `components/tasks/`:
   - `TaskItem.vue` (checkbox, title, due label in red when overdue, assignee chips, step count, priority marker, a claim button slot; emits `complete`, `open`, `claim`), touch-friendly with a `size="kiosk"` variant.
   - `StepList.vue` (tick steps; emits `tick(stepId, done)`).
   - `ProgressBar.vue` (accessible: `role="progressbar"`, `aria-valuenow`; turns the success colour when complete; kiosk size). The user likes progress bars: make it look good in light and dark themes using the existing tokens.
   - `MemberPicker.vue` ("Who did it?": avatars of given members, a preselected id, emits `pick(memberId)` or `cancel`; large targets for kiosk).
   - `UndoToast.vue` (message, countdown, emits `undo`).
   - `RruleBuilder.vue` (uses `lib/rrule.ts`; falls back to a raw text input).
   Component tests with `@vue/test-utils` for each.
4. `dashboard/layout/**`: add panel kinds `now` (options `members?`, `lists?`), `member_tasks`, `routine` (option `pinned?`), with sizing in the grid and **presets**: add `now` and `routine` to "Family hub", add `member_tasks` to a new preset "Chores board". Unknown kinds stay ignored. Update the layout tests.

## Acceptance
- [ ] `corepack pnpm@9 test` passes (from `apps/web`)
- [ ] `corepack pnpm@9 run typecheck` and `corepack pnpm@9 run lint` pass
- [ ] `corepack pnpm@9 run format` was run
