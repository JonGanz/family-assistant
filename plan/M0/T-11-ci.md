---
id: T-11
title: GitHub Actions CI + release, hygiene checks
milestone: M0
wave: 2
depends_on: [T-01, T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: [".github/**", "scripts/check-example-data.sh", ".gitleaks.toml"]
---

# T-11: GitHub Actions CI + release, hygiene checks

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Platform.md §5 (Supply chain, Public-repo hygiene), §10
- decisions/D-032, D-035

## You own
- `.github/**`
- `scripts/check-example-data.sh`
- `.gitleaks.toml`

## Work
1. `.github/workflows/ci.yml` runs on push and PR, with jobs:
   - **rust**: toolchain from rust-toolchain.toml, `Swatinem/rust-cache`, fmt, clippy `-D warnings`, `cargo nextest` (install with `taiki-e/install-action`) against a `pgvector/pgvector:pg16` service with `DATABASE_URL` set, and an offline check `SQLX_OFFLINE=true cargo check --workspace`.
   - **deny**: `EmbarkStudios/cargo-deny-action`.
   - **web**: corepack, `pnpm install --frozen-lockfile`, lint, typecheck, test, build.
   - **hygiene**: `gitleaks/gitleaks-action` (or the CLI), and `scripts/check-example-data.sh`, which fails if any email domain in `config/examples/**`, `**/fixtures/**`, or `e2e/**` is not `example.com`/`.org`/`.net` or `*.test`/`*.invalid`.
2. `.github/workflows/release.yml` runs on `v*` tags:
   - buildx multi-arch (amd64 and arm64) `fa-server` image from the root `Dockerfile` to `ghcr.io/${{ github.repository_owner }}/fa-server:{tag, latest}`, with OCI labels including the licence;
   - builds the `fa-gpu-agent` binary for x86_64-linux and attaches it to the GitHub release.
3. Pin every action to a full commit SHA, with the version in a comment. Use least-privilege `permissions:`.
4. Validate the workflows locally with `actionlint` if available (`go install` or the docker image `rhysd/actionlint`). Otherwise do a careful review.

## Acceptance
- [ ] actionlint passes (or explain why it couldn't run)
- [ ] `scripts/check-example-data.sh` passes on the current repo and fails on a temporary bad fixture (show both)

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
