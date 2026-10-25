---
id: D-049
title: Missed schedule-anchored tasks auto-skip by default
status: accepted
date: 2026-10-25
source_questions: [Q-046]
supersedes: 
superseded_by:
affects: [arch/Tasks.md]
---

# D-049: Missed schedule-anchored tasks auto-skip by default

## Context
The user picked option C. This settles the part of Q-036 that D-038 left open.

## Decision
- New `rrule` (schedule-anchored) definitions default to `on_missed = auto_skip`: when the next scheduled occurrence comes due, the missed one is marked `skipped` (audited, by the system) and replaced.
- `after_done` definitions keep `roll_over` (D-038).
- Both stay editable per definition. The edit form explains the choice.

## Consequences
- The default depends on `schedule_kind` and is set when the definition is created. Changing the schedule kind later doesn't change an `on_missed` value an adult has set.

## Alternatives considered
Roll-over for every definition (A). Not chosen.
