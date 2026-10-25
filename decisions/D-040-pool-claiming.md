---
id: D-040
title: Pool tasks: check off without claiming; optional claim that expires
status: accepted
date: 2026-10-25
source_questions: [Q-038]
supersedes: 
superseded_by:
affects: [arch/Tasks.md]
---

# D-040: Pool tasks: check off without claiming; optional claim that expires

## Context
The user picked option C.

## Decision
- Anyone in the pool can complete a pool task without claiming it.
- A member *can* claim it ("I'm on it"). The task then shows as theirs.
- A claim expires after a configurable time per list, default the end of the day, and can be released.

## Consequences
- `task.claimed_by` and `claimed_until`. `tasks.tick` clears expired claims.
- Refines D-013.

## Alternatives considered
No claiming (A) or claim-first (B). Not chosen.
