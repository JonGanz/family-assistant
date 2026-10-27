---
title: M3.5 ground rules (deltas on plan/_rules-common.md)
milestone: M3.5
---

# M3.5 ground rules

Read [`plan/_rules-common.md`](../_rules-common.md) first. These are the M3.5 deltas.

- **Contracts:** `plan/M3.5.md`, *Contracts fixed up front*, is the source of truth for every name, signature, DTO field and wire format shared between tasks.
- **The workspace may not build between waves.** W1 and W2 change types that `bins/fa-server` uses (`ProcessConfig`, `Http`, service configs); T-08 repairs the server in W4. Test your own crates with `cargo test -p <crate>` and `cargo clippy -p <crate> --all-targets -- -D warnings`. Run `cargo build --workspace` only if your brief asks, and report breakage outside your paths instead of fixing it.
- **SQL lives in fa-store** (T-02 only in M3.5). New repo code uses runtime `sqlx::query_as` + `FromRow`, so later tasks need no `.sqlx` regeneration. T-02 runs `cargo sqlx prepare --workspace` (with `SQLX_OFFLINE=false`) for any `query!` macros it touches.
- **Removals are part of the job.** D-068 removes advisory dismiss and the dashboard inbox; D-070 removes `/members`. Delete the code, its tests and its OpenAPI entries in your paths; don't leave dead branches.
- **Plain HTTP is a supported mode (D-072).** Nothing you write may assume HTTPS, set HSTS, or need a secure browser context.
- **Settings precedence (D-075):** env var > DB (`instance_setting`) > `FA_DEFAULT_*` env > built-in default. A value set by a plain env var is locked in the UI.
