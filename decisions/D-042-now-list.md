---
id: D-042
title: The Now list: deterministic score with configurable weights and size
status: accepted
date: 2026-10-25
source_questions: [Q-040]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Frontend.md]
---

# D-042: The Now list: deterministic score with configurable weights and size

## Context
The user picked option C.

## Decision
- The Now list shows the top N open tasks by a deterministic score: due proximity (overdue ranks highest), the definition's lead time, a manual `priority_flag` (`normal | high`), and an origin boost for weather tasks.
- The weights and N (default 7) are household settings, editable in the UI (D-029 style).
- Each dashboard device can filter the list by member or list.
- Phones show the current user's Now list first, then the household's.

## Consequences
- In M4 the `task.urgency` pipeline (D-027) adds a term to the same score.

## Alternatives considered
Due today or overdue only (A), or a fixed top N (B). Not chosen.
