---
id: T-11
title: "Content caps, retention loop, job timeout (C2-06, C2-12, C2-04 job part)"
milestone: M3.7
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-rules/src/parse.rs", "crates/fa-rules/tests/**", "crates/fa-mail/src/sync.rs", "crates/fa-mail/src/retention.rs", "crates/fa-jobs/src/worker.rs", "crates/fa-jobs/src/lib.rs", "crates/fa-jobs/tests/**"]
decisions: [D-035]
---

# T-11: Content caps, retention loop, job timeout (C2-06, C2-12, C2-04 job part)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-06, C2-12, C2-04 (the per-job maximum runtime in the worker only), the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`. Not yours: `provider.rs`, `sources.rs` (T-08), `fa-jobs/src/scheduler.rs`, `handlers.rs`.

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). For the job timeout: a per-kind maximum runtime with a default (configurable through the existing jobs config if one exists, else a constant per kind with a doc comment); a timed-out job fails like any other error and is logged with its kind and id only. Keep `crates/fa-jobs/tests/**` building after T-02 changed `seal`/`open` signatures (T-02 owns those tests in wave 1; if they are mid-edit when you start, do not touch them and report).
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55411); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
