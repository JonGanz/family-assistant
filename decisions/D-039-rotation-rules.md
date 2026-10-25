---
id: D-039
title: Rotation: configurable advancing and skips; member absences as a household feature
status: accepted
date: 2026-10-25
source_questions: [Q-037]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Domain.md]
---

# D-039: Rotation: configurable advancing and skips; member absences as a household feature

## Context
The user picked option C.

## Decision
- Per rotation definition:
  - `advance_on: any_completion | assignee_completion`, default `any_completion` (whoever does it, the turn moves on);
  - `skip_advances: true | false`, default `true`.
- **Member absences:** a member can be marked away for a date range (`member_away`). Every strategy honours it: `fixed` keeps its assignee and flags the occurrence to adults, `rotation` passes the turn to the next present member, and `pool` leaves them out. Smart assignment (M6) will use it too.

## Consequences
- The cursor moves in the same transaction as the completion or skip.
- Refines D-013.

## Alternatives considered
A plain rotation with no absences (A), or strictly assignee-based fairness (B). Not chosen as fixed behaviour; both are reachable through the settings.
