---
id: T-07
title: fa-jobs: Postgres job queue, scheduler, M0 jobs
milestone: M0
wave: 4
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-jobs/**"]
---

# T-07: fa-jobs: Postgres job queue, scheduler, M0 jobs

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §7
- crates/fa-store (job and schedule tables)

## You own
- `crates/fa-jobs/**`

## Work
1. **Queue API:**
   - `enqueue(kind, payload, run_at?, priority?, idempotency_key?)`. A duplicate idempotency key is a no-op.
   - A worker loop claims jobs with `SELECT … FOR UPDATE SKIP LOCKED`, sets `locked_by` and `locked_until` (a lease), and dispatches to registered `JobHandler`s (an async trait: `kind()` and `run(ctx, payload)`).
   - Success marks the job done. Failure retries with exponential backoff and jitter up to `max_attempts`, then marks it `dead`.
   - Expired leases are reclaimed.
   - Graceful shutdown uses a cancellation token.
2. **Scheduler:** every 30 s it reads the enabled `schedule` rows (cron expressions, using `croner` (pre-declared in the workspace by T-01)), and enqueues due jobs with the idempotency key `schedule:{id}:{fire_time}`.
3. **M0 handlers:**
   - `session.cleanup` deletes expired sessions.
   - `backup.verify` finds the **newest dump file** under `backup_dir` (recursively; `prodrigestivill/postgres-backup-local` writes `daily/`, `weekly/`, `monthly/`, and `last/`, and no status file). It upserts `system_status['backup'] = {newest_file, mtime, age_hours, ok}` and logs a warning if the newest file is older than 26 hours. If `backup_dir` isn't configured, it logs info and returns ok.
4. Seed the default schedules through an idempotent `ensure_default_schedules()`.
5. **Tests:** concurrent workers never double-process, retry and backoff go to dead, lease reclaim, idempotency, and the scheduler enqueues exactly once per fire time.

## Acceptance
- [ ] `cargo test -p fa-jobs` passes
- [ ] `cargo clippy -p fa-jobs -- -D warnings` passes

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
