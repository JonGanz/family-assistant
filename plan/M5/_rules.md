---
title: M5 ground rules (deltas on plan/_rules-common.md)
milestone: M5
---

# M5 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M5 deltas.

- **Contracts:** `plan/M5.md`, *Contracts fixed up front*, is the source of truth for table and column names, types, routes, job names, SSE events and wire formats. `arch/Meals.md` v1 and `arch/Frontend.md` §3f and §4 describe the behaviour.
- **Open questions (batch 25):** build the working assumptions and nothing else: one entry per date and slot (Q-115), one aisle order for all stores (Q-116), templates made only in the editor (Q-117). Keep these easy to change (one unique index, one ordering query, no "save as template" UI). Don't hard-code more around them.
- **Disk:** run `df -h /` before every build and stop below 6 GB free. Build with `CARGO_INCREMENTAL=0` and build or test **only your own crates** (`-p`). Run `cargo check --workspace` once at the end. Never use `--release`.
- **Dependencies:** the only new ones are `pluralizer` and `image` (features `jpeg`, `png`, `webp`, no defaults). The main session has added them to the root `Cargo.toml`. Use them as `workspace = true`. Report anything else you think you need; don't add it.
- **SQL:** do not run `cargo sqlx prepare`. Use runtime `sqlx::query` / `query_as` for every query you add.
- **Deterministic first (CLAUDE.md):** the normaliser, the matching, generation and reconciliation are pure rules with table tests. The LLM is only used in `ingredient.llm` and the existing `fa-recipe` fallback. Its results are proposals that an adult confirms. Tests use an `axum` stub server, never a real LLM.
- **Real data:** never import, generate or reconcile against the dev DB. Tests use `#[sqlx::test]` or a fresh container (port 55400 + task number).
- **Fixtures (D-035):** recipe pages, images and ingredient lines are fictional and written for the repo. Don't copy recipe text or photos from any website. Generate test images in code (`image` crate), and don't commit photos.
- **Web checks** (in `apps/web`): `corepack pnpm@9 run lint`, `typecheck`, `test`, `build`, then `corepack pnpm@9 run format`. Only the main session regenerates `apps/web/src/gen/**`, after wave 3, so web tasks start from the generated types.
- **UI rules:** the same as M4. Colours come only from `--ui-*` tokens. Icons come only from `ui/icons.ts` (a new icon goes in `ui/icons.ts`, which T-07 owns; other tasks report the icon keys they need). Layouts are mobile-first at 360 px, then `min-width` 600 and 840 px. Touch targets are 48 px or more, and every icon-only button has an `aria-label`. Apply changes per D-096. Filter rows follow D-105.
- **Kiosk:** the kiosk never shows email-derived data (D-068). Adding and checking grocery items needs no PIN. Edits and deletes go through the existing adult-PIN elevation.
- **Local rustc is the stable toolchain** (`rust-toolchain.toml`); `image` 0.25.10 needs rustc 1.88 or later.
