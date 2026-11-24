---
id: Q-120
title: Gate releases on CI
status: open
batch: 27
area: infra
created: 2026-11-23
resolved:
decisions: []
---

# Q-120: Gate releases on CI

## Context
`ci.yml` runs only by hand (switched off on purpose), so pushing a `v*` tag publishes an image with no checks (V-01). `scripts/ci-local.sh` is the only gate and relies on someone running it. Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
Should CI run again, and should a release depend on it?

## Options
- A. Configurable, default: CI runs on pull requests and `v*` tags, and the release job needs it to pass (recommended)
- B. CI runs on pull requests only; tags publish without a gate
- C. Keep CI manual; the release checklist (`scripts/ci-local.sh`) is the gate

## Answer
<!-- answer below this line -->

