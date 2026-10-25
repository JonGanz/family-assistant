---
id: D-038
title: Missed occurrences: configurable on_missed, roll-over by default for after-done tasks
status: accepted
date: 2026-10-25
source_questions: [Q-036]
supersedes: 
superseded_by:
affects: [arch/Tasks.md]
---

# D-038: Missed occurrences: configurable on_missed, roll-over by default for after-done tasks

## Context
The user picked option A (roll over) for completion-anchored tasks. They did not say what schedule-anchored tasks should do by default. They also considered "progressive" tasks (read 10 minutes, then 30 if missed) and decided those are **goals with progress**, not tasks (see D-048).

## Decision
- Each definition has `on_missed: roll_over | auto_skip | pile_up`.
- `after_done` definitions default to `roll_over`: the one open occurrence stays open, shows as overdue, and ranks higher over time. Nothing new is created until it is done or skipped.
- The default for `rrule` definitions is open (Q-046). Until it is answered, it is `roll_over` too.
- Tasks do not change their own content when missed. Escalating targets belong to goals.

## Consequences
- `auto_skip` marks the old occurrence `skipped` (audited, `skipped_by = system`) when the next one is created.

## Alternatives considered
Pile-up or auto-skip as the only behaviour. Not chosen.
