---
title: M5.1 ground rules (deltas on plan/_rules-common.md)
milestone: M5.1
---

# M5.1 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M5.1 deltas.

- **Contracts:** `plan/M5.1.md`, *Contracts fixed up front*, is the source of truth. `arch/Meals.md` §3, §4.1 (2a, 2b) and `arch/Frontend.md` §3f and §4 (Recipe window) describe the behaviour. Decisions D-124 and D-125.
- **Disk:** `df -h /` before building; stop below 6 GB. `CARGO_INCREMENTAL=0`, build and test only your crates (`-p`), `cargo check --workspace` once at the end, never `--release`.
- **SQL:** runtime `sqlx::query` / `query_as` only; do not run `cargo sqlx prepare`.
- **No new dependencies.** Report one if you think you need it.
- **Fixtures (D-035):** fictional recipes written for the repo; never copy text from a real site.
- **Web checks** (in `apps/web`): `corepack pnpm@9 run lint`, `typecheck`, `test`, `build`, then `corepack pnpm@9 run format`. Only the main session regenerates `apps/web/src/gen/**`. Don't run dev servers or bind 8080/5173; e2e runs in the main session.
- **UI rules:** colours only from tokens; icons only from `ui/icons.ts` (frozen: report a missing key); 48 px touch targets; `aria-label` on icon-only buttons; changes apply per D-096.
