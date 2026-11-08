---
id: Q-109
title: When the plan changes after the list was generated
status: open
batch: 24
area: domain
created: 2026-11-07
resolved:
decisions: []
---

# Q-109: When the plan changes after the list was generated

## Context
Each generated item records the meals it came from (D-015). Later a meal is removed, moved outside the range, or its servings change.

## Question
What happens to the items it contributed?

## Options
- A. Unchecked generated quantities are reduced or the item removed; manual and checked items are never touched; the change shows with Undo (suggested)
- B. Nothing: the list is edited by hand
- C. The app asks each time
- D. Configurable in Settings → Meals, default A

## Answer
<!-- answer below this line -->
Option C.
