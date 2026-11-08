---
title: Meals — recipes, meal plan, templates, living grocery list
scope: recipe storage, import, images and ratings; the ingredient catalogue and normaliser; meal slots, the meal plan and meal templates; grocery generation, plan-change reconciliation, stores and aisles, the living list with the staples check and offline use; meals on the dashboard; API; authorization
status: draft
updated: 2026-11-08
drawio: Meals.drawio
drawio_src: src/Meals.py
depends_on: [D-006, D-015, D-019, D-021, D-035, D-068, D-072, D-075, D-093, D-094, D-096, D-105, D-108, D-109, D-110, D-111, D-112, D-113, D-114, D-115, D-116, D-117, D-118, D-119, D-120, D-121, D-122]
milestone: M5
research: [R-007]
---

# Meals: recipes, meal plan, templates, living grocery list (DRAFT v1.1)

This doc covers M5 (D-023, D-108). v1 is re-derived from the requirements, D-015, D-021 and the batch 24 answers (D-109 to D-119); it replaces the pre-answer v0. v1.1 adds the batch 25 answers: several entries per slot (D-120), an aisle order per store (D-121), and templates saved from the plan or duplicated (D-122).

**Not in M5:** meal suggestions and dietary preferences (D-021), export (D-015), pantry inventory beyond the staples flag (D-021), a cook per meal (D-110), repeating entries (D-112), meal notifications (D-119). The M4 follow-ups go to M4.2 (D-119).

## 1. Concepts
| Concept | What it is | Decided by |
|---|---|---|
| **Recipe** | A stored recipe: title, ingredient lines (each keeps its source text), steps, servings, times, tags, an image, a note, favourite, per-member ratings | D-094, D-109 |
| **Import** | URL (JSON-LD → microdata → LLM), pasted text (LLM), or the manual form; always an editable draft, stored only when an adult saves | D-109 |
| **Ingredient** | Catalogue entry: canonical name, aliases, staple flag, aisle, default store, default unit. Seeded per household from a hand-written list by `ensure_defaults` | D-021, D-114, D-115 |
| **Normaliser** | Deterministic: parse → clean → singularise → alias or trigram match → convert within a dimension. The LLM only proposes matches for lines the rules miss; an adult confirms | D-015, D-113, R-007 |
| **Meal slot** | Named part of the day with a time; editable; only Dinner shown by default | D-015, D-111 |
| **Meal plan entry** | A recipe or free text with servings, at a date and slot. A slot holds any number of entries, in order. Movable and swappable | D-112, D-113, D-120 |
| **Meal template** | Named, N days long; each day × slot holds zero or more items (recipe or free text). Made in the editor, saved from a plan range, or duplicated. Applied from a start date with a per-slot Replace/Add choice; applied entries are detached | D-112, D-120, D-122 |
| **Grocery list** | One per household, never closes; checked items archive after `grocery_archive_hours` (default 24) | D-021 |
| **Grocery item** | Text, optional ingredient, quantities, store, the meals it came from, manual or generated, checked state | D-015, D-021, D-115 |
| **Staples check** | Generated staple lines go to a separate section listing the meals that need them; "low" moves one onto the list | D-021 |
| **Store / aisle** | Editable lists. The household aisle order is the default, and each store has its own order and hidden aisles. The list groups by store, then aisle | D-115, D-121 |

## 2. Crates (page "Components")
| Crate | Role |
|---|---|
| `fa-domain` | `RecipeDraft`, `RecipeIngredient` (draft line), `RecipeOrigin` move here from `fa-recipe` (which re-exports them); `RecipeOrigin` gains `microdata`. The meals DTOs and authz actions |
| `fa-store` | Migration and repos for every §3 table (`GroceryStoreRepo` for `grocery_store`), the seed loader's insert, RLS |
| `fa-recipe` | Extraction (M4) plus a microdata parser over `scraper` and a pasted-text input; ingredient line parser extended per R-007. Stores nothing |
| `fa-meals` (new) | `Normaliser`, `RecipeService` (import, save, image), `PlanService` (entries, move/swap, templates preview/apply), `GroceryService` (generate, impact, reconcile, add/check/low, archive), job handlers, `ensure_defaults` and the ingredient seed |
| `fa-api` | The §5 routes; the dashboard payload gains today's meals; SSE `grocery.updated`, `mealplan.updated` |
| `fa-server` | Registers the jobs and the cron |
| `apps/web` | Food views, Settings → Meals, kiosk Food view and `meals` panel (Frontend.md §3f, §4) |

