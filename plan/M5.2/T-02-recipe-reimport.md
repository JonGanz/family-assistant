---
id: T-02
title: "Re-import a recipe from its source URL"
milestone: M5.2
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/food/RecipeView.vue", "apps/web/src/views/food/RecipeEditView.vue", "apps/web/src/views/food/recipes.test.ts", "apps/web/src/stores/recipes*.ts", "apps/web/e2e/m52.spec.ts", "apps/web/e2e/fixtures/recipes/reimport*.html", "crates/fa-meals/tests/recipes.rs"]
decisions: [D-126]
---

# T-02: Re-import a recipe from its source URL

You are a sub-agent implementing one task of milestone M5.2 of the Family Assistant project (Rust backend, Vue 3 + TypeScript PWA in `apps/web`).

## Read first
- `plan/_rules-common.md`, `plan/M5.2/_rules.md`, `plan/M5.2.md`, `decisions/D-126-recipe-reimport.md`, `arch/Meals.md` §4.1 (2, 2a, 2c)
- `apps/web/src/views/food/RecipeView.vue`, `RecipeEditView.vue` (`fromView`, `fromDraft`, `store.pendingDraft`), `components/food/recipes/NewRecipeSheet.vue` (how an import is started and errors shown; read-only), `api/food.ts` (`importRecipe`, read-only), the recipes store under `apps/web/src/stores/`
- `apps/web/e2e/m51.spec.ts` and `e2e/fixtures/recipes/sections.html` (how e2e serves a recipe page through the stub), `crates/fa-meals/tests/recipes.rs`

## You own
The paths in `owns`. If the recipes store lives elsewhere, report it instead of editing. Don't edit `api/**`, `ui/**`, `gen/**`, `components/**` (report needs).

## Work
1. **Recipe page:** for adults, when `source_url` is set, a "Re-import from source" action next to Edit. It opens a confirmation ("Ingredients, steps and sections will be replaced by the page's current version. Nothing is saved until you press Save.") and on confirm navigates to the editor with `?reimport=1`.
2. **Editor:** also offers the action (same confirmation) when editing a recipe with a `source_url`. On `reimport=1` (or the button), after loading the recipe, call `importRecipe({ url: source_url })` (check the request shape in `api/food.ts`) with a busy state; on success replace in the form the ingredients, steps, sections (via `fromDraft`), yield, servings and times and set `import_method` to the draft's origin; keep title, tags, note and photo. Show a notice that the form holds the re-imported version. On failure show the error and leave the form as loaded. Save stays the ordinary update (`PUT /recipes/{id}` with `If-Match`), Cancel discards. Remove `reimport` from the URL once handled so a reload does not import again.
3. **Unit tests** in `recipes.test.ts` for the form replacement (kept and replaced fields).
4. **Backend proof** in `crates/fa-meals/tests/recipes.rs`: create a flat recipe, add a meal plan entry, a template entry and a rating that use it, update it with sections (as a re-import would), then assert the same recipe id, the plan entry, template entry and rating are still there and an unchanged line kept its match. Use the existing helpers in that file.
5. **e2e** `apps/web/e2e/m52.spec.ts`: a recipe imported from a fictional flat fixture page, rated and added to the plan; the fixture is then served with sections (add `reimport-*.html` fixtures and swap via the stub the same way m51 serves fixtures; if the stub cannot change a page's body, serve the sectioned version at the same URL with a query toggle or report what is needed); re-import, Save; the recipe page shows the sections, the rating and the plan entry remain.

## Acceptance
- [ ] Web checks from `_rules.md` green; `cargo test -p fa-meals --test recipes` green against a fresh Postgres per `_rules-common.md`; `cargo fmt` clean
- [ ] `m52.spec.ts` written (the main session runs e2e)
- [ ] Report: files changed, test names, anything outside your paths that needs changing
