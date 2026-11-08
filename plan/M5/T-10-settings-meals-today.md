---
id: T-10
title: "Settings → Meals, the meals settings store, Today's meals card"
milestone: M5
wave: 4
depends_on: [T-06]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/stores/mealsSettings.ts", "apps/web/src/views/SettingsMealsView.vue", "apps/web/src/views/parts/meals/**", "apps/web/src/views/settingsNav.ts", "apps/web/src/views/HomeView.vue", "apps/web/src/components/food/TodayMeals.vue", "apps/web/src/views/settingsMeals.test.ts"]
decisions: [D-111, D-114, D-115, D-121, D-021, D-015, D-113, D-117, D-096]
---

# T-10: Settings → Meals and Today's meals

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Vue 3 PWA. M5 adds meals. Households configure them in **Settings → Meals**:
- the meal slots (only Dinner shown by default, D-111);
- the plan horizon;
- the stores and aisles the shopping list groups by (D-115);
- the ingredient catalogue with staples (D-021, D-114);
- the matching settings (D-113).

You build that screen and the shared `useMealsSettingsStore` the other Food tasks read. You also add "Today's meals" to the app's Today page (D-117).

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: the *API* table (`/meals/*`, `/ingredients*`) and *Web* (the `useMealsSettingsStore` names are a contract; write them **first**).
- `arch/Frontend.md` §3c (the Settings area), §3f (Settings → Meals), §1.4; `arch/Meals.md` §3 and §9.
- `views/settingsNav.ts`, `views/SettingsHouseholdView.vue`, `views/SettingsTasksView.vue` (a list-detail settings page), `views/HomeView.vue`, `apps/web/src/api/food.ts`.

## You own
The paths in `owns`. The scaffold (main session) already has the `/settings/meals` route, your stub view and the stub `stores/mealsSettings.ts` with the contract names; replace the stubs. Don't edit `router/index.ts`, `api/food.ts` or `ui/icons.ts`.

## Work
1. **`stores/mealsSettings.ts` first** (T-07 and T-08 read it in the same wave; keep the scaffold's exported names). It exports `useMealsSettingsStore()` with `settings`, `slots`, `visibleSlots` (computed, in order), `aisles`, `stores`, `ingredients`, `load()` (all in parallel), `saveSettings(patch)`, `saveSlots(list)`, `saveAisles(list)`, `saveStores(list)`, `storeAisles(id)`, `saveStoreAisles(id, list)`, `ingredient` CRUD and `merge(id, into)`, and `invalidate(kind)` (on `household.updated` and `resync`). Every member may read it; writes are adult only.
2. **`settingsNav.ts`:** a Household-group page `{to: '/settings/meals', icon: 'food', label: 'Meals', blurb: 'Meal slots, stores, aisles and ingredients', adult: true}`.
3. **`SettingsMealsView.vue`**, with cards in `views/parts/meals/`:
   - **Slots:** name, time (`"HH:MM"` input), a Shown switch (instant, D-096), reorder, add, rename; removing a used slot shows the server's `slot_in_use` message.
   - **Plan horizon:** weeks, 1 to 52 (instant).
   - **Stores:** add, rename, reorder, delete with confirm ("items move to Any store"). Per store, an **aisle order** sheet (`GET/PUT /meals/stores/{id}/aisles`): reorder, and a switch to hide an aisle the store does not have (D-121).
   - **Aisles:** rename, add, delete, and the household order (used for "Any store" and new stores).
   - **Ingredients:** search the catalogue (about 300 rows, client-side filter); a row has the staple switch (instant); the detail sheet edits canonical name, aliases (chips), aisle, default store and default unit; New ingredient; **Merge into…** with confirm.
   - **Matching:** `ingredient_llm` switch, `trigram_threshold` slider (0.2 to 0.95), the descriptor words as chips, and `grocery_archive_hours` (1 to 720).
4. **`components/food/TodayMeals.vue`** and **`HomeView.vue`:** a "Today's meals" card listing today's entries in visible slots (from `GET /meal-plan?from=today&to=today`, household tz), linking to `/food/plan`. It is hidden when there are none.
5. **Tests** (`views/settingsMeals.test.ts`):
   - `visibleSlots` order;
   - the slot time validation (`"HH:MM"`);
   - the staple switch saves instantly with Undo;
   - merge confirms;
   - a child cannot reach the page;
   - the Today card hides when empty.

## Acceptance
- [ ] In `apps/web`: `lint`, `typecheck`, `test` and `build` pass, then `format`.
- [ ] The report lists the exact store API (T-07, T-08 and T-11 use it), "list what outside your paths needs changing", and README changes needed: none.
