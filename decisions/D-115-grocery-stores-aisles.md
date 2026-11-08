---
id: D-115
title: Grocery list grouped by store, then aisle
status: accepted
date: 2026-11-08
source_questions: [Q-108]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-115: Grocery list grouped by store, then aisle

## Context
Q-108 answer: option A (grouped by aisle, editable aisles, manual items matched to the catalogue), plus the idea that items often come from different stores (milk at the grocery store, eggs in bulk elsewhere), so grouping by store would be nice too.

## Decision
Adopted: stores fit the one living list (D-021) as a grouping, not as separate lists.
- An editable **store** list per household (name, order); none by default, so a household that never adds one sees aisles only.
- An ingredient may have a **default store**; a grocery item takes it when added and can be moved to another store or to "Any store".
- The list groups by store (in store order, "Any store" last), then by aisle in the household's aisle order; manual items are matched to the catalogue when possible, else "Other". The Staples check section stays separate (D-021).
- A store filter (D-105 chips) shows one store's items while shopping.

## Consequences
Tables `store`, `aisle`; `ingredient.default_store`, `grocery_item.store`. Per-store aisle order is asked in batch 25 (Q-116).

## Alternatives considered
A flat list (Q-108 B); separate lists per store (conflicts with D-021).
