---
id: D-051
title: Goals: count or quantity measures, weekly cadence with reset, set by adults, progress bars
status: accepted
date: 2026-10-25
source_questions: [Q-048]
supersedes: 
superseded_by:
affects: [arch/Domain.md, arch/Overall.md, arch/Goals.md (future)]
---

# D-051: Goals: count or quantity measures, weekly cadence with reset, set by adults, progress bars

## Context
The user picked option D (their own variant).

## Decision
- **Measures:** a count ("nights read") or a quantity with a unit ("minutes", "km"). No milestone checklists or manual percentages for now.
- **Cadence:** a goal is either a one-off total over a date range, or **recurring**: a target per period that resets each period on a set day or date ("150 minutes a week, resets Monday"). Weekly is the default period.
- **Who:** a goal can belong to any member. **Only adults create and edit goals** for now.
- **Display:** progress bars on phones and in an optional dashboard `goals` panel.
- **Not now:** streaks, points and rewards. They are deferred, not ruled out.

## Consequences
- Not decided yet; settle in `Goals.md` questions before M7: progress sources (manual entry, counting completed tasks), privacy of adults' goals, and period lengths other than weekly.
- D-013's "no gamification" stays in force until a later decision changes it.

## Alternatives considered
Minimal (A), full with streaks (B), or the suggested configurable default (C). Not chosen as such.
