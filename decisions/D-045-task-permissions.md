---
id: D-045
title: Task permissions: adults manage lists and definitions; per-list flags for children and the dashboard
status: accepted
date: 2026-10-25
source_questions: [Q-043]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Domain.md]
---

# D-045: Task permissions: adults manage lists and definitions; per-list flags for children and the dashboard

## Context
The user picked option C.

## Decision
- Adults create and edit lists, recurring definitions, and routines.
- Each list has `child_can_add: own | none`. With `own`, a child can add one-off tasks for themself to that list and edit their own one-off tasks.
- Each list has `dashboard_add: true | false`. With `true`, anyone can add one-off tasks to it from a dashboard without a PIN.
- Defaults: a "General" list has `dashboard_add = true`, and other lists have `false`. `child_can_add` defaults to `own` for homework and `none` otherwise.

## Consequences
- New rows in the RBAC matrix (Domain.md).

## Alternatives considered
Adults only (A), or one fixed rule for all lists (B). Not chosen.
