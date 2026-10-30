---
title: M3.6 ground rules (deltas on plan/_rules-common.md)
milestone: M3.6
---

# M3.6 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M3.6 deltas.

- **Contracts:** `plan/M3.6.md`, *Contracts fixed up front*, is the source of truth for every name, type and wire format shared between tasks.
- **fa-server does not build with the local rustc** (1.96; `mail-parser` needs 1.98). Test your own crates with `cargo test -p <crate>` and `cargo clippy -p <crate> --all-targets -- -D warnings`, and don't run `cargo build --workspace`. The main session checks the whole build in the Docker image (T-09).
- **Web checks** (from `apps/web`): `corepack pnpm@9 run typecheck`, `corepack pnpm@9 run lint`, `corepack pnpm@9 test`. If `node_modules` is missing, run `corepack pnpm@9 install --frozen-lockfile` first.
- **`apps/web/src/gen/**` is generated.** Web tasks don't edit it; the main session regenerates it after T-03.
- **Colours never appear as literals in components** after this milestone: use `var(--color-*)` and `var(--hue-*)`. The only literal colours allowed are in `themes/families/**`, calendar, category and task-list colours that come from data, and tests.
- **Public AGPL repo (D-035):** no household specifics in fixtures; member names in tests are fictional.
- **Don't invent palette values.** Colours in theme data come from each project's published palette (recorded `source` URL); anything derived is marked in a comment.
