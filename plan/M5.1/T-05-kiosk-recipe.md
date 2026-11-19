---
id: T-05
title: "Kiosk: recipe window sections, live scaling, id-keyed ticks"
milestone: M5.1
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/dashboard/details/RecipeDetail.vue", "apps/web/src/dashboard/details/recipeChecks.ts", "apps/web/src/dashboard/details/recipeChecks.test.ts", "apps/web/src/lib/recipeScale.ts", "apps/web/src/lib/recipeScale.test.ts", "apps/web/src/dashboard/strings.ts"]
decisions: [D-124, D-125]
---

# T-05: Kiosk: recipe window sections, live scaling, id-keyed ticks

You are a sub-agent implementing one task of milestone M5.1 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`; the kiosk is the wall-screen dashboard).

## Read first
- `plan/_rules-common.md`, `plan/M5.1/_rules.md`, `plan/M5.1.md` (Types, Web scaffold, Scaling)
- `decisions/D-124-recipe-sections.md`, `decisions/D-125-recipe-live-scaling.md`; `arch/Frontend.md` §4 *Recipe window*
- `apps/web/src/dashboard/details/RecipeDetail.vue` (current layout: image card filling its card, details card, two-column ingredients, larger steps, tick boxes), `recipeChecks.ts`, `apps/web/src/lib/recipeSections.ts` (read-only), the regenerated types in `apps/web/src/gen/`

## You own
The paths in `owns`. Don't edit `lib/recipeSections.ts`, `api/**`, `ui/**` (report needs).

## Work
1. **Sections:** with `groupRecipe`, keep the top row (image + details), then shared ingredients full width (two columns inside the card as today), then one column per section (ingredients card above steps card; `section_columns` columns before wrapping, one under 600 px; inside a section card the ingredients are one column), then finish steps full width (heading "Steps" without sections, "Finish" with). A recipe without sections looks exactly as now. Steps are objects now (`s.text`).
2. **Ticks by id:** `recipeChecks.ts` stores line and step **ids** (strings) instead of indexes; bump the storage key/version so old index data is dropped; keep pruning. Update its tests.
3. **Scaling:** `lib/recipeScale.ts` per the plan (`formatQty`, `scaleLine`) with table tests (½ → "½", 0.75×2 → "1½", 1/3×1.5 → "½", 2.5 → "2½", 0.2 → "0.2", ranges "1-2 cups" ×2 → "2-4 cups", "1 1/2 cups" ×2 → "3 cups", no qty → unchanged, factor 1 → `raw_text`). In the header, a segmented row of factors from `recipe.scale_factors` (1 added if missing, labels "½×", "1×", "1.5×", "2×", "3×" via `formatQty`), 48 px targets, starting at 1× on every open (component state only). Lines render `scaleLine`; the Servings fact shows the scaled servings (`formatQty`). Step text is unchanged. Strings go into `dashboard/strings.ts`.

## Acceptance
- [ ] in `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test`, `build` pass (failures only in T-04's files may be ignored: name them), then `corepack pnpm@9 run format`
- [ ] report: files, results with counts from output, and anything outside your paths that needs changing
