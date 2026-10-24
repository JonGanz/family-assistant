---
id: T-12
title: Dockerfile, compose example, .env, deploy README, runbooks
milestone: M0
wave: 2
depends_on: [T-01, T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: ["Dockerfile", ".dockerignore", "deploy/compose.example.yaml", "deploy/.env.example", "deploy/README.md", "deploy/systemd/**", "deploy/runbooks/**"]
---

# T-12: Dockerfile, compose example, .env, deploy README, runbooks

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §8, §9
- decisions/D-005, D-010, D-020, D-025, D-026, D-033

## You own
- `Dockerfile`
- `.dockerignore`
- `deploy/compose.example.yaml`
- `deploy/.env.example`
- `deploy/README.md`
- `deploy/systemd/**`
- `deploy/runbooks/**`

## Work
1. **Dockerfile** (multi-stage):
   - `cargo-chef` planner and builder stages (with `SQLX_OFFLINE=true`), building `fa-server` in release mode.
   - A node stage: corepack pnpm, build `apps/web`.
   - Runtime: `debian:bookworm-slim` (or distroless cc) with ca-certificates, a non-root user, `/srv/web`, an entrypoint of `fa-server`, a default `CMD ["serve","all"]`, and a `HEALTHCHECK` on `/healthz`.

   Add a `.dockerignore`.
2. **deploy/compose.example.yaml** with profiles, as in Platform.md §8:
   - default: `postgres`, `api` (`serve api`, `127.0.0.1:8080:8080`), `worker` (`serve worker`), `ntfy` (`binwiederhier/ntfy`, `127.0.0.1:8090:80`, a cache volume), and `backup` (`prodrigestivill/postgres-backup-local`: daily schedule, `BACKUP_KEEP_DAYS=7`, `BACKUP_KEEP_WEEKS=4`, `BACKUP_KEEP_MONTHS=6`, `${BACKUP_DIR}` mounted);
   - `ai-cpu`: `s1-small` and `embed` as **commented, clearly marked placeholders** (M4);
   - `gpu`: `ollama` with the NVIDIA device reservation, and an `s1-large` placeholder.

   Each fa-server service has `image: ghcr.io/<owner>/fa-server:${FA_VERSION:-latest}` **and** `build: ..`. The master key is a compose `secrets:` entry from `${FA_MASTER_KEY_FILE}`. There are healthchecks and `depends_on` with conditions, and Postgres is not published.
3. **deploy/.env.example**, with every variable commented.
4. **deploy/README.md**: prerequisites, generating the master key (`head -c 32 /dev/urandom > fa_master_key && chmod 400`), **backing up the master key offline**, two-machine and single-machine layouts, `tailscale serve` commands for the app and ntfy, first-run steps (`config import`, `user add`, `device pair`), and a note on updates (the hoster's choice, D-033).
5. **deploy/systemd/fa-gpu-agent.service**: a unit template for M4, with a comment saying the binary is a stub until M4.
6. **deploy/runbooks/**:
   - `install.md`;
   - `restore.md`, the step list from Platform.md §9 (T-10 adds script references later);
   - `rotate-key.md`, which documents the planned procedure and marks the CLI command as TODO for M4+.

## Acceptance
- [ ] `docker compose -f deploy/compose.example.yaml config` validates with a sample env (for each profile)
- [ ] `docker build .` succeeds **if** T-03 to T-08 code exists by then; otherwise build the Rust stage against the current stubs and note it
- [ ] hadolint passes if available (`docker run --rm -i hadolint/hadolint < Dockerfile`)

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
