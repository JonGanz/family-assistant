---
id: T-08
title: "Shopping list, offline queue, service worker caching"
milestone: M5
wave: 4
depends_on: [T-06]
agent: {model: sonnet, effort: high}
status: todo
owns: ["apps/web/src/stores/grocery.ts", "apps/web/src/lib/offlineQueue.ts", "apps/web/src/lib/offlineQueue.test.ts", "apps/web/src/views/food/ShoppingListView.vue", "apps/web/src/components/food/grocery/**", "apps/web/src/views/food/grocery.test.ts", "apps/web/vite.config.ts", "apps/web/src/lib/pwa.ts"]
decisions: [D-021, D-015, D-019, D-110, D-115, D-118, D-072, D-096, D-105]
---

# T-08: Shopping list and offline use

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Vue 3 PWA that must also work over plain HTTP (no service worker there, D-072). M5 adds one living grocery list per household (D-021), grouped by store and then aisle (D-115). Anyone, including the kitchen kiosk without a PIN, adds and checks items, and only adults edit or delete (D-110). The list must keep working in a shop with poor signal (D-118). You build the Shopping list tab, its reusable component (which the kiosk reuses in wave 5), the offline queue and the HTTPS-only service worker caching.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: the *API* table (`/grocery*`, `/meals/changes`), *Web* (stores, shared components, e2e selectors) and *Domain types* (`GroceryView`, `GroceryItemView`, `StapleView`).
- `arch/Frontend.md` §1.1 (secure and insecure contexts, the offline queue) and §3f (Shopping list); `arch/Meals.md` §4.7.
- `apps/web/src/api/food.ts`, `apps/web/src/lib/ids.ts` (`newId()`), `apps/web/src/lib/pwa.ts`, `apps/web/vite.config.ts` (VitePWA, registered by hand only in a secure context), `stores/lists.ts`, `ui/UiFilterChips.vue`.

## You own
The paths in `owns`. The scaffold (main session) already has your route, stub view and stub store; replace the stubs. Don't edit `router/index.ts`, `api/food.ts` or `ui/icons.ts`.

## Work
1. **`lib/offlineQueue.ts`:** a small generic queue persisted in `localStorage` key `fa.grocery.queue` (every access wrapped in try/catch, with in-memory fallback). It handles:
   - `enqueue(op)`, where `op` is `{kind: 'add'|'check'|'uncheck'|'low', id, body?}`;
   - `flush(send)` in order, stopping at the first network error;
   - dropping an op on 404;
   - treating 409 on check as done (check-offs win);
   - `pending` (a ref with the count);
   - `online` from `navigator.onLine` plus `online`/`offline` events and failed requests.
2. **`stores/grocery.ts`:**
   - state `view` (GroceryView), `load()` and `invalidate(kind)` (on `grocery.updated` and `resync`);
   - `add(text, opts)` generates `newId()` client-side, updates optimistically and sends or queues;
   - `check`/`uncheck`/`low` are optimistic, then sent or queued;
   - `update`/`remove` are online only and adult only, with If-Match;
   - flush on reconnect;
   - `undo(changeId)`.
3. **`components/food/grocery/ShoppingList.vue`** (props `{canEdit, kiosk?}`):
   - the add box with catalogue suggestions (`GET /ingredients?q=`);
   - groups by store in order ("Any store" last, and aisles only when the household has no stores), then by aisle ("Other" last);
   - rows with text, `display` quantity and a source hint ("for Tue dinner");
   - check-off is instant with Undo, and checked items go to a collapsed "Checked" group;
   - the **Staples check** section with Low buttons;
   - a store filter (`UiFilterChips`, D-105, remembered in `fa.grocery.stores`);
   - the edit sheet and delete with Undo only when `canEdit`;
   - the "Offline, N changes waiting" note (`grocery-offline-note`);
   - with `kiosk`: larger targets, no edit sheet, and delete hidden unless the device is elevated (`canEdit` comes from the caller).
4. **`views/food/ShoppingListView.vue`:** `FoodTabs` plus `ShoppingList` with `canEdit` = adult.
5. **Service worker (secure context only):** extend the VitePWA config so the app shell is precached (if not already) and `GET /api/v1/grocery`, `/api/v1/meals/stores` and `/api/v1/meals/aisles` use a NetworkFirst runtime cache with a short timeout. Never cache other API routes or anything authenticated beyond these. Keep `lib/pwa.ts`'s rule that registration only happens in a secure context.
6. **Tests:**
   - `offlineQueue.test.ts`: order, persistence, 404 drop, 409 on check, the in-memory fallback when `localStorage` throws;
   - `grocery.test.ts`: grouping order; no stores means aisles only; the staples section; Low; a child gets no edit controls; queued adds show the offline note and flush.

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass, then `format`.
- [ ] The `build` output shows the service worker's runtime cache routes, and the plain-HTTP rule still holds: no registration code runs when `isSecureContext` is false.
- [ ] The report lists:
  - the `ShoppingList` props and events (T-11 reuses them);
  - the cached routes;
  - "list what outside your paths needs changing";
  - README changes needed: none.