New dependencies (verified 2026-11-08 with `cargo search`/`cargo info`): `pluralizer` 0.6.0 (MIT or Apache-2.0), `image` 0.25.10 (MIT or Apache-2.0; features `jpeg`, `png`, `webp` only, no defaults). `strsim` and `scraper` are already workspace dependencies. `pg_trgm` ships in `pgvector/pgvector:pg16` (checked).

Coupling: `fa-meals` depends on `fa-domain`, `fa-store`, `fa-recipe`, `fa-llm`, `fa-connectors` (guarded `Http`). Only `fa-api` and `fa-server` depend on `fa-meals`.

## 3. Data model (page "Model")
Every table has `id uuid`, `household_id` (RLS like the rest of the schema), `created_at`, `updated_at`. Entities edited from several clients carry `version int` (optimistic concurrency: `If-Match` on the request, 409 when stale, as for tasks).

| Table | Columns | Notes |
|---|---|---|
| `recipe` | title, source_url?, servings?, yield_text?, prep_minutes?, cook_minutes?, total_minutes?, import_method `jsonld\|microdata\|llm\|manual`, tags text[], note?, favourite bool, created_by member, version | D-109 |
| `recipe_image` | recipe (PK), mime `image/jpeg\|image/webp\|image/png`, bytes bytea ≤ 512 KB, sha256, source `import\|upload` | Same pattern as `member_avatar`; in the DB, so backups carry it |
| `recipe_ingredient` | recipe, ord, raw_text, qty?, unit?, item?, note?, ingredient?, match_state `matched\|unmatched\|proposed`, proposal jsonb? | `raw_text` is always the source line; `proposal` = the LLM's `{ingredient_id?, new_name?, aisle?, confidence}` |
| `recipe_step` | recipe, ord, text | |
| `recipe_rating` | recipe, member, stars 1–5 | Unique (recipe, member); a member rates for themself (D-109, D-110) |
| `ingredient` | canonical_name (singular, lower case, unique per household), aliases text[], is_staple bool default false, aisle?, default_store?, default_unit?, origin `seed\|user\|llm` | `origin = llm` only after an adult confirms a proposal |
| `aisle` | name, ord | Seeded with the seed's aisles. `ord` is the household order ("Any store" and new stores) |
| `grocery_store_aisle` | store, aisle, ord, hidden bool | PK (store, aisle). A store's own order (D-121). Copied from `aisle.ord` when the store is created; a missing row means the household order, not hidden |
| `grocery_store` | name, ord | None by default (D-115). Not `store`: `Store` is the DB handle type in Rust |
| `meal_slot` | name, ord, time `"HH:MM"` local, visible bool | Seeded Breakfast 07:30, Lunch 12:00, Dinner 18:00, Snack 15:30; only Dinner visible (D-111) |
| `meal_plan_entry` | date, slot, ord, recipe? xor free_text?, servings?, created_by, version | Several per (date, slot), ordered by `ord` (D-120) |
| `meal_template` | name, days int ≥ 1, note?, version | D-112 |
| `meal_template_cell` | template, day int (1..days), slot, ord, recipe? xor free_text?, servings? | Several per (day, slot), ordered; empty slots have no rows (D-120) |
| `grocery_list` | — | Exactly one per household, created with it |
| `grocery_item` | list, ingredient?, text, qty?, unit?, store?, aisle? (cached from ingredient), section `list\|staples`, manual bool, added_by?, added_via_device?, checked_at?, checked_by?, archived_at?, version | `id` may be client-generated (D-118) |
| `grocery_item_source` | item, entry (meal_plan_entry), qty?, unit?, generated_for `daterange` | One per item and entry; drives D-116 |
| `meals_change` | kind `generate\|reconcile\|template_apply\|swap\|move`, actor member?, device?, inverse jsonb (the rows to restore or delete), expires_at (now + 10 min), undone_at? | The Undo for multi-row changes (there is no generic undo endpoint yet, only per-feature ones such as mail actions); pruned daily |
| `meals_settings` | grocery_archive_hours int default 24, ingredient_llm bool default true, trigram_threshold real default 0.45 | One row per household. `household.meal_plan_horizon_weeks` (default 2) and `household.units` stay on `household` |

