---
id: T-04
title: "App: recipe page sections, editor sections, Settings → Meals, e2e"
milestone: M5.1
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/views/food/RecipeView.vue", "apps/web/src/views/food/RecipeEditView.vue", "apps/web/src/views/food/recipes.test.ts", "apps/web/src/components/food/recipes/**", "apps/web/src/views/SettingsMealsView.vue", "apps/web/src/views/parts/meals/**", "apps/web/src/stores/recipes.ts", "apps/web/src/stores/mealsSettings.ts", "apps/web/e2e/m51.spec.ts", "apps/web/e2e/stubs/**"]
decisions: [D-124, D-125]
---

# T-04: App: recipe page sections, editor sections, Settings → Meals, e2e

You are a sub-agent implementing one task of milestone M5.1 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`).

## Read first
- `plan/_rules-common.md`, `plan/M5.1/_rules.md`, `plan/M5.1.md` (Types, Web scaffold)
- `decisions/D-124-recipe-sections.md`, `decisions/D-125-recipe-live-scaling.md`; `arch/Frontend.md` §3f (Recipes, Settings → Meals)
- The regenerated types in `apps/web/src/gen/` (`RecipeView.sections`, `steps: RecipeStepView[]`, `RecipeInput.sections`, `RecipeDraft.sections`, settings fields), `apps/web/src/lib/recipeSections.ts` (`groupRecipe`, read-only for you), your owned files, `apps/web/e2e/m5.spec.ts` and `apps/web/e2e/stubs/server.mjs`

## You own
The paths in `owns`. Don't edit `lib/recipeSections.ts`, `api/food.ts`, `ui/**` or `dashboard/**` (report needs).

## Work
1. **Recipe page** (`RecipeView.vue`): use `groupRecipe`. Shared ingredients card full width; one column per section with an ingredients card above a steps card (CSS grid, `repeat(min(n, section_columns), 1fr)`; one column under 600 px); finish steps card full width below ("Steps" heading when there are no sections, "Finish" when there are). Keep the matched-ingredient display per line. A recipe without sections looks as before. Steps are now objects (`s.text`).
2. **Editor** (`RecipeEditView.vue` and the recipe components): load sections from `RecipeView` and from an import `RecipeDraft`; edit as shared lines + finish steps + a list of sections each with name, lines, steps. Actions: add section, rename, move up/down, merge into another (appends its lines and steps), delete (its lines and steps go to shared / finish); every line and step can be moved to another section (a select or "Move to…" menu). Save sends `RecipeInput.sections`. Unmatched-line highlighting and proposals keep working.
3. **Settings → Meals:** a "Recipes" card: scale factors (chips with remove, an add field accepting 0.1..10; 1 cannot be removed) and section columns (1..4 segmented control), saved via the existing settings store per D-096.
4. **Unit tests** (`recipes.test.ts` or new tests in your paths): editor round trip of sections into `RecipeInput`; merge and delete move items as stated; the page renders shared, two sections and finish in that order.
5. **e2e** `apps/web/e2e/m51.spec.ts` (+ a fictional sectioned JSON-LD page served by `e2e/stubs/server.mjs`, following how m5 serves its fixture): import the page, see two sections in the editor, save, the recipe page shows two section columns and a Finish card. Don't run e2e yourself (the main session does); make sure it typechecks and lints.

## Acceptance
- [ ] in `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test`, `build` pass (failures only in T-05's files may be ignored: name them), then `corepack pnpm@9 run format`
- [ ] report: files, results with counts from output, and anything outside your paths that needs changing
