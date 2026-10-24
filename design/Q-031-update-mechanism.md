---
id: Q-031
title: How updates are deployed
status: open
batch: 5
area: infra
created: 2026-10-23
resolved:
decisions: []
---

# Q-031: How updates are deployed

## Question
How should new versions reach the core node?

## Options
- A: CI builds and pushes images to a registry; on the server `just deploy` pulls a pinned tag and restarts (manual, deliberate) (recommended)
- B: build on the server from git (`git pull && docker compose build`)
- C: automatic updates (e.g. Watchtower on a release tag)

## Answer
<!-- answer below this line -->