Each new column's writer: recipe columns and images by the recipe editor (`PUT /recipes/{id}`, `POST /recipes/{id}/image`) and the `recipe.image` job; ratings by `PUT /recipes/{id}/rating`; catalogue, aisles, stores, slots and `meals_settings` by Settings → Meals; entries by the plan routes and template apply; items by the grocery routes and generation.

**Invariants**
- An entry and a template cell each hold exactly one of `recipe` and `free_text`.
- A `staples` item is never manual; "low" moves it to `list`.
- An item's generated quantity is the sum of its sources' quantities (converted), plus any manual edit made after.
- Hidden slots keep their entries; they are not drawn and not generated from.
- Everything an item, entry or cell references belongs to the same household.

## 4. Behaviour
### 4.1 Recipe import, save and images (D-109)
1. `POST /recipes/import {url}` or `{text}` → `RecipeDraft` (nothing stored; the page fetch uses the `allow_private_hosts` setting too): JSON-LD, then microdata (`origin: microdata`), then the LLM (`origin: llm`; needs an endpoint, else `422 llm_unavailable`). Pasted text goes straight to the LLM. The manual form starts from an empty draft.
2. The adult edits and saves (`POST /recipes`). Each line runs through the normaliser (§4.2); unmatched lines are stored with `match_state = unmatched` and queue `ingredient.llm` (§4.2 step 5).
3. If the draft had an `image_url`, `recipe.image` fetches it through `Http` with the instance's `allow_private_hosts` setting (off by default, as for calendars; the M4 route hard-coded it off), body cap 5 MB, decodes it with `image`, resizes to at most 800 px on the long side, encodes WebP (JPEG if WebP is unavailable), and stores it if ≤ 512 KB. Failure leaves no image and logs only the recipe id. Upload: `POST /recipes/{id}/image` (png, jpeg, webp ≤ 5 MB in, same resize).

### 4.2 Normaliser (D-113, R-007)
1. **Parse** with the `fa-recipe` rule parser, extended with fl oz, pint, quart, gallon and `1 (14 oz) can`: qty (fractions, unicode fractions, ranges → low end), unit, item, note.
2. **Clean** the item: lower case, ASCII folding, preparation words and parentheticals into `note` ("diced", "(15-oz)", "to taste"), an editable descriptor stop list ("kosher", "fresh", "large"), singularise with `pluralizer` plus an exceptions list.
3. **Match** in order: canonical name, alias, trigram similarity ≥ `trigram_threshold` (`pg_trgm`), else unmatched.
4. **Units:** three dimensions with canonical units ml, g and count; a hand-written table (tsp, tbsp, cup = 240 ml, fl oz, pint, quart, gallon, ml, l, g, kg, oz, lb, plus count words such as clove, can, piece). Convert only within a dimension; volume and weight of one ingredient stay two quantities on one item. Display in `household.units`, rounded to kitchen sizes (¼ tsp steps up to 1 tbsp, ¼ cup steps, 5 g / 25 g / 0.1 kg steps, ¼ lb steps).
5. **Unmatched lines:** stay on lists as their text. If `ingredient_llm` is on and an endpoint is configured, `ingredient.llm{recipe_ingredient_id}` sends the cleaned line and the five nearest catalogue names to the LLM (`purpose = "ingredient"`) for `{ingredient_id?, new_name?, aisle?, confidence}`; the result is stored as `proposal`, `match_state = proposed`. The adult accepts or picks another match in the editor; either way the line's cleaned item becomes an alias, so the next import matches by rules. The LLM never writes `ingredient` rows itself.

### 4.3 Meal plan (D-112, D-113)
- The plan covers today to `meal_plan_horizon_weeks` ahead for writes; earlier dates are read-only history.
- An entry is a recipe or free text; servings default to the recipe's.
- **Move** an entry to another date and slot (appended after the entries there), or **swap** two entries (`POST /meal-plan/swap {a, b}`). Each is one change with Undo (D-120).
- Any change to an entry that has unchecked grocery contributions returns a `grocery_impact` (§4.6).

