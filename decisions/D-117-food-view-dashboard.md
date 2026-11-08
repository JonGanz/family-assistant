---
id: D-117
title: Food view with Meal Plan and Shopping List; today's meals on the home screens
status: accepted
date: 2026-11-08
source_questions: [Q-110]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-117: Food view with Meal Plan and Shopping List; today's meals on the home screens

## Context
Q-110 asked what the dashboard shows. Answer: the day's meals on the Home screen; a Food view with tabs Shopping List and Meal Plan; the shopping list grouped by store, where items are added or removed; the Meal Plan a month-style grid starting with the previous week, today highlighted with the same light/dark inversion, continuing until the screen is full. D-015 requires adding grocery items from the dashboard without a PIN.

## Decision
Read permissively, for both the app and the kiosk:
- **Today's meals** appear on the kiosk Home view (a `meals` panel in the panel catalogue and the Family hub preset) and on the app's Today page.
- A **Food** destination in the app navigation (a Food group) and a **Food** view in the kiosk bar (Home, Week, Food). Tabs: **Meal Plan**, **Shopping List**, and in the app also **Recipes** and **Templates** (adults).
- **Shopping List:** grouped by store then aisle (D-115), an add box, check-off; removal needs an adult (D-110), so on the kiosk it needs the PIN.
- **Meal Plan:** a grid of week rows (7 days, the household's `week_start`), starting with the previous week; today's cell drawn inverted like the calendar's today; rows continue until the screen is full on the kiosk (no scrolling) and scroll on to the horizon in the app. Cells show the visible slots.
- The kiosk Food view is a device setting, default on.

## Consequences
Frontend.md: new routes `/food/*`, the kiosk bar gains Food, the `meals` panel. Dashboard payload gains today's meals.

## Alternatives considered
Only an add button on the dashboard (Q-110 C); cook mode (Q-110 B) not asked for.
