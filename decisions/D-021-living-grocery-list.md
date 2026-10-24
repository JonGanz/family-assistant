---
id: D-021
title: One living grocery list with a pantry-staples check section
status: accepted
date: 2026-10-23
source_questions: [Q-021]
supersedes: 
superseded_by:
affects: [arch/Overall.md]
---

# D-021: One living grocery list with a pantry-staples check section

## Context
The user wants one living list. Pantry staples should appear as a separate section when a list is created for upcoming meals, so they can be scanned quickly. The dietary-preferences part of Q-021 was not answered.

## Decision
- There is one `GroceryList` per household, and it never closes. Generated and manual items merge into it. Checked items are archived after a configurable time (default 24 h), and their history is kept.
- The `Ingredient` catalogue has an `is_staple` flag that the household sets. When a list is generated from meals, staple ingredients **do not** go onto the list. They appear in a separate **"Staples check"** section showing which meals need them. Tapping 'low' moves a staple onto the list.
- **Dietary preferences are deferred.** Meal suggestions aren't in scope, so nothing uses them yet. We'll revisit if that changes. I'm not asking about it in a new batch.

## Consequences
- Matching ingredients to staples relies on the normaliser from D-015, so a staple must match after normalisation (for example, 'kosher salt' and 'salt').

## Alternatives considered
A new list per trip. Declined.