### 4.4 Meal templates (D-112)
- A template has a name, a length in days and cells (day × slot × ord) holding a recipe or free text.
- **Made** in the editor, by **Save as template** from the plan (`POST /meal-templates/from-plan {name, from, to, slot_ids}`: length = days in the range, at most 366; every entry in those slots becomes a cell with the same order, recipe or text and servings), or by **Duplicate** (`POST /meal-templates/{id}/duplicate {name}`) (D-122). All copies are detached.
- **Preview** `POST /meal-templates/{id}/preview {start}` (`YYYY-MM-DD`) → for every target `(date, slot)` the template's items and the existing entries there. Hidden slots are included and marked.
- **Apply** `POST /meal-templates/{id}/apply {start, add_slots: [{date, slot_id}]}`, in one transaction. For every target slot, its existing entries are **replaced** (deleted) unless the slot is listed in `add_slots`, in which case the template's items are **added** after them. The UI preselects Add for slots holding any free-text entry and Replace otherwise (D-120). The result is one change with Undo and a `grocery_impact` for the replaced entries (§4.6).
- Applied entries copy the cell's content and keep no link to the template; editing either side never affects the other.
- No repeating entries; re-apply by hand.

### 4.5 Grocery generation (D-015, D-021, D-113)
1. Input: `{entry_ids[]}` or `{from, to}` (`YYYY-MM-DD`; visible slots only).
2. For each recipe entry and line: scale by entry servings / recipe servings (no scaling when either is null), normalise.
3. Staple ingredient → `section = staples`, grouped by ingredient with the entries that need it. Otherwise merge into an unchecked, unarchived `list` item with the same ingredient (and, without an ingredient, the same cleaned text); else insert with the ingredient's default store and aisle.
4. One `grocery_item_source` per (item, entry); an entry already generated for an item adds nothing (idempotent).
5. The whole run is one change with Undo (D-096); SSE `grocery.updated`.

### 4.6 Plan changes after generation (D-116)
- When an entry with unchecked contributions is deleted, replaced (template apply included), moved outside its `generated_for` range, or has its servings changed, the plan route returns `grocery_impact: [{item_id, text, from_qty, to_qty, remove}]`.
- The UI shows it and asks "Update the shopping list?". Yes calls `POST /grocery/reconcile {impact_token}`; No leaves the list.
- Checked, archived and manual items are never in an impact. An empty impact asks nothing.

### 4.7 The living list (D-015, D-021, D-115, D-118)
- **Add** from any view, the kiosk included without a PIN: free text matched through the normaliser to an ingredient when possible (store and aisle from it), else aisle "Other", "Any store".
- **Check off / uncheck**, **low** (staple → list): anyone. **Edit, delete, move store:** adults (D-110).
- **Grouping:** by store (in `grocery_store.ord`, "Any store" last), then by aisle. Within a store, aisles follow that store's order (`grocery_store_aisle`, falling back to `aisle.ord`). An aisle the store hides, or no aisle, goes to that store's "Other" (last). "Any store" uses `aisle.ord`. The Staples check section comes after. A store filter shows one store (D-121).
- **Archive:** the `grocery.archive` cron (every 15 min) sets `archived_at` on items checked more than `grocery_archive_hours` ago; archived items stay for history and are never shown on the list.
- **Offline (D-118):** the web client queues adds (with a client-generated `id`) and check-offs, and replays them in order. The API treats a replayed add with a known id as success, and a check-off on a newer `version` as still applied (check-offs win); a change to a deleted item is dropped. Over HTTPS the service worker also caches the shell and the last list (Frontend.md §1.1).

