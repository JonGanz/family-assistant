---
id: D-043
title: Season windows and weather-triggered tasks: create by default, suggest configurable
status: accepted
date: 2026-10-25
source_questions: [Q-041]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Feeds.md]
---

# D-043: Season windows and weather-triggered tasks: create by default, suggest configurable

## Context
The user picked option C.

## Decision
- A weather rule with action `task` has `task_mode: create | suggest`, default `create`. `suggest` puts the task in a Suggested section for an adult to accept.
- A definition can have a **season window**: a month/day range (it may wrap the year end), plus an optional weather condition in the existing rule DSL (such as "not if rain > 5 mm today"). Outside the window, no occurrences are created.

## Consequences
- Weather-created tasks are deduplicated per rule and day.

## Alternatives considered
Always create (A) or always suggest (B). Not chosen.
