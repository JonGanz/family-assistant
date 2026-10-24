---
id: D-015
title: Configurable meal-plan horizon; grocery lists generated from selected meals; dashboard can add items
status: accepted
date: 2026-10-23
source_questions: [Q-014]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-015: Configurable meal-plan horizon; grocery lists generated from selected meals; dashboard can add items

## Context
The user wants to plan any number of weeks ahead (configurable), generate a list from any upcoming meals, see it in the app, add to it from the dashboard, and export nothing.

## Decision
- `household.meal_plan_horizon_weeks` is configurable. Meal-plan slots (breakfast, lunch, dinner, snack) are configurable too.
- **Grocery generation:** the user selects upcoming `MealPlanEntry`s, for example by a date range or by picking meals. Ingredients are normalised (unit conversion and singular/plural matching), merged, and added to a grocery list. Each item records which meals it came from.
- Manual items can be added from any view, including the dashboard (no PIN, D-019). Items are checked off while shopping.
- There is no export or integration.

## Consequences
- The grocery list model is still open (one living list or one list per trip), along with pantry and dietary needs. See Q-021.

## Alternatives considered
Exporting to AnyList or Keep. Declined.
