---
id: D-125
title: The dashboard recipe view scales the recipe live
status: accepted
date: 2026-11-17
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-125: The dashboard recipe view scales the recipe live

## Context
From a user instruction on 2026-11-17: a servings option on the dashboard recipe page to scale a recipe live to 1.5x or 2x.

## Decision
The kiosk recipe view shows a row of scale factors; the list is configurable in Settings > Meals, default 0.5, 1, 1.5, 2, 3 (1 is always offered). The choice is not saved and resets to 1x every time a recipe opens. Ingredient lines with a parsed quantity show the scaled amount (common fractions such as 1/4, 1/3, 1/2, 2/3, 3/4 as glyphs, else up to two decimals); lines without a quantity and the step texts stay as written; servings show scaled. Nothing is written to the recipe or the meal plan.

## Consequences
New meals_settings column recipe_scale_factors. RecipeView carries the factors and the column count so the kiosk needs no adult-only settings route.

## Alternatives considered
A servings stepper (scale to N servings): needs servings on every recipe, which imports often lack. Saving the factor per recipe or per plan entry: not asked for.
