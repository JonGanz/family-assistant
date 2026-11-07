---
title: Meals — recipes, meal plan, living grocery list
scope: recipe storage and import, the ingredient catalogue and normaliser, the meal plan, grocery generation and the living grocery list with the staples check, meals on the dashboard, API, authorization
status: draft
updated: 2026-11-07
drawio: Meals.drawio
drawio_src: src/Meals.py
depends_on: [D-006, D-015, D-019, D-021, D-035, D-068, D-072, D-093, D-094, D-095, D-096, D-105, D-108]
milestone: M5
research: [R-007]
---

# Meals: recipes, meal plan, living grocery list (DRAFT v0)

This doc covers M5 (D-023, D-108): storing recipes (imported through the M4 extraction contract, AI.md §9, or typed in), planning meals over a configurable horizon, and one living grocery list per household that meals feed into, with a staples check (D-015, D-021).

**v0 is a pre-answer draft.** Settled points cite their decisions. Points that wait on batch 24 (Q-098 to Q-114) are marked **(open: Q-NNN)** and describe the *suggested* option as a working assumption only; they are re-derived when the answers come in (CLAUDE.md workflow 3).

**Not in M5:** meal suggestions and dietary preferences (D-021), export to other apps (D-015), pantry inventory beyond the staples flag (D-021).

## 1. Concepts
| Concept | What it is | Decided by |
|---|---|---|
| **Recipe** | A stored recipe: title, ingredients (each keeps its source line), steps, servings, times, tags, image, how it was imported | D-094 (contract), Q-098, Q-099, Q-113 |
| **Ingredient** | A catalogue entry: canonical name, aliases, staple flag, aisle, default unit. Recipe and grocery lines link to it after normalisation | D-015, D-021, Q-106 |
| **Normaliser** | Deterministic: parse line → singularise → alias/trigram match → convert units within a dimension. The LLM only for lines rules cannot match | D-015, CLAUDE.md, Q-105, Q-107 |
| **Meal slot** | A named part of the day (breakfast, lunch, dinner, snack), configurable | D-015, Q-102 |
| **Meal plan entry** | A date + slot holding a recipe or free text, with servings (and maybe a cook) | D-015, Q-101, Q-103, Q-104 |
| **Grocery list** | Exactly one per household, never closes; checked items archive after a configurable time (default 24 h) | D-021 |
| **Grocery item** | A line on the list: text, optional ingredient, quantity and unit, the meals it came from, manual or generated | D-015, D-021 |
| **Staples check** | When generating, staple ingredients do not go on the list; they appear in a separate section with the meals that need them; "low" moves one onto the list | D-021 |

## 2. Crates (page "Components")
| Crate | Role |
|---|---|
| `fa-domain` | Gains `RecipeDraft`, `RecipeIngredient`, `RecipeOrigin` (moved from `fa-recipe`, AI.md §9) and the M5 domain types; authz actions for meals |
| `fa-store` | Migrations and repos: `recipe*`, `ingredient`, `meal_slot`, `meal_plan_entry`, `grocery_list`, `grocery_item*`, `meals_settings` |
| `fa-recipe` | Extraction (exists). M5 adds schema.org **microdata** (open: Q-098) and pasted-text input; it still stores nothing |
| `fa-meals` (new) | Normaliser, recipe service (import → draft → save, image fetch), meal plan service, grocery generation and plan-change reconciliation, archive job, ingredient LLM job |
| `fa-api` | `/recipes`, `/ingredients`, `/meal-plan`, `/grocery` routes; dashboard payload gains meals and grocery |
| `fa-jobs` (handlers in `fa-meals`) | `grocery.archive` (cron), `recipe.image` (fetch and resize), `ingredient.llm` (open: Q-107) |
| `apps/web` | Recipes, Meal plan, Grocery views; Settings → Meals; dashboard panels (open: Q-110) |

Coupling: `fa-meals` depends on `fa-domain`, `fa-store`, `fa-recipe`, `fa-llm` and `fa-connectors` (for `Http`, private hosts blocked). Nothing depends on `fa-meals` except `fa-api` and `fa-server`.

## 3. Data model (page "Model")
All tables carry `household_id` with RLS like the rest of the schema (Domain.md conventions). Entities edited from several clients carry `version` (optimistic concurrency).

