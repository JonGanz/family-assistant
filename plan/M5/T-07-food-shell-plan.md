---
id: T-07
title: "Food shell, navigation and routes, Meal plan, Templates"
milestone: M5
wave: 4
depends_on: [T-06]
agent: {model: sonnet, effort: high}
status: done
owns: ["apps/web/src/app/navigation.ts", "apps/web/src/stores/live.ts", "apps/web/src/stores/mealPlan.ts", "apps/web/src/views/food/MealPlanView.vue", "apps/web/src/views/food/TemplatesView.vue", "apps/web/src/views/food/TemplateEditView.vue", "apps/web/src/components/food/FoodTabs.vue", "apps/web/src/components/food/plan/**", "apps/web/src/views/food/plan.test.ts"]
decisions: [D-112, D-116, D-117, D-111, D-110, D-120, D-122, D-096, D-105]
---

# T-07: Food shell, Meal plan and Templates

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Vue 3 PWA (Pinia, the in-house `ui/` component library on theme variables). M5 adds a **Food** destination with the tabs Meal plan, Shopping list, Recipes and Templates. You build the shell (navigation, the tab row, live events), the **Meal plan** tab with the reusable grid, and the **Templates** tab with its editor and the apply flow. Three other tasks build the other tabs and settings in parallel and rely on the names you create.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md` (UI rules).
- `plan/M5.md`: the *API* table and *Web*, especially routes, live kinds, shared components, icons and e2e selectors.
- `arch/Frontend.md` §1.3, §1.4, §1.5, §2 and **§3f** (Meal plan, Templates); `arch/Meals.md` §4.3 to §4.6.
- `apps/web/src/api/food.ts` (written by the main session; use only it for calls); `apps/web/src/gen/`.
- Patterns:
  - `stores/lists.ts`, `stores/live.ts`, `app/navigation.ts`;
  - `views/CalendarView.vue` (today's inverted marker, week rows);
  - `ui/composables/useInstantSave.ts` and `useFormDraft.ts`, `ui/UiSheet`, `UiFilterChips`, `UiSegmented`.

## Note from the scaffold
`api/food.ts` exports `food` (app) and `foodKiosk` (device token). T-11 reuses your shared component on the kiosk, so give it an optional `api?: FoodApi` prop (default `food`), and make your store accept it too, or take it from the component.

## You own
The paths in `owns`. The main session's scaffold already holds `api/food.ts`, every route (`router/index.ts`), stub views and stores, and the icon keys (`ui/icons.ts`); those three files are frozen: report anything missing. Replace the `FoodTabs.vue` stub **first**, since T-08 to T-10 use it (props: none; it reads the route and the role).

## Work
1. **Navigation:** a Food destination (`id: 'food'`, icon `food`, `to: '/food'`, primary) in `DESTINATIONS`. The expanded drawer group "Food" lists Meal plan, Shopping list, Recipes, and Templates (adult only).
2. **Active state:** check that `/food/*` marks Food active (the scaffold's redirect reads `fa.food.tab`). Report a fix if it doesn't.
3. **Live:** add `mealplan.updated`, `grocery.updated` and `recipes.updated` to `AppLiveKind` and `KINDS`, and dispatch them to `useMealPlanStore().invalidate`, `useGroceryStore().invalidate` and `useRecipesStore().invalidate`. The stores exist as scaffold stubs, so import them by name.
4. **`FoodTabs.vue`:** `UiSegmented` on compact, a tab row otherwise. Templates is shown to adults only. It writes `fa.food.tab`. `data-test` keys `food-tab-*`.
5. **`stores/mealPlan.ts`:** `entries` by date range, `templates`, `load(from, to)`, `add`, `update`, `remove`, `swap`, `preview`, `apply`, `undo(changeId)`, and `invalidate(kind)` (refetch on `mealplan.updated` and `resync`). Plan writes return `grocery_impact`; expose it to the view.
6. **`components/food/plan/MealPlanGrid.vue`:** props `{from, weeks, editable, fill?}`. Week rows start on `week_start`, with seven columns on medium and expanded windows and a day list on compact. Each cell shows the visible slots in order (`useMealsSettingsStore().visibleSlots`, T-10), each listing its entries in `ord` order; a slot can hold several (D-120). Today is drawn inverted like the calendar's. With `fill`, rows fit the height with no scrolling (the kiosk). Entry chips, empty "+" slots when `editable`, and drag on pointer devices (onto a slot appends there, onto an entry swaps) plus a "Move to…" menu.
7. **`MealPlanView.vue`:**
   - the grid from the previous week to the horizon, scrolling up for past weeks (read-only);
   - `?add=<recipe id>` (from T-09's "Add to plan") opens the add sheet with that recipe;
   - the add/edit sheet (recipe search via `food.ts`, or free text; servings);
   - delete with Undo; swap/move with Undo (`change_id`);
   - the **impact dialog** (D-116: lists items; "Update the list" calls `reconcile`, "Leave the list" does nothing);
   - **Make grocery list:** a range (default the coming week) or a selection mode in the grid; a summary; Undo;
   - **Apply template:** pick a template and a start date, then the preview with a conflict list per slot showing its entries and a Replace/Add choice (Add preselected when the slot holds free text), then apply with Undo (D-120);
   - **Save as template:** a range (from, to) and slot picker plus a name; `POST /meal-templates/from-plan`, then open the new template (D-122);
   - children see it read-only.
8. **Templates:** `TemplatesView.vue` (list: name, length, slots used; new; **Duplicate** with a name prompt; delete with confirm) and `TemplateEditView.vue` (a form with `useFormDraft` and `UiSaveBar`: name, days, and a grid of day × visible slots where each cell holds zero or more items (recipe or free text), in order; week rows when `days` is a multiple of 7). 
9. **Tests** (`views/food/plan.test.ts`, vitest):
    - the grid's start row and today marker;
    - only visible slots drawn;
    - the impact dialog shows and calls reconcile;
    - template conflicts preselect Add for slots with free text and Replace otherwise;
    - a slot with two entries renders both in order;
    - save as template sends the range and slots;
    - the Food tab redirect memory;
    - children get no edit controls.

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass, then `corepack pnpm@9 run format`.
- [ ] All the e2e `data-test` keys for your views from *Web* are present.
- [ ] The report lists:
  - the shared names created (live kinds, `FoodTabs`, `MealPlanGrid` props);
  - "list what outside your paths needs changing";
  - README changes needed: none.

## Outcome (T-07 report, 2026-11-08)
- **Built:**
  - `FoodTabs.vue`, the Food destination and `FOOD_ENTRIES` / `NavModel.drawer` in `app/navigation.ts`;
  - `stores/live.ts` (the three kinds, dispatched to the plan, grocery, recipes and meals-settings stores);
  - `stores/mealPlan.ts` (chunked ranges, impact, generate, templates);
  - `components/food/plan/` (`MealPlanGrid`, `EntrySheet`, `ContentPicker`, `ImpactDialog`, `ApplyTemplateSheet`, `MakeListSheet`, `SaveAsTemplateSheet`, `planModel.ts`);
  - `MealPlanView`, `TemplatesView` and `TemplateEditView`;
  - `plan.test.ts` (10 tests);
  - `app/navigation.test.ts` updated, outside its owns at the main session's request.
- **Results** (reported): lint 0 errors, typecheck ok, 651 tests in 71 files, build ok before a final prop-default tweak.
- **Deviations:**
  - past weeks come through a "Show earlier weeks" button;
  - "Move to…" is the date and slot fields in the edit sheet;
  - delete's Undo re-adds the entry, at the end of its slot (the server returns no `change_id` for a delete);
  - add has no Undo;
  - it used `data-testid`.
- **Main session at integration:**
  - `AppLayout.vue` now builds the drawer from `nav.drawer`, so the Food group lists its tabs;
  - every Food file was normalised from `data-test` to `data-testid`.
