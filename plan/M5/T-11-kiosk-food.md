---
id: T-11
title: "Kiosk: Food view, meals panel, Week view meals"
milestone: M5
wave: 5
depends_on: [T-07, T-08, T-10]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/dashboard/**", "apps/web/src/stores/dashboard.ts", "apps/web/src/views/DashboardView.vue"]
decisions: [D-117, D-015, D-019, D-068, D-110, D-111]
---

# T-11: Kiosk Food view and meals panel

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser whose kitchen wall screen (an Echo Show in kiosk mode, `/dashboard`) is the main interface. M5 brings meals to the kiosk (D-117):
- a **Food** view in the kiosk bar with the meal plan and the shopping list;
- a **`meals` panel** showing today's meals on Home;
- each day's meals in the **Week** view.

Anyone at the kiosk adds and checks grocery items without a PIN (D-015, D-019). Edits and deletes need the adult PIN (D-110).

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md` (the kiosk rules).
- `plan/M5.md`: *Web* (Kiosk, shared components, e2e selectors) and *Domain types* (`DashboardMeal`, `DashboardSettings.food_view`).
- `arch/Frontend.md` §4 (all of it) and §3f; `arch/Meals.md` §7.
- Code:
  - `dashboard/DashboardCanvas.vue` (the `View` type, the `VIEWS` bar, Home, Week, Whiteboard);
  - `dashboard/panels/PanelHost.vue` (`REGISTRY`), `dashboard/layout/index.ts` (`PanelKind`, `KNOWN_PANELS`, `PRESETS`, the portrait groups);
  - `dashboard/editor.ts` and `LayoutEditor.vue` (the device settings editor);
  - `dashboard/strings.ts`, `dashboard/types.ts`, `stores/dashboard.ts` (`handleEvent`, elevation);
  - T-07's `components/food/plan/MealPlanGrid.vue` and T-08's `components/food/grocery/ShoppingList.vue` (props in their reports).

## You own
The paths in `owns`.

## Work
1. **Food view:**
   - `View` gains `'food'`. Its bar button sits between Week and Whiteboard (`kiosk-view-food`) and shows only when `settings.food_view` is true.
   - The view has two big tabs, Meal plan and Shopping list:
     - Meal plan is `MealPlanGrid` with `fill`, `editable: false` and `from` = the start of the previous week;
     - Shopping list is `ShoppingList` with `kiosk`, and `canEdit` = the device is elevated.
   - It follows the kiosk's orientation rules (bar on the shorter edge, D-090) and fills the screen without scrolling on the plan tab.
2. **`meals` panel** (`panels/MealsPanel.vue`, `panel-meals`):
   - today's `data.meals` (slot name, time, title) in slot order; hidden when empty, unless it is the only panel in its column;
   - add it to `PanelKind`, `KNOWN_PANELS`, `REGISTRY` and the **Family hub** preset (after `agenda`), and to the right portrait group.
3. **Week view meals:** the day cells (`DayCell.vue` / `WeekTablePanel.vue`) show that day's meals at the top of the day cell, as one compact line per visible slot ("Dinner: Lentil soup").
   - The week's entries come from `GET /meal-plan?from=&to=` (the device is allowed), fetched with the week and refreshed on `mealplan.updated`.
   - The existing fit rules (D-087) count the meals line.
4. **Live:** `stores/dashboard.ts` `handleEvent` refreshes on `mealplan.updated` (payload and the week's meals) and `grocery.updated` (the Food list only).
5. **Settings:** a "Food view" switch in the device settings editor (`food_view`, default on).
6. **Tests** (`dashboard/*.test.ts`):
   - the Food button is hidden when `food_view` is false;
   - the meals panel orders by slot and hides when empty;
   - the Family hub preset includes `meals`;
   - an older saved layout without `meals` still loads;
   - the Week cell shows the meals line;
   - the kiosk list shows delete only when elevated.

## Acceptance
- [ ] In `apps/web`: `lint`, `typecheck`, `test` and `build` pass, then `format`.
- [ ] Screenshots of the kiosk Food view in landscape and portrait via `scripts/ui-shots.sh` with an ad-hoc route list (T-12 adds the permanent routes in wave 6). If Docker is busy, say so. The report lists the PNG paths.
- [ ] The report says "list what outside your paths needs changing". README changes needed: none.
