---
id: T-06
title: "Web: Appearance page and kiosk theme control"
milestone: M3.6
wave: 2
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/views/SettingsAppearanceView.vue", "apps/web/src/views/SettingsLayout.vue", "apps/web/src/views/SettingsIndexView.vue", "apps/web/src/router/**", "apps/web/src/dashboard/DashboardCanvas.vue", "apps/web/src/dashboard/LayoutEditor.vue", "apps/web/src/dashboard/editor.ts", "apps/web/src/dashboard/layout/**", "apps/web/src/dashboard/strings.ts", "apps/web/src/views/appearance.test.ts"]
decisions: [D-079, D-081]
---

# T-06: Web: Appearance page and kiosk theme control

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Web theme contract* and the *Backend wire formats* for `ThemeChoice`
- `arch/Frontend.md` §3c, §3d, §4.1; `decisions/D-081-theme-per-device.md`
- the T-04 exports in `apps/web/src/themes/index.ts`; the existing settings views and router for the guard and nav style

## You own
The paths in the front matter. Do not edit `themes/**` (T-04, T-05) or `gen/**`.

## Work
1. **Appearance page** `/settings/appearance` (guard `user`, so children too), linked from the Personal group in `SettingsLayout` and `SettingsIndexView`:
   - a card per family with a swatch strip (neutrals and the eight hues) of its default variant;
   - a **mode** control (auto, light, dark), hidden for a family with a single variant, with a short note that "this theme only has a dark/light variant";
   - a **variant** select, shown only when the family has more than one variant in the effective mode (Catppuccin dark);
   - changes apply immediately through `useDeviceTheme()` and persist (`fa.theme`);
   - a line saying the choice is for this device only. Strings go where the other settings strings live.
2. **Kiosk layout:** `dashboard/layout/index.ts` and `editor.ts`: `DashboardSettings.theme` is a `ThemeChoice`. Parsing a saved layout accepts the legacy string (`"dark"` becomes `{family: 'catppuccin', mode: 'dark'}`) and always returns the object; presets keep the theme. Unknown family ids are kept as they are (the renderer falls back).
3. **LayoutEditor:** replace the auto/light/dark select with family, variant (if more than one in the mode) and mode controls, reusing the same rules as the Appearance page; the live preview follows.
4. **DashboardCanvas:** resolve the variant with `resolveVariant(choice, {prefersDark, night})`, where `night` comes from `resolveMode` and the sunrise/sunset in the snapshot, and apply it with `applyTheme` on the canvas root (scoped, so the editor preview can differ from the page). On the real `/dashboard` route also apply it to `document.documentElement`, so the page background matches. Replace the canvas's hard-coded `.canvas[data-theme=...]` token blocks. The burn-in shift applies when the resolved variant's mode is dark.
5. Tests: the page (mode hidden for a single-variant family, variant select only when relevant, choice saved), the layout parse (legacy string, object, unknown family kept), the canvas (a dark choice at noon, burn-in on dark only).

## Acceptance
- [ ] `corepack pnpm@9 run typecheck`, `run lint`, `test` pass; counts quoted only from the output
- [ ] no literal colour left in `DashboardCanvas.vue` (grep for `#`)
- [ ] report: anything in T-04's engine you needed that was missing

## Notes from T-04
`useDeviceTheme(enabled)` skips the kiosk shell, so the dashboard must call `applyTheme` itself. Switch `DashboardCanvas.vue` from the deprecated `resolveTheme` alias to `resolveMode` and delete the alias in `dashboard/theme.ts` (you may edit that one line, T-04 is done).

## Outcome (T-06 report)
- Appearance page at `/settings/appearance`, shared `themeControls` rules in `dashboard/editor.ts` used by the page and the LayoutEditor, `normalizeTheme`/`parseSettings` for the legacy string, DashboardCanvas applying the variant on its own root (and on `<html>` for the real `/dashboard` via a `page` prop).
- Edits outside the owned paths: `views/settingsNav.ts`, `views/DashboardView.vue`, `dashboard/dashboard.test.ts`.
- `gen/api.d.ts` still types the device theme as a string until regenerated (one `as never` in `LayoutEditor.vue` to remove then).
- Main session fixed the two-instance problem: `useDeviceTheme` now shares one choice per page (`themes/index.ts`, plus `resetDeviceTheme` for tests).
