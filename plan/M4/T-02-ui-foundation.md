---
id: T-02
title: "UI foundation: tokens, icons, components, composables, gallery, screenshot script"
milestone: M4
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/ui/**", "apps/web/src/main.ts (the tokens import line only)", "apps/web/src/router/index.ts (the DEV-only /ui-gallery block only)", "apps/web/package.json", "apps/web/pnpm-lock.yaml", "apps/web/vite.config.ts", "apps/web/env.d.ts", "apps/web/playwright.config.ts", "apps/web/e2e/ui-shots.spec.ts", "scripts/ui-shots.sh"]
decisions: [D-095, D-096, D-097, D-079, D-081]
---

# T-02: UI foundation: tokens, icons, components, composables, gallery, screenshot script

You are a sub-agent implementing one task of milestone M4 (UI revamp and LLM extraction slice) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue 3 frontend in `apps/web`. You build the design system that every later UI task uses: a home-grown component library modelled on Material 3 and driven by the existing theme variables. You do not migrate any view. The user reviews your gallery screenshots at checkpoint R1 before the views are migrated, so the gallery must show everything clearly.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`
- `plan/M4.md`: *UI library*, *Review checkpoints*, *Disk*
- `decisions/D-095-ui-component-library.md`, `decisions/D-096-apply-changes-model.md`
- `arch/Frontend.md` §1.2 to §1.4 (design system, navigation, applying changes), §3d (themes and their CSS variables) and §6 (accessibility)
- `research/R-005-material-symbols-vue.md` (icon packages, the `fill: currentColor` note, and the unverified Vite 8 compatibility)
- `apps/web/src/styles/base.css`, `apps/web/src/themes/` (`index.ts`, `types.ts`, `families/`), `apps/web/src/components/` (what exists today: `Tabs`, `StateBlock`, `ErrorBanner`, `PageHeader`, `tasks/UndoToast.vue`), `apps/web/vite.config.ts`, `apps/web/playwright.config.ts`, `scripts/e2e.sh`

## You own
The paths in `owns` above. In `main.ts` you add only `import './ui/tokens.css'` after the `base.css` import. In `router/index.ts` you add only a route block for `/ui-gallery` guarded by `import.meta.env.DEV`. **Not yours:** every view, layout, store and existing component (T-05 and wave 3 migrate them), `apps/web/src/gen/**`, `dashboard/**`, `kiosk/**`. You build no Rust.

## Work
1. **Icons.**
   - Add `unplugin-icons` and `@iconify-json/material-symbols` as devDependencies with `corepack pnpm@9 add -D`. Configure `Icons({ compiler: 'vue3' })` in `vite.config.ts` and add the `unplugin-icons/types/vue` reference in `env.d.ts`.
   - **Verify by building** (`build`, `typecheck` and a vitest test that mounts an icon in jsdom). If the plugin fails with Vite 8, vue-tsc or vitest, switch to `@material-symbols/svg-400` with `vite-svg-loader` (R-005) behind the same `icons.ts`, and report it.
   - Create `ui/icons.ts`: a typed map from semantic name to component (rounded style). Include at least: `home`, `today`, `calendar`, `tasks`, `routines`, `inbox`, `mail`, `bills`, `settings`, `more`, `add`, `edit`, `delete`, `back`, `close`, `check`, `undo`, `search`, `filter`, `sort`, `expand`, `collapse`, `chevron_right`, `menu`, `person`, `group`, `devices`, `palette`, `notifications`, `weather`, `label`, `link`, `open_external`, `warning`, `error`, `info`, `success`, `ai`, `refresh`, `sync`, `lock`, `key`, `visibility`, `visibility_off`.
   - Export `type IconName = keyof typeof icons` and a `UiIcon` component (`name: IconName`, `size` 20/24 px default 24, `aria-hidden` unless `label`).
   - The SVGs get `fill: currentColor`.
2. **Tokens** in `ui/tokens.css`, exactly the names in *UI library* in `plan/M4.md`.
   - Every colour token is a `color-mix()` of the theme variables `applyTheme` writes (list them in a header comment), never a literal.
   - Check the result in at least one light and one dark variant of three families (e.g. Catppuccin, Nord, Gruvbox).
   - Add a vitest test that resolves each colour token for every theme variant (compute the `color-mix` in TS from the theme data or assert the token references only theme variables) and checks that `--ui-on-surface` on `--ui-surface` and on each container tone reaches 4.5:1 with the helpers in `src/lib/contrast.ts`.
   - Under `prefers-reduced-motion`, the motion durations are `0ms`.
3. **Components**: one `.vue` file each under `ui/`, exported from `ui/index.ts`, exactly the list in *UI library*.
   - Props, emits and slots are typed (`defineProps<…>()`), with `v-model` where it fits (`UiTextField`, `UiSelect`, `UiSwitch`, `UiCheckbox`, `UiRadioGroup`, `UiSegmented`, `UiDialog`/`UiSheet` open state).
   - Mobile-first CSS, scoped, using only `--ui-*` tokens.
   - Material 3 behaviour to follow:
     - state layers on hover, focus and press;
     - a visible focus ring;
     - touch targets of at least 48 px, including the icon button's hit area;
     - `UiCard` variants differ by surface and elevation, not by borders alone;
     - `UiListItem` with `to` renders a `RouterLink` and the whole row is the target;
     - `UiTextField` has the label, supporting text, error text (`aria-invalid` and `aria-describedby`), prefix and suffix;
     - `UiSwitch` is `role=switch`;
     - `UiSegmented` is a radio group;
     - `UiDialog` and `UiBottomSheet` trap focus, close on Escape and scrim click (configurable), and restore focus;
     - `UiSheet` picks the bottom sheet on compact and the dialog otherwise;
     - `UiSnackbarHost` uses `role=status` and shows one message at a time, queued;
     - `UiSaveBar` is sticky at the bottom on compact and inline at the top of its container otherwise;
     - `UiNavBar`, `UiNavRail` and `UiNavDrawer` take `destinations: NavDestination[]` (type in `ui/nav.ts`: `{ id, label, icon: IconName, to, badge?: number, group?: string }`) and mark the active route by path prefix.
4. **Composables** in `ui/composables/`: `useBreakpoint`, `useSnackbar` (a provide/inject service, with a module singleton as fallback so it works in tests), `useInstantSave` and `useFormDraft`, with the signatures in `plan/M4.md`.
   - `useFormDraft` deep-clones the source, compares structurally, resets when the source changes while not dirty, and installs `onBeforeRouteLeave` (with a confirm through `UiDialog` or `window.confirm` as fallback) plus a `beforeunload` handler while dirty.
   - `useInstantSave` debounces nothing. It makes an optimistic set, awaits the save, then shows a snackbar with Undo. Undo saves the old value. An error reverts and shows the error message.
5. **Gallery** `ui/UiGallery.vue`, a DEV-only route at `/ui-gallery`.
   - A section per component showing every variant and state: default, hover is not needed, focus is shown via a "show focus" toggle that adds a class, disabled, error, loading, empty, long text and an icon.
   - It has a live demo of `useInstantSave` (switch with Undo, a failing save that reverts) and of `useFormDraft` with `UiSaveBar`, and a navigation demo that switches between bar, rail and drawer.
   - A theme switcher at the top (family, variant, mode) uses `applyTheme` from `src/themes`; the theme can also be set from the query (`?theme=<family>&variant=&mode=`) so the screenshot script can drive it.
   - Sample data is fictional.
6. **Screenshots.**
   - `apps/web/e2e/ui-shots.spec.ts` is tagged `@shots`. It visits `/ui-gallery` (and with `--all`, a list of app routes read from a constant at the top of the spec, signing in like the existing specs) at widths 360, 800 and 1280 and in the default light and dark variants, and writes full-page PNGs to `apps/web/test-results/ui-shots/<page>-<width>-<light|dark>.png`.
   - Exclude `@shots` from the default run with `grepInvert` in `playwright.config.ts`.
   - `scripts/ui-shots.sh [--gallery|--all]`: `--gallery` (default) starts the Vite dev server on a free port, runs the spec with `--grep @shots` and stops the server. `--all` reuses the start-up of `scripts/e2e.sh` (source or call its functions; do not copy it) and runs against that stack. The script prints the output directory and the file count.
7. **Tests** (vitest, jsdom), in `ui/*.test.ts`:
   - token contrast;
   - `useInstantSave` (success, Undo restores, failure reverts);
   - `useFormDraft` (dirty, discard, save, source reset);
   - `UiSwitch` and `UiSegmented` keyboard and ARIA;
   - `UiDialog` focus trap and Escape;
   - nav components mark the active destination by prefix;
   - `UiIcon` renders an SVG.

## Acceptance
- [ ] In `apps/web`: `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass; `corepack pnpm@9 run format` was run
- [ ] `build` output shows the icons are tree-shaken (quote the size of the main chunk before and after from the build output)
- [ ] `scripts/ui-shots.sh --gallery` wrote the PNGs (quote the count from its output); list the files
- [ ] the default e2e run does not pick up `ui-shots.spec.ts` (`corepack pnpm@9 exec playwright test --list` output)
- [ ] report:
  - the files built;
  - the icon route taken (unplugin or fallback) and why;
  - the theme variables the tokens use;
  - any family whose tones looked poor, with the fix;
  - **questions for the user's R1 review** (density, corner radius, card style, icon weight);
  - "list what outside your paths needs changing"

## Outcome (T-02 report)
- **Built:**
  - the `ui/` library: tokens, 31 components plus the internal `UiOverlay`, `icons.ts` with 44 semantic names, and the composables `useBreakpoint`, `useSnackbar`, `useInstantSave`, `useFormDraft` and `useFocusTrap`;
  - the dev-only `/ui-gallery`;
  - `scripts/ui-shots.sh` and `e2e/ui-shots.spec.ts` (`@shots`, enabled by `UI_SHOTS`).
- **Icons:** `unplugin-icons` 24.0.0 with `@iconify-json/material-symbols` works with Vite 8, vue-tsc and vitest.
- **Contrast:** a test checks the tokens in all 36 theme variants.
- **Deviations:**
  - The gallery sits in the setup layout, so the 1280 px shot is narrow; a bare layout is a follow-up.
  - `useFormDraft` confirms with `window.confirm`.
  - `UiTextField` puts its label above the field.
- **Follow-ups:**
  - Theme data: the accent and on-accent pair is below AA in Solarized light, Tokyo Night day and Rose Pine dawn, and Seoul256's thin variants reach only 4:1.
  - README: `ui-shots.sh` (for T-17).
  - The `--all` shots have not run yet.
