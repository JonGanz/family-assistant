---
id: D-109
title: "Recipes: three import paths, stored images, a library with per-member ratings"
status: accepted
date: 2026-11-08
source_questions: [Q-098, Q-099, Q-113]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/AI.md, arch/Frontend.md]
---

# D-109: Recipes: three import paths, stored images, a library with per-member ratings

## Context
M4 built the recipe-extraction contract (AI.md §9, D-094) and stored nothing. Batch 24 asked how recipes get in (Q-098), how images are kept (Q-099) and what the library offers (Q-113). Answers: A, A, B.

## Decision
- **Import (Q-098 A):** a URL (schema.org JSON-LD, then microdata, then the LLM), pasted page text through the LLM, and a manual form. Every path opens an editable draft; nothing is stored until an adult saves it.
- **Images (Q-099 A):** on import the image is downloaded through the guarded HTTP client, resized and served by the app; adults can upload a photo for any recipe. The kiosk and phones never request third-party image hosts.
- **Library (Q-113 B):** search by title and ingredient, tag filter chips (D-105), favourites, "last planned" from the plan, a household note per recipe, and **per-member ratings** (1 to 5, each signed-in member rates for themself; a base for later suggestions, which stay out of scope per D-021).

## Consequences
- `fa-recipe` gains a microdata parser and a text input; `RecipeOrigin` gains `microdata`. `POST /recipes/import` replaces `POST /ai/recipes/extract` (kept one milestone as an alias).
- New tables `recipe_rating`, image storage on disk under the data volume (backed up with it).

## Alternatives considered
Photo/OCR import (Q-098 B), hotlinked images (Q-099 B), title search only (Q-113 C).
