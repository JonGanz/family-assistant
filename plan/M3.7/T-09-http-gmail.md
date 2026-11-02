---
id: T-09
title: "HTTP guard gaps and Gmail message size (C2-01, C2-16)"
milestone: M3.7
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-connectors/src/http.rs", "crates/fa-connectors/src/google/**", "crates/fa-connectors/tests/**"]
decisions: [D-035]
---

# T-09: HTTP guard gaps and Gmail message size (C2-01, C2-16)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-01, C2-16, the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`. Not yours: `imap.rs` (T-08), `ics.rs` (T-10), callers in other crates (if renaming `Http::new` to `Http::permissive` breaks a caller outside `fa-connectors`, keep `new` as a deprecated alias of `permissive` and report the callers).

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). For C2-01 the per-request body cap for message fetches is a constant `GMAIL_MESSAGE_MAX_BYTES = 40 MB` with a doc comment pointing at Q-081 (a question opened today about the configurable limit); an over-limit message must not stop the sync: return a typed error the caller can skip (`ConnectorError::TooLarge` already exists; make sure `get_raw` maps it so that `fetch_many` in `fa-mail` can treat it per message, and report what `fa-mail` must do, since `provider.rs` is T-08).
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55409); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
