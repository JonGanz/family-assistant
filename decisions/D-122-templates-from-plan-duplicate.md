---
id: D-122
title: Templates from the editor, from a plan range, or by duplicating
status: accepted
date: 2026-11-08
source_questions: [Q-117]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-122: Templates from the editor, from a plan range, or by duplicating

## Context
D-112 adds meal templates made in an editor. Q-117 asked whether templates can also be made from the plan. Answer: C, the editor, "Save as template" from a plan range, and duplicating a template.

## Decision
Templates are made three ways:
- In the **template editor**.
- **Save as template** from the meal plan: pick a date range and the slots to include. The new template's length is the range in days, and every entry in those slots is copied as a cell (recipe or free text, servings, order) on its day. Like an apply, the copy is detached: later plan edits don't change the template.
- **Duplicate** an existing template, under a new name, to change it without losing the original.

## Consequences
- `POST /meal-templates/from-plan {name, from, to, slot_ids}` (the range is at most 366 days).
- `POST /meal-templates/{id}/duplicate {name}`.
- UI: a "Save as template" action in the Meal plan (range and slot picker), and "Duplicate" on a template.

## Alternatives considered
Editor only (Q-117 B); save from the plan without duplicating (Q-117 A).
