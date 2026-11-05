---
id: T-18
title: "Shell changes from review R2: hamburger drawer on phones, collapsible desktop drawer, sticky save bar"
milestone: M4
wave: 3
depends_on: [T-05]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["apps/web/src/ui/UiNavBar.vue", "apps/web/src/ui/UiNavRail.vue", "apps/web/src/ui/UiNavDrawer.vue", "apps/web/src/ui/UiTopAppBar.vue", "apps/web/src/ui/UiSaveBar.vue", "apps/web/src/ui/nav.ts", "apps/web/src/ui/navItem.ts", "apps/web/src/ui/composables/**", "apps/web/src/ui/UiGallery.vue", "apps/web/src/ui/components.test.ts", "apps/web/src/ui/composables.test.ts", "apps/web/src/app/**", "apps/web/src/layouts/AppLayout.vue", "apps/web/e2e/ui-shots.spec.ts"]
decisions: [D-095, D-096]
---

# T-18: Shell changes from review R2

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. T-05 built the app shell on the `ui/` design system. The user reviewed it at checkpoint R2 and asked for three changes. You make them. Wave-3 tasks are migrating views at the same time; they own the views and you own the shell.

## Read first
- `plan/_rules-common.md`, `plan/M4/_rules.md`
- `plan/M4.md`, *Navigation (T-05)* and *UI library*
- `arch/Frontend.md` §1.3 (navigation, updated for R2) and §1.4 (applying changes)
- T-05's brief and its `## Outcome` in `plan/M4/T-05-app-shell.md`
- `apps/web/src/layouts/AppLayout.vue`, `apps/web/src/app/navigation.ts`, `apps/web/src/ui/{UiNavBar,UiNavRail,UiNavDrawer,UiTopAppBar,UiSaveBar}.vue`, `apps/web/src/ui/nav.ts`

## You own
The paths in `owns` above. **Not yours:** every file in `apps/web/src/views/**`, the stores, and the other `ui/` components. If a view needs to change, report it.

## Work
1. **Compact (< 600 px): no bottom bar.**
   - `UiTopAppBar` shows a **menu button** (`menu` icon, `aria-label="Open navigation"`) at its start on compact, unless the page passes a back target (nested pages keep the back arrow).
   - It learns that it is inside the app shell through `provide`/`inject`: `AppLayout` provides `{ openNav() }` under an exported `InjectionKey` in `ui/nav.ts`. Outside the shell (setup, bare, kiosk) nothing changes.
   - The menu opens a **modal `UiNavDrawer`** (scrim, focus trap, Esc and scrim-click close, closes on navigation) with every destination, grouped as on expanded, plus Log out at its foot.
   - Remove the bottom bar and the More sheet from the compact layout. Reset `--ui-bottom-offset` to 0 on compact, since the snackbar and Save bar no longer sit above a bar.
   - The top bar's `#actions` slot is where pages put contextual actions. Make sure it fits at least two icon buttons plus an overflow `UiMenu` at 360 px, and the title truncates with an ellipsis.
   - Keep `UiNavBar` in the library (the gallery shows it), but the shell no longer uses it.
2. **Medium (600–839 px):** the rail, unchanged.
3. **Expanded (≥ 840 px): collapsible drawer.**
   - A toggle at the top of the standard drawer (`menu_open` / `menu` icon, `aria-label` "Collapse navigation" / "Expand navigation", `aria-expanded`) switches between the drawer and the rail.
   - The choice is stored per device in `localStorage` key `fa.nav.collapsed` (`"1"` or absent), read and written through `app/navPrefs.ts` (`getNavCollapsed()`, `setNavCollapsed()`) with try/catch.
   - The content area takes the freed width, with no layout jump on load: read the preference before the first render.
4. **Save bar sticky on every window class:**
   - `UiSaveBar` is `position: sticky; bottom: 0` (plus `env(safe-area-inset-bottom)`) on all widths, spanning the content column, with its own surface (`--ui-surface-container-high`) and `--ui-elev-2`.
   - Drop any "top of the card on wide screens" behaviour.
   - Because it is a library change, every form that uses it gets this.
5. **Gallery and tests:**
   - The gallery shows the top bar with the menu button and the collapsed and expanded drawer.
   - Unit tests: the menu button appears only on compact inside the shell; the drawer opens and closes and closes on navigation; the collapse preference round-trips through `localStorage` and survives a throwing `localStorage`; the Save bar class is sticky at all three widths.
   - Update `app/AppLayout.test.ts`.
6. **Screenshots:**
   - In `e2e/ui-shots.spec.ts`, replace the shell's "More sheet" shot with `shell-nav-drawer` (360, the menu opened) and add `shell-rail-collapsed` (1280, the drawer collapsed: click the toggle).
   - Keep the per-area route files as they are (they are the wave-3 tasks').

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] The new unit tests from Work 5 exist and pass
- [ ] `scripts/ui-shots.sh --gallery` passes. Do not run `--all`: the main session runs it after wave 3. List the PNG paths.
- [ ] e2e: the specs use `getByRole('navigation', { name: 'Primary' })` and links inside it. At 1280 px the drawer must keep that accessible name. Grep `apps/web/e2e/*.spec.ts` for nav selectors and keep every one of them working at the viewport the specs use. Report any you could not keep.
- [ ] Report (`plan/_rules-common.md` format), including: what a view must do to put actions in the top bar, and anything outside your paths that needs changing
