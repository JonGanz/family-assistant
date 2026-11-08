---
id: D-110
title: Meals permissions; no cook per meal
status: accepted
date: 2026-11-08
source_questions: [Q-100, Q-101]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Domain.md]
---

# D-110: Meals permissions; no cook per meal

## Context
The RBAC matrix only covered adding grocery items (D-015, D-019). Q-100 asked for the rest; Q-101 asked whether an entry names a cook. Answers: A and C.

## Decision
- Adults manage recipes, templates, the meal plan, the ingredient catalogue and Settings → Meals.
- Children and the dashboard read recipes and the plan.
- Everyone, the dashboard included without a PIN, adds grocery items and checks them off; adults delete or edit items and mark staples. A dashboard elevated by an adult PIN acts as that adult.
- Every signed-in member rates recipes for themself (D-109).
- A meal-plan entry has **no cook** field and creates no task.

## Consequences
Domain.md RBAC matrix gains the meals rows; `fa-domain/src/authz.rs` gains the actions.

## Alternatives considered
Children proposing meals (Q-100 B), everyone editing (Q-100 C), a cook with or without a task (Q-101 A, B).
