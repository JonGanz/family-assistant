---
id: T-08
title: "IMAP and mail source hardening (C2-03, C2-04 IMAP part, C2-07, C2-10, C2-11)"
milestone: M3.7
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-connectors/src/imap.rs", "crates/fa-mail/src/provider.rs", "crates/fa-mail/src/sources.rs", "crates/fa-mail/src/lib.rs", "bins/fa-server/src/wiring.rs", "crates/fa-mail/tests/**"]
decisions: [D-035]
---

# T-08: IMAP and mail source hardening (C2-03, C2-04 IMAP part, C2-07, C2-10, C2-11)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. You fix a group of QUICK findings from the second code review.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md` (runtime queries only, no `cargo sqlx prepare`; no git state changes)
- `plan/reviews/2026-11-02-code-review-2.md`: the rows C2-03, C2-04 (the IMAP timeouts only), C2-07, C2-10, C2-11, the "Clean" paragraph for the invariants you must keep, and the matching "Suggested order of work" group
- The files you own, and the arch section named in each finding's area (`arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5)

## You own
The paths in `owns`: the IMAP client, the mail provider and sources, `MailConfig` (add `allow_private_hosts`), the server wiring that passes it, and the fa-mail tests. Not yours: `crates/fa-connectors/src/http.rs` (T-09), `crates/fa-mail/src/sync.rs` and `retention.rs` (T-11), `crates/fa-jobs/**` (T-11).

## Work
Fix each finding as its "Fix" column says, keeping behaviour configurable where the Fix names a setting (a constant with a doc comment is acceptable when the row says so). For C2-03 reuse the private-address predicate from `fa-connectors/src/http.rs` (import it; if it is private, ask for it to be made `pub` by reporting, and in the meantime add a `pub fn is_private_addr` next to it only if that file is not being edited, otherwise copy the minimal check into `imap.rs` with a comment). The runtime setting is the same `allow_private_hosts` CalDAV uses (D-075 System settings). GreenMail tests run with `FA_TEST_GREENMAIL`; if the container is not available, say so and run the rest.
For every finding add a test that fails before and passes after. Keep every "Clean" invariant. If a fix needs a path you do not own, do the minimum inside your paths and report the rest.

## Acceptance
- [ ] `cargo test -p <each crate you touched>` passes against a fresh container (port 55408); `cargo clippy -p <crates> --all-targets -- -D warnings`; `cargo fmt --check`; `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: per finding ID, what changed and the test that covers it; anything left for a LARGER item
