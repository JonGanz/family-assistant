---
id: D-124
title: Recipes have named sections, each with its own ingredients and steps
status: accepted
date: 2026-11-17
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-124: Recipes have named sections, each with its own ingredients and steps

## Context
From a user instruction on 2026-11-17 (no question; the user accepted the proposed defaults). A recipe often has parts that belong together but are not separate recipes (a chicken and its own sauce). Separate recipes already cover dishes that only share a meal slot. schema.org JSON-LD has HowToSection for steps but no grouping for ingredients; the importer flattened the sections and kept heading lines such as 'For the sauce:' as ingredients.

## Decision
A recipe has zero or more ordered, named sections. Each ingredient line and each step belongs to one section or to none. Ingredients without a section are shared and show in a full-width Ingredients card above the section columns. Steps without a section are the finish steps and show in a full-width Finish card below the columns. Each section is a column with its ingredients card above its steps card. The number of columns before wrapping is configurable in Settings > Meals, default 2; phones always stack. A recipe without sections looks as before. Import keeps JSON-LD HowToSection names, turns heading-like ingredient lines into sections, reads ingredient groups from common recipe-plugin markup when JSON-LD has none, pairs ingredient groups with step sections by name, and the LLM fallback may return sections. When pairing is wrong the editor fixes it: move any line or step to another section, rename, reorder, merge or delete sections; there is no separate pairing screen. Existing recipes stay flat; re-importing from the source page is a later option. Sections do not change the shopping list or ingredient matching.

## Consequences
New recipe_section table and a nullable section_id on recipe_ingredient and recipe_step; RecipeDraft, RecipeInput and RecipeView carry sections; steps in RecipeView become objects with an id, so the kiosk ticks are keyed by row id instead of position. New meals_settings column recipe_section_columns.

## Alternatives considered
Sub-recipes (a recipe linking another recipe as a component): reusable across recipes, but needs scaling and shopping-list recursion; can be added later as a section that points at a recipe. A free-text heading per line: no structure for the two-column layout.
