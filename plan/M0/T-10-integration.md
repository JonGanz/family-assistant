---
id: T-10
title: Integration: e2e smoke, compose boot test, backup/restore rehearsal script
milestone: M0
wave: 6
depends_on: [T-08, T-12]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["e2e/**", "scripts/**", "deploy/runbooks/restore.md (update only)"]
---

# T-10: Integration: e2e smoke, compose boot test, backup/restore rehearsal script

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §8, §9, §11
- deploy/ (from T-12)
- apps/web/e2e (from T-09; coordinate: if it's missing, write the spec yourself under `e2e/`)

## You own
- `e2e/**`
- `scripts/**`
- `deploy/runbooks/restore.md (update only)`

## Work
1. `scripts/e2e.sh`:
   - builds the image (or uses `cargo run` and `pnpm build` in dev mode);
   - brings up a clean Postgres;
   - runs `config import` of the example and creates an adult and a child user;
   - starts `serve all`;
   - runs Playwright `smoke.spec.ts` headless (install browsers with `corepack pnpm@9 exec playwright install chromium`);
   - tears down.
2. `scripts/compose-smoke.sh`:
   - `docker compose -f deploy/compose.example.yaml --env-file <generated test env> up -d` (default profile) with a temporary master key and a temporary `BACKUP_DIR`;
   - waits for `/readyz`;
   - triggers an immediate backup (the backup container's on-demand mechanism) and checks that a dump file appears in `BACKUP_DIR`;
   - tears down with volumes.
3. `scripts/restore-rehearsal.sh <dump>`: restores into a fresh Postgres container, starts api against it with the same master key, checks `/readyz`, and checks that the member count matches. Update `deploy/runbooks/restore.md` to reference it.
4. Fix nothing in other tasks' code. Report any failures with logs.

## Acceptance
- [ ] `scripts/e2e.sh` passes
- [ ] `scripts/compose-smoke.sh` passes
- [ ] `scripts/restore-rehearsal.sh` passes against the dump that compose-smoke produced

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