## 5. API (adult unless noted; plan/M5.md fixes the shapes)
| Endpoint | Purpose |
|---|---|
| `GET /recipes?q=&tag=&favourite=&before=` | Library (`{items, next_before}`); any member |
| `POST /recipes`, `GET/PUT/DELETE /recipes/{id}` | Create, read (any member), update, delete |
| `POST /recipes/import` | `{url}` or `{text}` → `RecipeDraft`; `POST /ai/recipes/extract` stays one milestone as an alias |
| `GET /recipes/{id}/image`, `POST /recipes/{id}/image`, `DELETE /recipes/{id}/image` | Image (GET any member) |
| `PUT /recipes/{id}/rating` | `{stars: 1..5 \| null}`; the caller's own rating; any signed-in member |
| `POST /recipes/{id}/lines/{line_id}/match` | `{ingredient_id}` or `{new_name, aisle?}`: accept or pick; adds the alias |
| `GET/POST /ingredients`, `PUT/DELETE /ingredients/{id}` | Catalogue, staples, aliases, aisle, default store |
| `GET/PUT /meals/settings`, `GET/PUT /meals/slots`, `GET/PUT /meals/aisles`, `GET/PUT /meals/stores` | Settings → Meals (lists replaced whole, ordered) |
| `GET/PUT /meals/stores/{id}/aisles` | A store's aisle order and hidden aisles (D-121) |
| `GET /meal-plan?from=&to=` | Entries in range; any member, the dashboard too |
| `POST /meal-plan`, `PUT/DELETE /meal-plan/{id}`, `POST /meal-plan/swap` | Write; each returns `{entry(s), grocery_impact}` |
| `GET/POST /meal-templates`, `GET/PUT/DELETE /meal-templates/{id}`, `POST /meal-templates/{id}/preview`, `POST /meal-templates/{id}/apply`, `POST /meal-templates/from-plan`, `POST /meal-templates/{id}/duplicate` | Templates |
| `GET /grocery` | `{stores, aisles, items, staples}`; any member and the dashboard |
| `POST /grocery/items` | Add (optional client `id`); anyone |
| `POST /grocery/items/{id}/check`, `/uncheck`, `/low` | Anyone |
| `PUT/DELETE /grocery/items/{id}` | Edit, move store, delete; adult |
| `POST /grocery/generate`, `POST /grocery/reconcile` | Generation and D-116 reconcile; return the applied change for Undo |
| `POST /meals/changes/{id}/undo` | Undo a multi-row change (generate, reconcile, template apply, swap, move) within 10 minutes; adult, or the same device for kiosk adds and check-offs (single-row edits use the D-096 instant-save Undo, which re-saves the old value) |

## 6. Authorization (D-110; adds to Domain.md's matrix)
| Action | child | adult | dashboard device | device + adult PIN |
|---|---|---|---|---|
| Read recipes, plan, templates, list | ✓ | ✓ | ✓ (no templates) | ✓ |
| Add a grocery item; check off, uncheck, mark low | ✓ | ✓ | ✓ | ✓ |
| Rate a recipe (own rating) | ✓ | ✓ | ✗ | ✗ |
| Edit, move or delete items; generate; reconcile | ✗ | ✓ | ✗ | ✓ |
| Manage recipes, the plan, templates, catalogue, stores, aisles, slots, settings | ✗ | ✓ | ✗ | ✓ |

Meals are not email-derived, so they may appear on the dashboard (D-068).

## 7. Frontend
See Frontend.md §3f (Food: Meal Plan, Shopping List, Recipes, Templates; Settings → Meals) and §4 (kiosk Food view, `meals` panel). Today's meals appear on the kiosk Home and the app's Today (D-117).

## 8. Jobs
| Job | Trigger | Does |
|---|---|---|
| `recipe.image` | recipe saved with an `image_url` | §4.1 step 3; max 3 attempts |
| `ingredient.llm` | a line saved unmatched | §4.2 step 5; skipped when no endpoint or `ingredient_llm` off; max 3 attempts |
| `grocery.archive` | cron `*/15 * * * *` | §4.7 |

**Seeding** is not a job: `fa_meals::ensure_defaults(conn)` (idempotent) loads the aisle and ingredient seed, the slots, the list and `meals_settings` (D-114). It runs next to `fa_weather::ensure_default_rules` in `setup_household` and in the server's startup seeder for every existing household.

## 9. Configuration
Every toggle is in `meals_settings` and edited in Settings → Meals (D-075); nothing in TOML. The horizon and units stay on `household`. The seed is a data file in `crates/fa-meals/seed/` (hand-written, AGPL, D-114).

## 10. Testing
- Normaliser table tests: fractions, ranges, `1 (14 oz) can`, plurals, aliases, descriptors, trigram threshold, conversion and rounding, no volume↔weight.
- Seed: unique canonical names, no alias shared by two ingredients, every aisle exists.
- Generation: merge, staples, idempotency, scaling, hidden slots skipped; impact and reconcile; template preview, apply with Replace and Add, save from plan, duplicate, Undo; per-store aisle order and hidden aisles.
- Microdata parser and image resize fixtures: fictional recipes and generated images written for the repo (D-035).
- Offline replay: duplicate add id, check-off against a newer version, change to a deleted item.
- e2e: import a fixture page served by the test server, plan it, apply a template over a free-text entry kept, generate, check off from the kiosk, add from the kiosk without a PIN.
