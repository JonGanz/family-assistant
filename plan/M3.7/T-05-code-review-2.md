---
id: T-05
title: "Second code review: fa-mail internals, ICS and recipe fetch, tasks.rs, notify webhooks"
milestone: M3.7
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["plan/reviews/2026-11-02-code-review-2.md"]
decisions: [D-035, D-066]
---

# T-05: Second code review (read-only)

You are a sub-agent doing a time-boxed, **read-only** code review of the Family Assistant project, a self-hosted family organiser (Rust backend, Vue frontend, AGPL, public repo). You write exactly one file: `plan/reviews/2026-11-02-code-review-2.md`. You change no code.

## Read first
- `CLAUDE.md` (workflow 7 "Reviews", the standing constraints), `plan/_rules-common.md`
- `plan/reviews/2026-10-26-repo-review.md`: section "Code" and its "Coverage" paragraph, and the ID and table conventions (ID, Sev, Finding, Fix, Size, Status)
- `arch/Ingest.md`, `arch/Feeds.md`, `arch/Tasks.md`, `arch/Platform.md` §5 (Security) for the intended behaviour

## Scope (what the first review did not cover)
1. `crates/fa-mail/src/**`: IMAP and Gmail sync internals (connection, auth, fetch, idempotency of message ingestion, parsing of untrusted mail with `mail-parser`, size limits, attachment handling, error paths that could loop or leak).
2. `crates/fa-connectors/src/**` beyond `http.rs`, and `crates/fa-calendar/src/**`: ICS fetch and parse (untrusted input, recurrence expansion bounds, timezone handling), recipe or schema.org fetch if present.
3. `crates/fa-api/src/tasks.rs` and the services it calls in `crates/fa-tasks/src/**`: authorisation per route (children vs adults, device vs session), state machines (complete, reopen, skip, accept, claim, release), concurrency (`version`/`If-Match`), input validation.
4. `crates/fa-notify/src/**`: webhooks and ntfy delivery (SSRF, secrets in logs, retries, timeouts, topic handling).
Note that `fa-connectors/src/http.rs` got an SSRF guard in M3.5 (C-05); check that every outbound client in scope actually uses it.

## Method
Read the code (not only grep hits), follow each untrusted input to where it is stored or acted on, and check each route's authz against `fa-domain/src/authz.rs`. Run `cargo test -p <crate>` only if you need to confirm a suspicion (fresh container port 55405). Do not change files. Time-box: stop adding findings after about 90 minutes of reading and write the report.

## Output
`plan/reviews/2026-11-02-code-review-2.md` with front-matter `title, date, scope, status: open` and:
- a coverage paragraph (what you read, what you did not get to);
- one table per area with IDs `C2-NN` (continue numbering across tables), columns `ID | Sev (HIGH/MED/LOW) | Finding (file:line) | Fix | Size (QUICK/LARGER) | Status (todo)`;
- a "Clean" paragraph listing what you checked and found correct (as the first review did);
- a "Suggested order of work" list: QUICK fixes first, grouped so that each group touches disjoint paths (the main session turns each group into one sub-agent brief with strict ownership), then LARGER items, then anything that needs a user decision phrased as a question.
No household specifics in the report (public repo).

## Acceptance
- [ ] the file exists, has the front-matter above, and `python3 tools/fa.py check` still passes
- [ ] report (your final message): the number of findings per severity computed from your table, and the QUICK groups with their paths
