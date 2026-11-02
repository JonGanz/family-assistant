---
id: T-12
title: "Tasks: suggested visibility, races, child reassignment, validation (C2-20, C2-22 to C2-26)"
milestone: M3.7
wave: 2
depends_on: [T-03]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-api/src/tasks.rs", "crates/fa-api/tests/**", "crates/fa-tasks/src/**", "crates/fa-store/src/repo/task.rs", "crates/fa-store/tests/**"]
decisions: [D-035]
---

# T-12: Tasks: suggested visibility, races, child reassignment, validation (C2-20, C2-22 to C2-26)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-20, C2-22, C2-23, C2-24, C2-25, C2-26, the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`. T-03 (wave 1) changed `list_tasks` to return `TaskPage {items, next_before}` and edited `fa-tasks/src/service.rs`; build on the working tree as you find it. Not yours: `fa-domain/**` (if a limit or error variant must live there, report it and use `field_invalid` with an existing variant), `fa-api/src/lib.rs`, `apps/web/**`.

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). C2-22: use `SELECT ... FOR UPDATE` in the task-opening read inside the transaction for complete, reopen, skip, claim and release, plus a conditional `set_claim`; runtime queries only. C2-20: a device, elevated or not, never sees `suggested` tasks; add the viewer matrix test (child, adult, device, elevated device). C2-25 limits: title 200, notes 10 000, 50 steps of 200 characters; state them in the OpenAPI descriptions of the request DTOs you can reach from `tasks.rs` (the DTOs live in `fa-domain`, so put the limits in the handler docs and report the DTO doc lines to add).
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55412); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
