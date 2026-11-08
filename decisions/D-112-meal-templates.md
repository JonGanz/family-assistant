---
id: D-112
title: Meal templates applied from a start date; no repeating entries
status: accepted
date: 2026-11-08
source_questions: [Q-103]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-112: Meal templates applied from a start date; no repeating entries

## Context
Q-103 asked what an entry can be and how repeats are made. The user answered with an idea: meal templates, for example a five-week "Fall 2026" dinner template applied from the first Monday in September, re-applied by hand later. No repeating entries.

## Decision
- A **meal template** has a name and a length in days (any number). For each day and slot it holds a recipe, free text, or nothing.
- **Apply** a template from a start date: day 1 of the template lands on that date, and each filled template cell fills the matching plan slot. Templates can fill different slots (a dinner template and a lunch template applied independently); empty cells and slots the template does not use leave the plan untouched.
- **Conflicts:** before applying, the app lists every slot that already has an entry and would be replaced; the user keeps or replaces each one (keep is preselected for free-text entries, so "Thanksgiving with cousins" is not clobbered by default). Then the apply is one change with Undo (D-096).
- **Detached:** applied entries are plain plan entries with no link back to the template. Moving, swapping, deleting or editing them never changes the template, and editing a template never changes the plan.
- Plan entries are a recipe or free text; they can be moved and swapped between slots and dates.
- **No repeating entries** for now; re-applying a template is how a cycle repeats.

## Consequences
New tables `meal_template`, `meal_template_cell`. `POST /meal-templates/{id}/preview` and `/apply`. Whether a slot holds one entry or several, and saving a plan range as a template, are asked in batch 25 (Q-115, Q-117); the design assumes one entry per slot until then.

## Alternatives considered
Copy-a-week (Q-103 A) and RRULE repeats (Q-103 B); templates cover both needs more simply.
