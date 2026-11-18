---
id: T-02
title: "Backend: recipe sections, step ids, scale and column settings"
milestone: M5.1
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-store/**", "crates/fa-domain/**", "crates/fa-meals/**", "crates/fa-api/**", "crates/fa-recipe/src/**"]
decisions: [D-124, D-125]
---

# T-02: Backend: recipe sections, step ids, scale and column settings

You are a sub-agent implementing one task of milestone M5.1 of the Family Assistant project (self-hosted family organiser, Rust backend with axum + sqlx + Postgres, Vue frontend).

## Read first
- `plan/_rules-common.md`, `plan/M5.1/_rules.md`
- `plan/M5.1.md`: *Contracts fixed up front* (Migration, Types, Store and service)
- `decisions/D-124-recipe-sections.md`, `decisions/D-125-recipe-live-scaling.md`; `arch/Meals.md` §3 and §4.1 2a/2b
- `crates/fa-store/migrations/20261108000001_m5_meals.up.sql` (RLS pattern), `crates/fa-store/src/repo/meals_recipe.rs` (`RecipeLineRepo::replace_for_recipe`, `RecipeStepRepo`), `crates/fa-store/src/repo/meals_grocery.rs` (`SETTINGS_COLS`, settings get/update), `crates/fa-domain/src/{ids.rs,recipe.rs,meals_api.rs}`, `crates/fa-meals/src/{recipes.rs,settings.rs}`, `crates/fa-api/src/meals.rs`

## You own
The paths in `owns`. In `crates/fa-recipe/src/**` make **field-only** edits (add `sections: vec![]` where a `RecipeDraft` is built); the import logic is T-03's in wave 2.

## Work
1. Migration `20261117000001_recipe_sections.{up,down}.sql` exactly as in the plan.
2. Domain types as in the plan (`RecipeSectionId`, `RecipeStepId`, `DraftSection`, `RecipeDraft.sections`, `RecipeSectionInput`, `RecipeInput.sections`, `RecipeSectionView`, `RecipeView.sections`, `RecipeLineView.section_id`, `RecipeStepView`, `RecipeView.steps: Vec<RecipeStepView>`, `RecipeView.scale_factors`, `RecipeView.section_columns`, the two settings fields on view and patch). Keep ts-rs and utoipa derives like the neighbours.
3. Store: `RecipeSectionRepo`; `NewLine.section_id`; `RecipeLineRow.section_id`; step rows with id and section; settings columns read and written.
4. Service: create/update write sections (validated: 1..100 chars trimmed, ≤ 20 sections → 422 like other validation errors), lines and steps in the global `ord` order the plan states; get returns sections, `section_id`s, step objects, and the household's factors and columns. Settings patch validates and normalises factors (sorted, deduped, 1 added) and columns (1..=4).
5. Fix every construction site and test in your paths. Add tests: create a recipe with shared lines, two sections and finish steps → read back grouped correctly; update that moves a line between sections keeps its id and match; deleting a section by saving without it moves nothing to another section (its lines and steps are simply not in the new body); settings defaults are `[0.5,1,1.5,2,3]` and `2`; patch normalises `[2,0.5]` to `[0.5,1,2]`; columns 5 → 422; an API test (`crates/fa-api/tests/`) that `GET /recipes/{id}` returns `sections`, `steps[].id`, `scale_factors`, `section_columns`.

## Acceptance
- [ ] fresh container (port 55402): `SQLX_OFFLINE=true DATABASE_URL=... cargo test -p fa-store -p fa-meals -p fa-api` pass
- [ ] `cargo test -p fa-recipe -p fa-domain` pass
- [ ] `cargo clippy -p fa-store -p fa-domain -p fa-meals -p fa-api -p fa-recipe --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] `SQLX_OFFLINE=true cargo check --workspace`
- [ ] report: files, test results with counts from output, and anything outside your paths that needs changing (the web app will break on `steps` until wave 2: that is expected, just list the web files you saw use `RecipeView.steps`)
