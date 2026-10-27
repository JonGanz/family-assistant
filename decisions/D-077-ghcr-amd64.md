---
id: D-077
title: Release images on GHCR, amd64 only for now
status: accepted
date: 2026-10-26
source_questions: [Q-072]
supersedes: 
superseded_by:
affects: [arch/Platform.md, README.md]
---

# D-077: Release images on GHCR, amd64 only for now

## Context
Installs built the image on the server (4 GB RAM, 15 GB disk, up to 30 min).

## Decision
Option A, limited to `linux/amd64`. CI builds and pushes `ghcr.io/jonganz/fa-server` on `v*` tags; the install is download `compose.yaml`, `docker compose up -d`. Building from a checkout stays for developers and for other architectures.

## Consequences
- ARM64 hosts (some NAS, Raspberry Pi) build locally until arm64 is added; adding it later is a CI matrix change.
- Human step: enable GHCR publishing and make the package public.

## Alternatives considered
- B, local build only.
- C, both with build first.
