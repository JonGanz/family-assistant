---
id: D-076
title: One app container; no split layout
status: accepted
date: 2026-10-26
source_questions: [Q-071]
supersedes: 
superseded_by:
affects: [arch/Platform.md]
---

# D-076: One app container; no split layout

## Context
Today compose runs `api` and `worker` from one image.

## Decision
The compose file runs **one `app` container** (`serve all`). The user judged a single container sufficient for the application's scale, so no split override is shipped. The `serve api` and `serve worker` modes stay in the binary but are not part of the deployment story.

## Consequences
- Simpler compose, one healthcheck, one log.
- Background jobs share resources with the web side, acceptable at household scale.

## Alternatives considered
- A, configurable with a split override: 'on brand' but unnecessary.
- B, two containers.
