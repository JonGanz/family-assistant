---
id: T-04
title: "Meal plan: compact add, Today; recipe back button"
milestone: M5.3
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/views/food/MealPlanView.vue", "apps/web/src/views/food/RecipeView.vue", "apps/web/src/views/food/RecipeEditView.vue", "apps/web/src/views/food/plan.test.ts", "apps/web/src/views/food/recipes.test.ts", "apps/web/src/views/food/planAgendaRange.ts", "apps/web/e2e/m53.spec.ts"]
decisions: []
---

# T-04: Meal plan: compact add, Today; recipe back button

You are a sub-agent on milestone M5.3 of the Family Assistant project (Vue 3 + TypeScript PWA in `apps/web`).

User requests:
1. "When logged in and looking at the Food > Meal Plan page, let's compact what we see. For each date, move a simple [+] button that is right aligned and always present, rather than a separate row."
2. "If the meal plan view also has such navigation [a Today button to come back when you navigated far away], ensure there is one there as well."
3. "Fix the 'back' button on the logged in recipe view. It takes you back to the recipe list page, rather than back. For example, if you open a recipe from the meal plan page, then hit the top-left back button, it takes you to the recipes tab rather than your previous view."

## Read first
- `plan/_rules-common.md`, `plan/M5.3/_rules.md`, `plan/M5.3.md`
- `views/food/MealPlanView.vue` (the agenda: per day a `.agenda__day` with `h2.agenda__head`, per slot a `.slot` with a chip list and a `.slot__add` button below it; `openAdd(d, slotId)`, `addToday`, `writable(d)`, the EntrySheet; the top actions row), `planAgendaRange.ts`, `plan.test.ts`
- `views/food/RecipeView.vue` (`<UiTopAppBar back @back="router.push('/food/recipes')">`, `router.push` after delete), `RecipeEditView.vue` (its back/cancel/save navigation)
- `apps/web/e2e/m5.spec.ts`, `m51.spec.ts`, `m52.spec.ts` for how food e2e specs log in and create recipes

## You own
The paths in `owns`. Don't edit `ui/**`, `components/**`, stores (report needs).

## Work
1. **Compact add:** in each writable day's heading row, a single right-aligned icon `[+]` button (icon `add`, 48 px touch target, `aria-label` "Add meal on <heading>") that opens the add sheet for that date; it is always shown on writable days (also when the day has meals; also when selecting is off) and the per-slot add buttons below the lists go away. With several slots the sheet still lets the user choose the slot (when `openAdd` needs a slot, pass the first slot; check `EntrySheet` allows changing it, else report). Keep the compact look: the heading and the button share one row; slot lists sit directly under it; no extra empty row per slot. Keep or replace `data-testid`s deliberately: existing tests and e2e specs use `plan-slot-add-<date>-<slot>`; update those tests/specs you own, and report specs you don't own that break (grep `plan-slot-add` across `apps/web/e2e`, `e2e/` and `src`).
2. **Today on the plan page:** the agenda starts two days ago; add a **Today** control (button in the actions row or beside the heading area, always visible) that scrolls today's `.agenda__day` into view (smooth unless `prefers-reduced-motion`), and is inactive when today's section is already in view. If the page has other date navigation (prev/next, a horizon), make Today restore it too; check and say what you found.
3. **Back button:** `RecipeView` back goes to the previous page: `router.back()` when the router has history (`window.history.state?.back` is a string), otherwise push `/food/recipes`. Same rule in `RecipeEditView` for its back button (after Save or Delete keep going where they go today, but Save should return to the recipe). Make it a small shared helper in `views/food/` (a new file you own, e.g. `goBack.ts`) with a unit test; do not use it in Cancel flows that intentionally return elsewhere without saying why.
4. Tests: plan tests for the heading `[+]` (present, opens the sheet for that date), the removed slot row, the Today control; recipes tests for back with and without history. e2e `m53.spec.ts`: open the plan page, open a recipe from a meal chip, press the top-left back, land on the plan page; add a meal through the heading `[+]`.

## Acceptance
- [ ] Web checks from `_rules.md` green
- [ ] `m53.spec.ts` written (the main session runs e2e)
- [ ] Report: what changed, specs outside your paths that need changes