| Table | Columns (beyond id, household_id, timestamps) | Notes |
|---|---|---|
| `recipe` | title, source_url?, servings?, yield_text?, prep_minutes?, cook_minutes?, total_minutes?, import_method `jsonld\|microdata\|llm\|manual`, image_path?, tags text[], note?, favourite, created_by, version | `import_method` per Domain.md; image stored as a resized copy (open: Q-099) |
| `recipe_ingredient` | recipe, ord, raw_text, qty?, unit?, item?, ingredient?, note?, match_state `matched\|unmatched\|proposed` | `raw_text` is always the source line |
| `recipe_step` | recipe, ord, text | |
| `ingredient` | canonical_name (unique per household, singular, lower case), aliases text[], is_staple (default false), aisle?, default_unit?, density_g_per_ml? (only if Q-105 B) | Seeded per household (open: Q-106) |
| `aisle` | name, ord | Editable order (open: Q-108) |
| `meal_slot` | name, ord, time `"HH:MM"` local, visible | Defaults Breakfast 07:30, Lunch 12:00, Dinner 18:00, Snack 15:30 hidden (open: Q-102); replaces Domain.md's fixed enum |
| `meal_plan_entry` | date `YYYY-MM-DD`, slot, recipe? xor free_text?, servings?, cook_member? (open: Q-101), created_by, version | Several per slot allowed (open: Q-103) |
| `grocery_list` | — | Exactly one per household, created with the household |
| `grocery_item` | list, ingredient?, text, qty?, unit?, section `list\|staples`, manual, added_by?, added_via_device?, checked_at?, checked_by?, archived_at?, version | Archived, not deleted, after `grocery_archive_hours` (D-021) |
| `grocery_item_source` | item, meal_plan_entry, qty?, unit? | M:N with the per-meal contribution, so a plan change can subtract exactly (open: Q-109) |
| `meals_settings` | grocery_archive_hours (default 24), unit_conversion, ingredient_llm, image_mode, … | One row per household; each toggle here is one an answer made configurable. `household.meal_plan_horizon_weeks` (exists, default 2) stays on `household` |

**Invariants**
- A `meal_plan_entry` has exactly one of `recipe` and `free_text`.
- A `grocery_item` with `section = staples` is generated, never manual; "low" moves it to `list`.
- Generated quantity on an item = sum of its `grocery_item_source` rows (after conversion) plus any manual edit delta.
- An item's `ingredient`, when set, belongs to the same household.

## 4. Behaviour
### 4.1 Recipe import (open: Q-098)
`POST /recipes/import {url}` or `{text}` → `RecipeDraft` through `fa-recipe` (JSON-LD, then microdata, then the LLM; AI.md §9). Nothing is stored until the adult saves the draft from the editor. On save every ingredient line runs through the normaliser (§4.2). A manual recipe is the same editor with an empty draft. The image is fetched and resized by the `recipe.image` job through the guarded `Http` (open: Q-099).

### 4.2 Normaliser (deterministic first; crate picks in R-007)
1. Parse the line (the M4 rule parser in `fa-recipe/src/ingredient.rs`, extended with fl oz, pint, quart, gallon and the `1 (14 oz) can` form) into qty, unit, item, note.
2. Clean the item: lower case, ASCII folding, preparation words and parentheticals into `note` ("diced", "(15-oz)", "to taste"), an editable descriptor stop list ("kosher", "fresh", "large"), singularise with `pluralizer` plus an exceptions list (R-007).
3. Match: exact canonical name → alias → trigram similarity above a threshold (`pg_trgm`, or `strsim`, already a workspace dependency) → unmatched.
4. Unmatched lines keep their text. An adult's manual pick is stored as a new alias, so the next import matches deterministically. (open: Q-107) suggested: an `ingredient.llm` job sends the line and the 5 nearest catalogue candidates and gets `{ingredient?, new_name?, aisle?, confidence}` as structured JSON; the adult confirms; the LLM never creates catalogue rows by itself.
5. Units: canonical unit per dimension (ml, g, count); convert within a dimension only (open: Q-105; volume↔weight only through an optional `density_g_per_ml`); display in `household.units` rounded to kitchen sizes. A small hand-written unit table, not `uom` (R-007: compile-time units do not fit runtime-parsed quantities).
6. Microdata (open: Q-098) is a hand parser over `scraper` (already a workspace dependency) that fills the same `RecipeDraft`; RDFa is not parsed (R-007).

### 4.3 Meal plan
Dates from today to `meal_plan_horizon_weeks` ahead (D-015), past weeks read-only history. Week grid: days × visible slots. Entries are recipes or free text; copy a week to a later one (open: Q-103). Servings default to the recipe's (open: Q-104).

