---
id: D-116
title: When the plan changes after a list was generated, the app asks
status: accepted
date: 2026-11-08
source_questions: [Q-109]
supersedes: 
superseded_by:
affects: [arch/Meals.md]
---

# D-116: When the plan changes after a list was generated, the app asks

## Context
Each generated item records its meals (D-015). Q-109 asked what happens when a meal is removed or its servings change afterwards. Answer: C, ask each time.

## Decision
When an entry that contributed to unchecked items is removed, replaced (including by a template apply), moved outside the dates it was generated for, or has its servings changed, the app shows what would change on the list (items reduced or removed) and asks: update the list, or leave it. Checked and manual items are never touched. With no unchecked contribution, nothing is asked.

## Consequences
`grocery_item_source` keeps each entry's contribution so the change can be computed; the plan endpoints return a `grocery_impact` the UI asks about, and `POST /grocery/reconcile` applies it.

## Alternatives considered
Automatic reduction (Q-109 A), never (Q-109 B).
