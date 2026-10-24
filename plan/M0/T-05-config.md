---
id: T-05
title: fa-config: process config, household TOML, import/export
milestone: M0
wave: 4
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-config/**"]
---

# T-05: fa-config: process config, household TOML, import/export

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §6
- decisions/D-018 (hybrid config)
- decisions/D-004
- config/examples/household.example.toml

## You own
- `crates/fa-config/**`

## Work
1. **Process config** (`ProcessConfig`) via figment: defaults, then an optional `fa.toml`, then `FA_`-prefixed env vars (with `DATABASE_URL` accepted as is). Fields: database_url, bind_addr (default `127.0.0.1:8080`), master_key_path (default `/run/secrets/fa_master_key`), public_base_url, cookie_secure (bool, default true), log_format (`pretty|json`), log_level, web_dist_dir (default `/srv/web`), backup_status_path (optional). Include validation with clear error messages.
2. **Household config schema** (`HouseholdConfig`, serde): `schema_version = 1`, `[household]`, `[[members]]` (a `key` slug used as the stable identity across imports), with room for later sections (the TOML deserialiser should **reject unknown top-level sections**, with a helpful error that lists the supported ones).
3. **Import** (`import(store, household_id?, cfg, mode: Merge|Replace) -> ImportReport`):
   - Creates the household if it's missing.
   - Upserts members by `key`, which is stored in `member.config_key` (provided by T-04).
   - Never touches accounts, PINs, or secrets.
   - Writes an audit entry (actor System).
4. **Export** (`export(store, household_id) -> HouseholdConfig`): the redacted TOML. It never includes hashes, tokens, or secrets.
5. Round-trip test: import the example file, export it, and the result is semantically equal to the input (insta snapshot).

## Acceptance
- [ ] `cargo test -p fa-config` passes (it uses the dev DB through fa-store's test helpers)
- [ ] `cargo clippy -p fa-config -- -D warnings` passes

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
