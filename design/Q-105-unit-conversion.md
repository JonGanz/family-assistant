---
id: Q-105
title: "Units on the grocery list: what gets converted"
status: open
batch: 24
area: domain
created: 2026-11-07
resolved:
decisions: []
---

# Q-105: Units on the grocery list: what gets converted

## Context
Merging "1 cup milk" and "250 ml milk" needs unit conversion (D-015). Volume and weight convert within themselves; volume↔weight needs a density per ingredient. The household already has `units: metric|imperial`. See R-007 for units and data.

## Question
What does the list convert?

## Options
- A. Convert within a dimension (volume, weight, count) and show the total in the household's units, rounded to kitchen sizes; never volume↔weight, so such lines stay separate on one item (suggested)
- B. A, plus volume↔weight through an editable per-ingredient density
- C. No conversion: lines with different units stay separate
- D. Configurable in Settings → Meals (off / within dimension / with densities), default A

## Answer
<!-- answer below this line -->
Option A.
