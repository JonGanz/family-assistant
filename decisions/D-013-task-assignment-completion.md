---
id: D-013
title: Task assignment strategies and per-list completion policy
status: accepted
date: 2026-10-23
source_questions: [Q-012]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/Tasks.md (future)]
---

> **Refined by** D-039 (rotation), D-040 (pool claiming), D-045 (permissions).

# D-013: Task assignment strategies and per-list completion policy

## Context
The user wants fixed, rotation, and pool strategies first, with smart assignment later. There are no points or rewards. Some lists can be checked off by kids, and others need an adult. For now, anyone can clear items from the dashboard.

## Decision
- **Phase 1 strategies:**
  - `fixed`: always the same member.
  - `rotation`: an ordered subset of members. The task moves on to the next person after each completion.
  - `pool`: anyone in a subset can claim it.
- **Phase 2:** `smart`, which balances load using the calendar and tasks. It sits behind the same `AssignmentStrategy` trait.
- `TaskList.completion_policy` is one of:
  - `assignee`: the assigned member, or any adult, can complete it.
  - `adult_only`: only an adult, from their own logged-in view.
- **Temporary override:** `household.dashboard_completion_override = true` is the default for now. It lets anyone complete any item from the dashboard device, using the member picker with no PIN (D-019). Turning it off enforces the policy on the dashboard as well.
- There is no gamification.
- Homework and its steps come from manual entry or are extracted from email.

## Consequences
- The audit log records who completed each task and from which device, which keeps the override accountable.

## Alternatives considered
Points or rewards. Deferred.
