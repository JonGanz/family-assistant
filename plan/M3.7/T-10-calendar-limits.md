---
id: T-10
title: "Calendar expansion limits (C2-13, C2-14, C2-17, C2-18)"
milestone: M3.7
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-calendar/src/expand.rs", "crates/fa-calendar/src/remote.rs", "crates/fa-calendar/tests/**", "crates/fa-connectors/src/ics.rs"]
decisions: [D-035]
---

# T-10: Calendar expansion limits (C2-13, C2-14, C2-17, C2-18)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-13, C2-14, C2-17, C2-18, the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`. Not yours: `crates/fa-calendar/src/service.rs`, `caldav.rs`, `http.rs`.

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). Add the hostile-feed fixture test the review asks for (`DURATION:P99999999999W`, a multi-MB `DESCRIPTION` on a daily rule, `FREQ=SECONDLY`) and check it never panics and stays under the caps. Fixture data is fictional (public repo).
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55410); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
