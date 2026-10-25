---
id: D-037
title: Recurrence is per definition: schedule (RRULE) or after-done interval, default after-done
status: accepted
date: 2026-10-25
source_questions: [Q-035]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Domain.md]
---

# D-037: Recurrence is per definition: schedule (RRULE) or after-done interval, default after-done

## Context
The user picked option C.

## Decision
- Each `TaskDefinition` has `schedule`, which is one of:
  - `rrule`: calendar-anchored, such as "every Saturday" or "1st of the month". A missed occurrence does not move the next one.
  - `after_done`: an interval counted from the last completion, such as "7 days after it was last done".
- New definitions start as `after_done`. The UI offers both.

## Consequences
- `next_due_at` is computed from the RRULE or from `last_done_at + interval`.
- Replaces the plain `recurrence (RRULE)` attribute in Domain.md.

## Alternatives considered
Only completion-anchored (A) or only schedule-anchored (B). Not chosen.
