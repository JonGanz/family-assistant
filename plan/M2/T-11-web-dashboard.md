---
id: T-11
title: "Web dashboard: now, member_tasks, routine panels, chips, member picker"
milestone: M2
wave: 5
depends_on: [T-09, T-03]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/dashboard/** (except layout/)", "apps/web/src/kiosk/**", "apps/web/src/layouts/KioskLayout.vue", "apps/web/src/stores/dashboard.ts", "apps/web/src/stores/dashboard.test.ts", "apps/web/src/views/DashboardView.vue", "apps/web/src/views/DevicesView.vue"]
---

# T-11: Web dashboard: now, member_tasks, routine panels, chips, member picker

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue 3 + TypeScript frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *HTTP API v1 additions* (dashboard, tasks complete, step ticks, routine ticks, POST /tasks from a device), *Web ownership split*
- `arch/Frontend.md` §4 (kiosk, M2 panels and data), `arch/Tasks.md` §4.3, §4.6, §4.7
- `apps/web/src/dashboard/**`, `kiosk/**`, `stores/dashboard.ts`, `views/DevicesView.vue` (the layout editor), `apps/web/src/gen/` (regenerated after Wave 4), T-03's `components/tasks/**` and `dashboard/layout/**`

## You own
The paths in the front-matter `owns` list. If you need a change in T-03's components or the layout engine, report it instead of editing.

## Work
1. **Store:** the dashboard snapshot gains `tasks` and `routines`. SSE `tasks.updated` and `routines.updated` refetch those parts (or the whole snapshot; keep it cheap). The offline cache includes them.
2. **Panels:**
   - `now`: the Now list (open, visible tasks by `score`, limited to the household's `now_list_size`), filtered by the panel's `members`/`lists` options. Kiosk-size `TaskItem`s.
   - `member_tasks`: one column per dashboard member (device `members` setting) with their open tasks for today and overdue ones.
   - `routine`: visible automatically while any shown routine's window is active (`starts_at ≤ now < done_by`, household tz), or always when `pinned`. One column per member: big step buttons, `ProgressBar`, a celebratory but calm "done" state. Steps tick through `PATCH /routine-runs/{id}/steps/{step}` with `member` = the run's member (no picker needed).
   - `agenda`: add key-date tasks (`calendar_display = always`) as distinct items. `week`: add `until_done` and `always` task chips on their due day.
3. **Completion on the dashboard:** tapping a task's checkbox opens `MemberPicker` ("Who did it?", preselecting the single assignee), then `POST /tasks/{id}/complete {member}`, then an `UndoToast` for 10 seconds. While `dashboard_completion_override` is off, completion is hidden (show tasks read-only).
4. **Add from the dashboard:** a "+" on the `now` panel when some list has `dashboard_add`: a large on-screen form (title, list picker limited to `dashboard_add` lists, optional assignee) and the member picker for who's adding it.
5. **Layout editor (DevicesView):** the new panel kinds and their options (members and lists filters for `now`, `pinned` for `routine`), and the new preset from T-03. The live preview renders the new panels with sample data.
6. Tests (vitest): panel rendering from a snapshot, routine window visibility (tz), the picker flow, read-only mode with the override off, the SSE refetch, the offline cache with tasks.

## Acceptance
- [ ] `corepack pnpm@9 test`, `run typecheck`, `run lint`, and `run build` pass (from `apps/web`)
- [ ] `corepack pnpm@9 run format` was run
- [ ] Manually check the kiosk page at 1920×1080 (landscape) and portrait with the example household on a throwaway stack (see T-10's note: **never use the dev DB's existing real data**). Describe what you checked.

## Wave 1 outcomes (T-03)
- `lib/tasks.ts` view-models: `TaskItemModel`, `MemberRef`, `StepModel`, `ScheduleModel`; helpers `dueLabel`, `isOverdue`, `calendarDisplayLabel`, `scheduleSummary`, `seasonValid`, `progress`. Map generated DTOs onto them (`assignees` ids → `MemberRef`, `stepsDone`/`stepsTotal` from `steps`).
- `components/tasks/`: `TaskItem` (`size="kiosk"`), `StepList`, `ProgressBar` (`size="kiosk"`), `MemberPicker` (`size="kiosk"`), `UndoToast` (emits `undo` and `expire`, 10 s default), `RruleBuilder` (`v-model`).
- Layout: kinds `now`, `member_tasks`, `routine`; "Family hub" gains `now` and `routine`; new preset "Chores board". Panel options (`members`, `lists`, `pinned`) are loose `PanelConfig` keys, not typed fields.

## Wave 4 outcomes (read before starting)
- `apps/web/src/gen` is regenerated (`api.d.ts`, `openapi.json`, `types/`) with every M2 route and DTO. The main session added `tasks: [], routines: []` to `dashboard/preview.ts` so the typecheck passes (T-11 fills real sample data).
- T-09 behaviour worth knowing: complete/reopen accept an empty body; devices must send `member`; `POST /members/{id}/notify-prefs/test` returns **204** on success, 503 when ntfy isn't configured, 502 on a publish error; devices may read `/task-lists` and `/routines`; a child adding a task gets assignees forced to themself (naming anyone else is 403); a child gets 404 for another member's task. Validation errors are 422 problem+json with an extra `field` (and `position` for DSL errors).
- CLI for local checks: `fa-server tasks run-now` (one pass of materialize, tick, routines) and `fa-server notify test --member <key>`.
