---
id: T-14
title: "Follow-up: dashboard routine window and portrait layout bugs"
milestone: M2
wave: 7
depends_on: [T-12]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/dashboard/**", "apps/web/e2e/m2.spec.ts"]
---

# T-14: dashboard routine window and portrait layout bugs

You are a sub-agent fixing two bugs that the M2 e2e found in the dashboard (Vue 3, `apps/web`) of the Family Assistant project.

## Read first
- `plan/M2/_rules.md`; `arch/Frontend.md` §3a, §4 (kiosk layouts must fit 1920×1080 and portrait 1080×1920 without overlap)
- `apps/web/src/dashboard/tasksModel.ts`, `tasks.test.ts`, the layout presets in `apps/web/src/dashboard/layout/`, `apps/web/e2e/m2.spec.ts`

## Bugs
1. **Unpinned routine panel never shows.** `RoutineRunView.starts_at` and `done_by` are `"HH:MM"` strings, but `routineActive()` calls `Date.parse()` on them (NaN). Fix: compare the household-timezone `HH:MM` (and date vs `run_date`) of `now` with the strings. Fix the unit test fixtures in `tasks.test.ts` to use real `HH:MM` values. Remove `test.fail()` from the e2e spec "an unpinned routine panel shows the run inside its window".
2. **Portrait overlap.** The "Family hub" preset at 1080×1920 with a full Now list runs past the bottom and panels draw over each other. Fix the layout so panels get bounded heights and scroll/truncate internally (list overflow hidden with a "+N more" or internal scroll), never overlapping. Remove `test.fail()` from "Family hub lays out Now and the routine in portrait". 1920×1080 must stay correct.

## Acceptance
- [ ] `cd apps/web && pnpm test && pnpm run typecheck && pnpm run lint` pass
- [ ] You cannot run the full e2e cheaply; run `scripts/e2e.sh` only if needed — otherwise the main session runs it. Say which.
- [ ] Report: root causes, files changed
