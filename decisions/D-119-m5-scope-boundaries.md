---
id: D-119
title: M5 is food only, with no meal notifications; M4 follow-ups go to M4.2
status: accepted
date: 2026-11-08
source_questions: [Q-112, Q-114]
supersedes: 
superseded_by:
affects: [arch/Overall.md, arch/Meals.md]
---

# D-119: M5 is food only, with no meal notifications; M4 follow-ups go to M4.2

## Context
Q-112 asked about meal reminders (answer A, none); Q-114 asked whether M5 also takes the M4 gaps G4-01 to G4-15 and the code review 2 LARGER items (answer A, food only).

## Decision
- M5 sends no meal notifications.
- The M4 gaps (G4-01 to G4-15, including D-098 to D-102 and the code review 2 LARGER items) form a separate follow-up milestone, **M4.2**, scheduled after M5.

## Consequences
README and Overall.md roadmaps gain M4.2; IMPROVEMENTS.md points G4 to it.

## Alternatives considered
Reminders (Q-112 B to D); folding the follow-ups into M5 (Q-114 B, C).
