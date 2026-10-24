---
id: T-01
title: Workspace scaffold, dev compose, tooling
milestone: M0
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["Cargo.toml (root)", "rust-toolchain.toml", "deny.toml", ".cargo/", "justfile", ".gitignore", "crates/*/Cargo.toml + crates/*/src/lib.rs (stubs only)", "bins/*/ (stubs)", "deploy/compose.dev.yaml", ".env.dev", "config/examples/household.example.toml", ".editorconfig", "rustfmt.toml", "clippy.toml"]
---

# T-01: Workspace scaffold, dev compose, tooling

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §1, §2, §10

## You own
- `Cargo.toml (root)`
- `rust-toolchain.toml`
- `deny.toml`
- `.cargo/`
- `justfile`
- `.gitignore`
- `crates/*/Cargo.toml + crates/*/src/lib.rs (stubs only)`
- `bins/*/ (stubs)`
- `deploy/compose.dev.yaml`
- `.env.dev`
- `config/examples/household.example.toml`
- `.editorconfig`
- `rustfmt.toml`
- `clippy.toml`

## Work
1. Create the Cargo **workspace** (resolver 3, edition 2024, `license = "AGPL-3.0-or-later"`, and `[workspace.package]` shared metadata). The members are `crates/fa-domain`, `crates/fa-store`, `crates/fa-config`, `crates/fa-auth`, `crates/fa-jobs`, `crates/fa-api`, `bins/fa-server`, and `bins/fa-gpu-agent`. Each gets a minimal `lib.rs`/`main.rs` stub (fa-gpu-agent just prints "not implemented until M4").
2. **Pre-declare `[workspace.dependencies]`** for everything M0 needs, so later tasks only write `x.workspace = true`:
   - async and HTTP: tokio (full), axum, axum-extra (cookie, typed-header), tower, tower-http (trace, compression-gzip, fs, timeout, cors), tower-sessions, tower-sessions-sqlx-store (postgres);
   - DB: sqlx (runtime-tokio, tls-rustls, postgres, uuid, chrono, json, migrate, macros);
   - core: uuid (v7, serde), chrono (serde), chrono-tz, serde, serde_json, toml, figment (env, toml), thiserror, anyhow, tracing, tracing-subscriber (env-filter, json), clap (derive, env);
   - crypto: argon2, rand, aes-gcm, sha2, base64, hex, subtle;
   - API and types: utoipa (axum_extras, uuid, chrono), utoipa-swagger-ui (axum), ts-rs (uuid-impl, chrono-impl, serde-compat), futures, async-stream;
   - jobs: croner, tokio-util (rt), rand;
   - dev-only: insta, tempfile.

   Pin current stable versions and **verify they resolve** with `cargo check`.
3. Add `deny.toml` with advisories, bans, and an AGPL-compatible **licence allowlist** (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, MPL-2.0, Zlib, Unicode-3.0, Unicode-DFS-2016, CC0-1.0, AGPL-3.0-or-later, 0BSD, BSL-1.0, CDLA-Permissive-2.0).
4. Add `deploy/compose.dev.yaml`: `pgvector/pgvector:pg16`, user/password/db `fa`/`fa`/`fa`, port `127.0.0.1:55432:5432`, a named volume, and a healthcheck. Add `.env.dev` with `DATABASE_URL`.
5. Add `justfile` recipes: `dev-db`, `dev`, `test`, `lint`, `gen`, `e2e` (mirror Platform.md §10. A recipe that depends on later tasks can just echo TODO).
6. Add `config/examples/household.example.toml` with a fictional family: 2 adults and 2 children (one in grade 3, one in daycare), `example.com` emails, tz `America/New_York`, and a made-up location. Follow the format described in the plan contracts.
7. Add `.gitignore` entries for Rust `target/` and `node_modules/`, `.env` (but not `.env.example` or `.env.dev`), `apps/web/dist`, and the Playwright artefacts. Keep the existing Python cache entries.

## Acceptance
- [ ] `cargo check --workspace` and `cargo clippy --workspace -- -D warnings` pass
- [ ] `cargo fmt --check` passes
- [ ] `docker compose -f deploy/compose.dev.yaml up -d` brings up a healthy Postgres, and `SELECT extname FROM pg_available_extensions WHERE name='vector'` returns a row
- [ ] If `cargo-deny` is installable (`cargo install cargo-deny --locked`), `cargo deny check` passes; otherwise say so

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
