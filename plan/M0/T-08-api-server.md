---
id: T-08
title: fa-api + fa-server CLI: routes, SSE, OpenAPI, serve modes
milestone: M0
wave: 5
depends_on: [T-05, T-06, T-07]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-api/**", "bins/fa-server/**"]
---

# T-08: fa-api + fa-server CLI: routes, SSE, OpenAPI, serve modes

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- plan/M0.md (the HTTP API v1 contract, which is the source of truth)
- arch/Platform.md §3, §4, §5

## You own
- `crates/fa-api/**`
- `bins/fa-server/**`

## Work
1. **fa-api:**
   - An axum `Router` with every route in the contract table.
   - An `Actor` extractor (cookie through tower-sessions, or a bearer token) that calls `fa_auth::resolve_actor`.
   - A `require(action, resource)` helper that calls the domain policy.
   - CSRF: mutating methods need `X-FA-Request: 1`, otherwise 403 problem+json.
   - An error type that maps to `application/problem+json`.
   - The `version`/`If-Match` pattern on `PATCH /members/{id}` (conflict → 409).
   - tower-http trace, compression, a 30 s timeout, and request IDs.
2. **SSE hub:**
   - A broadcast channel, with events stamped by a monotonic id.
   - `/api/v1/stream` sends `heartbeat` every 15 s and `member.updated` on member changes.
   - It honours `Last-Event-ID` with a small ring buffer (the last 256 events).
   - It is scoped to the actor's household.
3. **OpenAPI** via utoipa at `/api/v1/openapi.json`, plus Swagger UI at `/api/docs` in dev only (`cfg` or a config flag).
4. **Static web:** serve `web_dist_dir` with an SPA fallback to `index.html` for non-API paths.
5. **fa-server CLI** (clap):
   - `serve {api|worker|all}`. `api` runs migrations under the advisory lock at startup, and `worker` runs the fa-jobs worker and scheduler.
   - `migrate`, `config import <file> [--replace]`, `config export [--out]`.
   - `user add --member <key> --username <u>` (password from a prompt or `--password-stdin`), `user reset-password`.
   - `device pair --name <n>` (prints the token and pair URL), `device revoke <id>`.
   - `pin set --member <key>`.
   - `gen-openapi --out apps/web/src/gen/openapi.json`.
   - Structured logging per `log_format`.
   - Graceful shutdown on SIGTERM.
6. **Integration tests** (spawn the router in-process against the dev DB):
   - log in as an adult and a child, and check `/me`;
   - a child gets 403 on `POST /members`;
   - device pair → bearer `/me` → elevate with the wrong PIN 5 times → locked;
   - a missing CSRF header gives 403;
   - SSE receives a heartbeat and a `member.updated` after a PATCH;
   - `/readyz` reports migrations current.

## Acceptance
- [ ] `cargo test -p fa-api -p fa-server` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Manual: `cargo run -p fa-server -- config import config/examples/household.example.toml`, then `user add`, then `serve all`, then `curl` login and `/api/v1/me` work (paste the transcript)
- [ ] `fa-server gen-openapi` writes `apps/web/src/gen/openapi.json`

## Ground rules (all M0 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M0.md` (especially the contracts), and the arch sections listed under *Read first*.
- **Edit only the paths you own** (listed below). If you need a change somewhere else, don't make it: say so in your report.
- Code style: idiomatic Rust 2024 edition, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs. Keep comments sparse and useful.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com`.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups or requests for other tasks' paths.
