---
id: D-041
title: Routines: per-member timed checklists with day filters and a dashboard progress panel
status: accepted
date: 2026-10-25
source_questions: [Q-039]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Domain.md, arch/Frontend.md]
---

# D-041: Routines: per-member timed checklists with day filters and a dashboard progress panel

## Context
The user picked option C and asked specifically to keep the **progress bar**.

## Decision
- A routine is its own kind: a per-member, ordered checklist with a time window (`starts_at`, `done_by`), optional per-step target times, and a day filter.
- Day filter: configurable, default weekdays. Other choices are every day, weekends, or *school days* taken from a chosen calendar (for example a school ICS feed, where days with a matching all-day "no school" event are excluded).
- A dashboard `routine` panel appears automatically during a routine's window on devices that enable it. It is big and touch-friendly, one column per member, with a **progress bar** for each.
- Steps are checked off without a PIN (D-019).

## Consequences
- Each routine produces one occurrence per member per qualifying day. Unfinished routines are auto-skipped at the end of the day, not rolled over.
- Routine progress is visible to adults on their phones.

## Alternatives considered
A daily task with steps (A). Not chosen.
