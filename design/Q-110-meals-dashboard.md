---
id: Q-110
title: Meals and grocery on the dashboard
status: open
batch: 24
area: frontend
created: 2026-11-07
resolved:
decisions: []
---

# Q-110: Meals and grocery on the dashboard

## Context
D-015 requires adding grocery items from the dashboard without a PIN. The dashboard is at-a-glance (D-068) with configurable panels and a Week view (Frontend.md §4).

## Question
What appears on the dashboard?

## Options
- A. A `meals` panel (today's and the next meals with the cook) and a `grocery` panel (the list, an add box, check-off); both in the panel catalogue, `meals` in the Family hub preset; a meals row in the Week view (suggested)
- B. A, plus a cook mode: open a planned recipe full screen with large steps and the screen kept awake
- C. Only an add-to-grocery button
- D. Configurable through the layout (as now); the Week view meals row a device setting, default on

## Answer
<!-- answer below this line -->
The day's meals should show up on the Home screen. A "Food" view should offer a few tabs: Shopping List, and Meal Plan.

Shopping List will show the lists (perhaps grouped by store) where items can be added or if needed, removed.

Meal Plan will show a month view grid of the meal plan starting with the previous week, the current date highlighted (probably using the same inversion of light/dark), and continuing on until the screen is full.
