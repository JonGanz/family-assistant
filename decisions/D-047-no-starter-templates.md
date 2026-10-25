---
id: D-047
title: No starter task templates for now
status: accepted
date: 2026-10-25
source_questions: [Q-045]
supersedes: 
superseded_by:
affects: [arch/Tasks.md]
---

# D-047: No starter task templates for now

## Context
The user picked option A for now.

## Decision
- No template library and no template picker. Households add their own lists, chores, and routines.
- The example household TOML gets no task seed data either. Test fixtures may still create fictional tasks.

## Consequences
- The TOML import/export (D-018) still covers tasks, so a household can share or back up its own setup.
- A template library can be added later as a data file without schema changes.

## Alternatives considered
A template picker (B), plus example TOML seeds (C). Deferred.
