---
title: M5.3 ground rules (deltas on plan/_rules-common.md)
milestone: M5.3
---

# M5.3 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M5.3 deltas.

- **Contracts:** `plan/M5.3.md`; `arch/Frontend.md`.
- **Web checks** (in `apps/web`): `corepack pnpm@9 run lint`, `typecheck`, `test`, `build`, then `corepack pnpm@9 run format`. Don't run dev servers or bind 8080/5173; e2e and screenshots run in the main session.
- **UI rules:** colours only from tokens; icons only from `ui/icons.ts` (report a missing key); 48 px touch targets; `aria-label` on icon-only buttons. Kiosk text scales with the root font: size kiosk controls from `--kiosk-font-*`/`em`, not px (the shared `UiButton` is px-sized; see the M5.2 timer card for the pattern).
- **No new dependencies; no API, type or migration changes.** Report one if you think you need it.
