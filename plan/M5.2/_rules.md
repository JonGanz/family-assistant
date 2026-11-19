---
title: M5.2 ground rules (deltas on plan/_rules-common.md)
milestone: M5.2
---

# M5.2 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M5.2 deltas.

- **Contracts:** `plan/M5.2.md`; decision D-126; `arch/Meals.md` §4.1 2c, `arch/Frontend.md` (Recipe page, Timers view).
- **No API, type, migration or dependency changes.** Report one if you think you need it.
- **Web checks** (in `apps/web`): `corepack pnpm@9 run lint`, `typecheck`, `test`, `build`, then `corepack pnpm@9 run format`. Don't run dev servers or bind 8080/5173; e2e runs in the main session.
- **UI rules:** colours only from tokens; icons only from `ui/icons.ts` (report a missing key); 48 px touch targets; `aria-label` on icon-only buttons.
- **Fixtures (D-035):** fictional recipes only.
