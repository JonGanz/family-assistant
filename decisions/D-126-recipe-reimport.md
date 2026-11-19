---
id: D-126
title: Re-import a recipe from its source URL
status: accepted
date: 2026-11-19
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-126: Re-import a recipe from its source URL

## Context
User instruction of 2026-11-19: recipes imported before sections (D-124) stay flat and there was no way to import them again. The user asked for a one-recipe-at-a-time re-import, no bulk action, keeping the recipe's meal plan entries and ratings.

## Decision
A recipe with a source URL offers 'Re-import from source' to adults (recipe page and editor). It runs the existing import (POST /recipes/import) and opens the editor with the imported ingredients, steps, sections, yield, servings and times in place of the current ones; the title, tags, note and photo stay. Nothing is saved until the user presses Save, which updates the same recipe (PUT /recipes/{id}), so its id and with it plan entries, template entries and ratings are kept; ingredient matches are kept for lines whose text is unchanged. A confirmation first says that edits to ingredients and steps will be replaced.

## Consequences
No backend change. A failed import leaves the recipe untouched and shows the error. A test proves plan entries and ratings survive an update that adds sections.

## Alternatives considered
Re-import all URL recipes at once (rejected by the user); a server-side re-import that saves directly (no chance to review).
