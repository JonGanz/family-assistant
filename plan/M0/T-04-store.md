---
id: T-04
title: fa-store: migrations, RLS, repositories, envelope crypto
milestone: M0
wave: 3
depends_on: [T-03]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-store/**", ".sqlx/ (root, sqlx offline data)"]
---

# T-04: fa-store: migrations, RLS, repositories, envelope crypto

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Domain.md (Persistence conventions, Identity & Access)
- arch/Platform.md §5 (Secrets), §7 (job tables)
- plan/M0.md (contracts, risk: RLS and pooling)

## You own
- `crates/fa-store/**`
- `.sqlx/ (root, sqlx offline data)`

## Work
1. **Migrations** in `crates/fa-store/migrations/` (sqlx, timestamped, reversible where practical) for: `household`, `member` (arrays for interests, hobbies, and responsibilities; `pin_hash` nullable; `pin_failed_attempts int`, `pin_locked_until timestamptz?`; `config_key text`, unique per household, used by config import; `archived_at`), `user_account` (unique username per household; `failed_attempts int`, `locked_until timestamptz?`), `session` (the shape `tower-sessions-sqlx-store` expects, **or** our own table plus a custom store. Choose one and document why), `device` (token_hash unique), `device_elevation`, `notify_pref`, `audit_entry` (append-only: **revoke UPDATE/DELETE** from the app role, via a trigger that raises), `secret`, `job`, `schedule` (columns as in Platform.md §7), and `system_status` (`key text PK`, `value jsonb`, `updated_at`; system-wide, not RLS-scoped; used by health and backup checks). Every table follows the conventions: UUID PKs, `household_id`, timestamptz, text and CHECK enums, `created_at`/`updated_at` (with an update trigger), and `version int` where Domain.md says so.
2. **RLS:** enable and **force** RLS on every household-scoped table, with the policy `household_id = current_setting('app.household_id')::uuid`. Some lookups have to happen before the household is known (login by username, device token lookup, healthz). Handle them safely, either through `SECURITY DEFINER` functions that return the minimal row, or through a separate BYPASSRLS role used only by `fa-auth`'s lookup paths. Pick one and justify it in a short ADR-style comment in the migration.
3. **API:**
   - `Store` (a PgPool wrapper).
   - `store.tx_for(household_id)` → a transaction with `SET LOCAL app.household_id`.
   - `store.tx_system()` for the system or worker paths that work across households.
   - Repositories for each M0 entity (insert, get, list, update with an optimistic `version` check → `Conflict` error, and archive), using compile-time-checked `sqlx::query!`/`query_as!`.
   - `migrate()` runs under a Postgres advisory lock.
4. **Envelope crypto** (`secret.rs`): `MasterKey::from_file(path)` (32 raw bytes, or 64 hex characters). `seal(plaintext) -> SealedSecret{ciphertext, nonce, wrapped_dek, key_version}` uses a random DEK with AES-256-GCM, and the DEK is wrapped by the master key with AES-256-GCM. Add `open(...)` and `SecretRepo` (put, get, rotate stub). Plaintext must never be logged: use a redacted `Debug`.
5. **Tests** (`#[sqlx::test]` against the dev DB): RLS isolation (household A can't see B, even with a raw `SELECT *`), the audit-log UPDATE/DELETE prohibition, optimistic concurrency conflicts, and a seal/open round trip plus tamper detection.
6. Generate the **offline data** with `cargo sqlx prepare --workspace` (install sqlx-cli if needed: `cargo install sqlx-cli --no-default-features --features rustls,postgres --locked`), so `SQLX_OFFLINE=true cargo build` works.

## Acceptance
- [ ] The dev DB is up (`docker compose -f deploy/compose.dev.yaml up -d`)
- [ ] `cargo test -p fa-store` passes
- [ ] `SQLX_OFFLINE=true cargo check -p fa-store` passes
- [ ] `cargo clippy -p fa-store -- -D warnings` passes

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
