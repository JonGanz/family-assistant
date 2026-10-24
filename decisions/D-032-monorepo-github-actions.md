---
id: D-032
title: Single repo (design + code) on GitHub with GitHub Actions CI
status: accepted
date: 2026-10-23
source_questions: [Q-030]
supersedes:
superseded_by:
affects: [arch/Platform.md]
---

# D-032: Single repo (design + code) on GitHub with GitHub Actions CI

## Context
The user picked option A.

## Decision
- Code lives in **this** repo, next to the design dirs (`arch/`, `design/`, `decisions/`, `log/`, `research/`), which stay where they are at the root.
- **GitHub Actions:**
  - `ci.yml` runs on every PR and push: Rust fmt, clippy (`-D warnings`), nextest against a Postgres service container, `cargo deny`, the sqlx offline check, and web lint, typecheck, vitest, and build.
  - `release.yml` runs on `v*` tags: it builds the multi-arch `fa-server` image (amd64 and arm64), pushes it to GHCR, and builds the `fa-gpu-agent` binary as a release asset.
- The repo never contains household data (D-004). CI uses only the example config and synthetic fixtures.

## Consequences
- Whether the repo and images are public or private is **[Q-033]**. It affects image pulls (a GHCR token is needed if private) and licensing.

## Alternatives considered
Self-hosted Forgejo, or a separate code repo. Not chosen.
