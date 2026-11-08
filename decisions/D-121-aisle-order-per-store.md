---
id: D-121
title: Each store has its own aisle order and hidden aisles
status: accepted
date: 2026-11-08
source_questions: [Q-116]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md, arch/Domain.md]
---

# D-121: Each store has its own aisle order and hidden aisles

## Context
D-115 groups the list by store, then aisle. Q-116 asked whether aisle order is shared or per store. Answer: B, each store has its own aisle order and may hide aisles it does not have.

## Decision
- The household **aisle list** (names, plus a default order) stays. It is the order for items with no store ("Any store") and the starting point for a new store.
- Each **store** has its own aisle order and can **hide** aisles it does not have. A new store copies the household order, with nothing hidden.
- On the list, a store's items are grouped in that store's order. An item whose aisle the store hides goes under that store's "Other".

## Consequences
- New table `grocery_store_aisle (store_id, aisle_id, ord, hidden)`, written by `PUT /meals/stores/{id}/aisles` and copied from the household order when a store is created. Missing rows mean the household order and not hidden, so a new aisle shows in every store.
- Settings → Meals gets a per-store aisle editor.

## Alternatives considered
One shared order (Q-116 A); a shared default plus an optional override (Q-116 C). B's rows default to the shared order anyway, so a store that is never edited behaves like C.