### 4.4 Grocery generation (D-015, D-021)
1. The user picks entries (a date range, or individual meals).
2. For each recipe entry, each ingredient line is scaled by entry servings / recipe servings and normalised.
3. Lines whose ingredient is a staple go to `section = staples`, listing the meals that need them; others merge into an existing unchecked, unarchived item with the same ingredient and a compatible unit, else a new item. Lines without an ingredient merge only on identical cleaned text.
4. Each contribution is a `grocery_item_source` row; generating the same entry twice adds nothing (idempotent per entry and line).
5. The result is one apply-with-Undo action (D-096).

### 4.5 Plan changes after generation (open: Q-109)
Suggested: removing an entry, moving it out of the generated range or changing its servings subtracts its sources from unchecked items, removing items that reach zero; manual and checked items are untouched; shown with Undo.

### 4.6 Living list (D-021)
Add from any view, including the dashboard without a PIN (D-015, D-019). Check off sets `checked_at`; the `grocery.archive` cron (every 15 min) archives items checked more than `grocery_archive_hours` ago. Changes publish SSE `grocery.updated`. Offline: (open: Q-111) suggested: an open page queues adds and check-offs locally and replays them by item `version`.

## 5. API (sketch; plan/M5.md fixes the final table)
| Endpoint | Purpose |
|---|---|
| `GET/POST /recipes`, `GET/PUT/DELETE /recipes/{id}` | List (search, tags, `{items, next_before}` envelope), create, read, update, delete |
| `POST /recipes/import` | `{url}` or `{text}` → `RecipeDraft` (nothing stored); replaces `POST /ai/recipes/extract` (kept one milestone as an alias, C-13 convention) |
| `GET /recipes/{id}/image` | The stored image |
| `GET/POST /ingredients`, `PUT/DELETE /ingredients/{id}` | Catalogue, staples, aisles, aliases |
| `GET/PUT /meals/settings`, `GET/PUT /meals/slots`, `GET/PUT /meals/aisles` | Settings → Meals |
| `GET /meal-plan?from=&to=` (`YYYY-MM-DD`), `POST /meal-plan`, `PUT/DELETE /meal-plan/{id}`, `POST /meal-plan/copy-week` | Plan |
| `GET /grocery`, `POST /grocery/items`, `PUT/DELETE /grocery/items/{id}`, `POST /grocery/items/{id}/check`, `…/uncheck`, `…/low` | The list |
| `POST /grocery/generate` | `{entry_ids[]}` or `{from, to}` → the applied change (for Undo) |

## 6. Authorization (adds to the RBAC matrix in Domain.md; open: Q-100)
Suggested working assumption:
| Action | child | adult | dashboard device | device + adult PIN |
|---|---|---|---|---|
| Read recipes, meal plan, grocery list | ✓ | ✓ | ✓ | ✓ |
| Add a grocery item (exists) | ✓ | ✓ | ✓ | ✓ |
| Check off / uncheck, mark a staple low | ✓ | ✓ | ✓ | ✓ |
| Delete or edit items, generate from meals | ✗ | ✓ | ✗ | ✓ |
| Manage recipes, the meal plan, the catalogue, Settings → Meals | ✗ | ✓ | ✗ | ✓ (plan only) |

Meals are not email-derived, so they may appear on the dashboard (D-068).

## 7. Frontend (summary; Frontend.md gets a §3f when this is accepted)
- Navigation gains a **Food** group (Meal plan, Recipes, Grocery) in `app/navigation.ts` (Frontend.md §1.3).
- **Meal plan:** week grid (day rows on compact, columns on expanded), add from recipe search or free text, "Make grocery list" with range or pick.
- **Recipes:** library (search, tag chips per D-105, favourites), detail, editor (draft from import, unmatched lines highlighted with proposals).
- **Grocery:** grouped by aisle (open: Q-108), "Staples check" section, add box on top, check-off with Undo.
- **Settings → Meals:** slots, aisles, catalogue and staples, toggles from `meals_settings`, the horizon.
- **Dashboard:** (open: Q-110) suggested `meals` and `grocery` panels and a meals row in the Week view.

## 8. Configuration
Every toggle lives in `meals_settings` and is edited in Settings → Meals (D-075); nothing is in TOML. `meal_plan_horizon_weeks` stays on `household`.

## 9. Testing
- Normaliser table tests (fractions, ranges, plurals, aliases, unit conversion and rounding, no volume↔weight).
- Generation: merge, staples section, idempotency, scaling, plan-change subtraction.
- Microdata parser fixtures (fictional recipes written for the repo, D-035).
- e2e: import a fixture page served by the test server, plan it, generate, check off from the dashboard.
