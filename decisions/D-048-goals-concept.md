---
id: D-048
title: Goals with progress are a separate concept from tasks (scope pending)
status: accepted
date: 2026-10-25
source_questions: [Q-036]
supersedes: 
superseded_by:
affects: [arch/Overall.md, arch/Tasks.md]
---

> **Refined by** [D-050](D-050-roadmap-goals.md) (goals are M7) and [D-051](D-051-goals-model.md) (goal model).

# D-048: Goals with progress are a separate concept from tasks (scope pending)

## Context
While answering Q-036 the user suggested that progressive work, such as "read 10 minutes a night" that grows when missed, is better modelled as a **goal with progress** than as a task. They added that goals could track how far children are with homework and how adults are doing on their own targets and objectives.

## Decision
- Adopted: goals are a separate concept from tasks. A task is a unit of work that is done or not. A goal has a target and measured progress over time.
- Tasks will not grow or change their content when missed.
- How goals are modelled and when they are built are open (Q-047, Q-048). They are **not** part of M2 unless Q-047 says so.

## Consequences
- Keeps the task model simple. A goal can later count task completions as progress ("read on 20 school nights this month").
- D-013 rules out gamification (points, rewards). Goal progress is tracking, not a reward system, so it does not conflict. Q-048 asks about streaks explicitly.

## Alternatives considered
Escalating tasks. Rejected by the user in the same answer.
