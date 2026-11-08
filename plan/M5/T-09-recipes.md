---
id: T-09
title: "Recipes: library, recipe page, import, editor"
milestone: M5
wave: 4
depends_on: [T-06]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/stores/recipes.ts", "apps/web/src/views/food/RecipesView.vue", "apps/web/src/views/food/RecipeView.vue", "apps/web/src/views/food/RecipeEditView.vue", "apps/web/src/components/food/recipes/**", "apps/web/src/views/food/recipes.test.ts"]
decisions: [D-109, D-110, D-113, D-096, D-105]
---

# T-09: Recipes

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Vue 3 PWA. M5 stores recipes. They come in from a link (schema.org data, else the local LLM), from pasted text, or typed in, and always through an editable draft that is stored only on Save (D-109). Ingredient lines are matched to a catalogue, and lines the rules could not match may carry an LLM proposal that an adult accepts or corrects (D-113). Every member rates recipes for themself. You build the Recipes tab: library, recipe page, the "new recipe" flow and the editor.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: the *API* table (`/recipes*`, `/ingredients`), *Domain types* (`RecipeSummary`, `RecipeView`, `RecipeLineView`, `IngredientProposal`, `RatingsView`, `RecipeDraft`) and *Web*.
- `arch/Frontend.md` §3f (Recipes) and §1.4; `arch/Meals.md` §4.1 and §4.2.
- `apps/web/src/api/food.ts`, `apps/web/src/gen/`, `stores/lists.ts`, `ui/` (`UiSheet`, `UiSaveBar`, `useFormDraft`, `useInstantSave`, `UiFilterChips`).

## You own
The paths in `owns`. The scaffold (main session) already has your routes, stub views and stub store; replace the stubs. Don't edit `router/index.ts`, `api/food.ts` or `ui/icons.ts`.

## Work
1. **`stores/recipes.ts`:**
   - `page` (cursor paging), `query` (q, tags, favourite), `load`/`more`;
   - `current` (`RecipeView`) with `open(id)`, `save`, `remove`, `favourite`, `rate`, `matchLine`, `importDraft`;
   - `invalidate(kind)` on `recipes.updated` and `resync`.
2. **`RecipesView.vue`:**
   - search (title and ingredient) and tag chips (`UiFilterChips`, D-105) built from the tags in the loaded page, plus a Favourites chip;
   - cards with the image (`GET /recipes/{id}/image` when `has_image`), title, total time, average rating and last planned;
   - "New recipe" (`recipe-new`), adults only.
3. **`RecipeView.vue`:**
   - image, servings, times, tags, ingredient lines (the matched ingredient name on wide screens), steps, the household note;
   - **ratings:** the caller's own 1 to 5 stars, instant with Undo (`recipe-rating`), and the members' ratings below;
   - "Add to plan" (navigates to `/food/plan?add=<recipe id>`; T-07 opens its add sheet from that query);
   - Edit and Delete for adults (delete confirms: "planned meals keep the name as text").
4. **New recipe sheet:** three choices, **From a link** (`recipe-import-url`, `recipe-import-submit`), **Paste text** and **Type it in**. Import shows progress. The errors `no_recipe`, `fetch_failed`, `llm_unavailable` and `llm_failed` show a plain message with a "Type it in" fallback. On success open `RecipeEditView` with the draft in memory (route state); nothing is stored yet.
5. **`RecipeEditView.vue`:**
   - `useFormDraft` plus `UiSaveBar`: title, servings, times, tags, note;
   - image: keep the imported URL (sent as `image_url` on create), upload (POST bytes after save), or remove;
   - lines (`recipe-line-<ord>`, one per line, reorder, add, remove) and steps;
   - on an existing recipe, **unmatched** lines are highlighted. A `proposed` line shows the proposal as a chip with **Accept**, **Pick another** (catalogue search) and **New ingredient** (name and aisle), each calling the match route;
   - Save (`recipe-save`); 409 shows "changed elsewhere, reload" (D-096).
6. **Tests** (`views/food/recipes.test.ts`):
   - the import error fallback;
   - the draft is not stored before Save;
   - a proposal chip Accept calls the match route;
   - a rating with Undo;
   - a child sees no New, Edit or Delete;
   - the tag chips filter.

## Acceptance
- [ ] In `apps/web`: `lint`, `typecheck`, `test` and `build` pass, then `format`.
- [ ] Your e2e `data-test` keys are present.
- [ ] The report lists "list what outside your paths needs changing" and README changes needed: none.
