---
id: D-033
title: Ship a complete docker-compose sample; deployment/update strategy is the hoster's choice
status: accepted
date: 2026-10-23
source_questions: [Q-031]
supersedes:
superseded_by:
affects: [arch/Platform.md]
---

# D-033: Ship a complete docker-compose sample; deployment/update strategy is the hoster's choice

## Context
The user wants the deployment strategy left to whoever hosts it, with a docker-compose sample containing every service they need.

## Decision
- `deploy/compose.example.yaml` is one file with **profiles**, so any hardware layout can use it (D-026):
  - (default) `postgres`, `api`, `worker`, `ntfy`, `backup`
  - `ai-cpu`: `s1-small`, `embed`, for the core node
  - `gpu`: `ollama`, `s1-large` with the NVIDIA runtime, for a GPU node, or for a single machine that runs everything
- Every image has `image:` (GHCR tag) and a `build:` fallback, so hosters can pull or build.
- It comes with `.env.example`, comments on the docker secret for the master key, and a `deploy/README.md` covering both a two-machine layout and a single-machine layout.
- `fa-gpu-agent` ships as a native binary plus a systemd unit, because it needs host access. A container variant is documented as unsupported.
- **No prescribed update tool.** Our part is the versioned images, migrations that run on start (and are backward compatible within a minor version), and release notes.

## Consequences
- Hosters can run Watchtower, a manual pull, or anything else.
- Platform.md's `deploy/core` and `deploy/gpu` folders become a single compose sample with profiles.

## Alternatives considered
A prescribed `just deploy` flow. Declined.
