---
id: D-105
title: Every filter row is an additive multi-select with All first
status: accepted
date: 2026-11-07
source_questions: [Q-090, Q-091]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-105: Every filter row is an additive multi-select with All first

## Context
Q-090 and Q-091: the user found the filter rows inconsistent. Bills had single-choice state chips, and the Calendar and Tasks member filters behaved differently.

## Decision
Every filter row in the app is an additive multi-select chip row of one shape:
- The first chip is 'All'. Choosing All clears every other chip.
- The other chips are the groups, members or states. Choosing any of them clears All, and each toggles on and off independently, so for example Bills can show Proposed and Open together.
- Clearing the last chosen chip returns to All.
- The row is one shared `ui/` component used by Bills (state), Calendar (members), Tasks (members) and Mail (category, member, source, state). The Inbox 'AI only' chip stays a separate toggle.
- A remembered filter stores the chosen set (Tasks keeps `fa.tasks.filter`, now a list, default the viewer's own member chip, that is 'Mine').
Bills keeps its grouping by state (Q-090 A), and Tasks keeps the tabs Now, Lists and Suggested (Q-091 A).

## Consequences
A web change (T-20): a new `UiFilterChips` component, and the stores and views moved to sets. Where an API filter takes a single value, T-20 filters on the client or reports the API change needed.

## Alternatives considered
Single-choice chips per row (the M4 state), or a per-row mix of both.
