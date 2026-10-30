---
id: T-04
title: "Web: theme engine, tokens and app bootstrap"
milestone: M3.6
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/themes/**", "apps/web/src/styles/**", "apps/web/src/main.ts", "apps/web/src/App.vue", "apps/web/src/dashboard/theme.ts", "apps/web/src/lib/contrast*"]
decisions: [D-079, D-081]
---

# T-04: Web: theme engine, tokens and app bootstrap

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Web theme contract* (the types and exports are fixed there)
- `arch/Frontend.md` §3d; `decisions/D-079-theme-catalogue.md`, `D-081-theme-per-device.md`
- `apps/web/src/styles/base.css`, `apps/web/src/main.ts`, `apps/web/src/App.vue`, `apps/web/src/dashboard/theme.ts`, `apps/web/src/lib/contrast.ts`

## You own
- `apps/web/src/themes/**` (including the stub `themes/families/index.ts`, which T-05 takes over)
- `apps/web/src/styles/**`, `apps/web/src/main.ts`, `apps/web/src/App.vue`
- `apps/web/src/dashboard/theme.ts`, `apps/web/src/lib/contrast*`

## Work
1. `themes/types.ts`, `themes/index.ts` and the stub `themes/families/index.ts` exactly as in the contract. The stub holds Catppuccin Latte (light) and Mocha (dark) with the real published values (source URL and licence filled in).
2. `applyTheme`, `resolveVariant`, `loadDeviceChoice`, `saveDeviceChoice` (try/catch around storage, bad JSON falls back to `DEFAULT_CHOICE`) and `useDeviceTheme`.
3. `styles/base.css`: the tokens become the defaults for the shipped theme (Latte, and Mocha under `prefers-color-scheme: dark`), so the first paint is right before any script runs. Add the new tokens (`--color-surface-2`, `--hue-*`). Keep every existing token name working. Replace manual `[data-theme]` overrides if they conflict with the engine.
4. Bootstrap: `main.ts` or `App.vue` calls `useDeviceTheme()` for the app shell. The kiosk shell's theme comes from the device settings (T-06 wires it), so the bootstrap must not override it on `/dashboard`.
5. `dashboard/theme.ts`: rename `resolveTheme` to `resolveMode` and update its tests (keep the behaviour).
6. `themes/themes.test.ts`: structure (all tokens and all 8 hues present, `#rrggbb` only, ids are slugs and unique, each family has at least one variant) and the **contrast rules** from the plan over every family in `FAMILIES`. Tests for `resolveVariant` (single-mode family ignores the mode, `auto` with `night`, unknown family falls back), `loadDeviceChoice` (bad JSON, unavailable storage), and `applyTheme` (variables and `data-theme-mode`).

## Acceptance
- [ ] `corepack pnpm@9 run typecheck`, `run lint`, `test` pass; counts quoted only from the output
- [ ] with the stub families, the contrast test passes (adjust the stub if a published value fails, and say so in the report)
- [ ] report: the exact exports of `themes/index.ts`, and any contract deviation

## Outcome (T-04 report)
- Exports of `themes/index.ts`: `FAMILIES`, `HUES`, `DEFAULT_CHOICE`, `familyOf`, `resolveVariant`, `applyTheme`, `loadDeviceChoice`, `saveDeviceChoice`, `useDeviceTheme(enabled?: () => boolean)` returning `{ choice, setChoice }`, plus the types. `App.vue` enables it for every layout except the kiosk.
- `dashboard/theme.ts` exports `resolveMode`; a deprecated alias `resolveTheme` remains because `DashboardCanvas.vue` (T-06) still imports it. T-06 switches to `resolveMode` and deletes the alias.
- The Latte stub had four hues below the 3:1 rule (orange, yellow, pink, green); they were darkened and marked "derived". T-05 replaces the stub and applies the same test.
- Verified by the agent: typecheck, lint, 44 test files, 276 tests passing.
