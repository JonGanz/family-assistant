---
id: T-13
title: "Notify: duplicate sends, outage backoff, header length (C2-27, C2-28, C2-29)"
milestone: M3.7
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-notify/src/**", "crates/fa-notify/tests/**", "crates/fa-store/src/repo/notification.rs", "crates/fa-connectors/src/ntfy.rs"]
decisions: [D-035]
---

# T-13: Notify: duplicate sends, outage backoff, header length (C2-27, C2-28, C2-29)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-27, C2-28, C2-29, the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`. Not yours: `fa-jobs/**` (T-11), `fa-store/src/repo/*.rs` other than `notification.rs`.

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). C2-27: claim pending rows with `SELECT ... FOR UPDATE SKIP LOCKED` inside the send transaction (runtime queries), so an overlapping run sends nothing twice; test with two concurrent runs against a fresh DB. C2-28: only 4xx counts against `attempts`; transport errors and 5xx keep the row pending with a backoff capped at 15 minutes up to a maximum age constant (24 h, doc comment "to become a setting"). C2-29: title 120, body 2000 characters, truncated on a char boundary.
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55413); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
