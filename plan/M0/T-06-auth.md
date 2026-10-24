---
id: T-06
title: fa-auth: passwords, sessions, devices, PIN elevation, Policy wiring
milestone: M0
wave: 4
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-auth/**"]
---

# T-06: fa-auth: passwords, sessions, devices, PIN elevation, Policy wiring

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §5 (Security)
- arch/Domain.md (RBAC matrix)
- decisions/D-006, D-019, D-031
- crates/fa-domain (the policy)

## You own
- `crates/fa-auth/**`

## Work
1. **Passwords:**
   - argon2id with OWASP-recommended parameters (constant in one place).
   - `hash_password`, and `verify_password`, which is constant-time and returns a needs-rehash hint.
   - The policy check comes from fa-domain (adult ≥12, child ≥4).
2. **Login service:**
   - Look up the account by username (through the RLS-safe lookup path from fa-store), then verify.
   - **Rate limit:** 5 failures per account in 15 minutes, which locks the account for 15 minutes, plus a per-IP limit. A simple in-memory per-IP token bucket, plus the DB-backed `user_account.failed_attempts` and `locked_until` columns (from T-04).
   - Session creation with a role-based TTL (adult 30 days, child 180 days, both rolling).
   - Logout. Listing and revoking sessions.
3. **Devices:**
   - `pair(adult, name)` makes a 32-byte random token (base64url), stores its SHA-256, and returns the token and `pair_url = {public_base_url}/dashboard#device={token}`.
   - `authenticate_bearer(token)` → `Device` (and it updates `last_seen` at most once a minute).
   - `revoke`.
4. **PIN:**
   - `set_pin(adult, pin)` hashes with argon2id.
   - `elevate(device, member_id, pin)` creates a `DeviceElevation` for 5 minutes. 5 failures (`member.pin_failed_attempts` and `pin_locked_until`, from T-04) lock it for 15 minutes and **record an audit entry** (a later notification hook is a TODO trait method).
5. **Actor resolution:** `resolve_actor(cookie?, bearer?) -> Actor`. A bearer token plus an active elevation becomes `ElevatedDevice`. This is framework-agnostic (no axum types), because fa-api adapts it.
6. **Tests:**
   - unit tests for hashing and policy;
   - DB tests for login success and failure, lockout, session TTL by role, device pairing and authentication, revocation, elevation success, elevation expiry, and elevation lockout;
   - an audit entry for each privileged operation.

## Acceptance
- [ ] `cargo test -p fa-auth` passes
- [ ] `cargo clippy -p fa-auth -- -D warnings` passes

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
