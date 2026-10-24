---
id: D-025
title: Exception to D-003: third-party model servers may run as Python containers
status: accepted
date: 2026-10-23
source_questions: [Q-024]
supersedes:
superseded_by:
affects: [arch/Overall.md]
research: [R-001]
---

# D-025: Exception to D-003: third-party model servers may run as Python containers

## Context
System One models run only on Python runtimes (R-001). The user picked option A.

## Decision
- **Our code stays Rust** (D-003 still applies).
- Third-party **model servers**, such as System One servers (`gliner2`, `strands-decider serve`, Kev), may run as containerised Python sidecars. Our code talks to them over HTTP only.
- Each sidecar is pinned to an image version and has a health endpoint. Its configuration lives in our config.

## Consequences
- The pinned images must be rebuilt for updates. Python code is never vendored into our repo.

## Alternatives considered
No Python at all (System 1 waits for ONNX or Rust builds). Not chosen.
