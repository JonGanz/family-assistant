---
id: D-027
title: System One decision points: category, time-sensitivity, member linking, calendar routing, task urgency, bill detection
status: accepted
date: 2026-10-23
source_questions: [Q-026]
supersedes:
superseded_by:
affects: [arch/Overall.md]
research: [R-001]
---

# D-027: System One decision points: category, time-sensitivity, member linking, calendar routing, task urgency, bill detection

## Context
The user picked option A (uses 1 to 4).

## Decision
System One questions are used at these **decision points**, with deterministic rules always first:
| Decision point | Type | Options |
|---|---|---|
| `email.category` | Choice | household categories |
| `email.time_sensitive` | Noul | n/a |
| `email.member_link` | Choice | members (+ 'household', 'none') |
| `event.calendar_route` | Choice | writable calendars (D-008) |
| `task.urgency` | Score 1–5 | n/a |
| `email.is_bill` | Noul | n/a |
Weather advisories and every kind of extraction stay with System 2.

## Consequences
- `email.member_link` is single-label (R-001). An email about several children needs a Noul question per child when the Choice probabilities are split. This is a configurable fallback.

## Alternatives considered
Email category and time-sensitivity only. Not chosen.
