---
id: R-005
title: Material Symbols icons in a Vue 3 + Vite app
date: 2026-11-04
method: Sonnet 5.5 sub-agent; `npm view` for versions, licences and dependencies; `npm pack` of `@material-symbols/svg-400` inspected; repo `apps/web/package.json` and `vite.config.ts` read. No icon was built into the app
verified: versions and licences from the npm registry on 2026-11-04; the SVG file content was read from the tarball. Import syntax and bundle effects are from package knowledge and not built: marked UNVERIFIED below
informs: [M4 plan]
---

# R-005: Material Symbols for Vue 3

## 1. Candidates (npm, 2026-11-04)
| Package | Version | Licence | Last modified | Unpacked | Notes |
|---|---|---|---|---|---|
| `@material-symbols/svg-400` | 0.48.3 | Apache-2.0 | 2026-11-04 | 13 MB | one SVG per icon, e.g. `rounded/home.svg`, `rounded/home-fill.svg`; repo marella/material-symbols. Weight variants are separate packages (svg-100 and so on) |
| `material-symbols` | 0.48.3 | Apache-2.0 | 2026-11-04 | 13 MB | same author; fonts and CSS, not suited to tree-shaking |
| `@iconify-json/material-symbols` | 1.2.97 | Apache-2.0 | 2026-10-28 | 8.9 MB | whole set as one JSON; names like `home-rounded`; a build-time source only |
| `unplugin-icons` | 24.0.0 | MIT | 2026-09-11 | 71 KB | deps: unplugin, local-pkg, @iconify/utils, obug, @antfu/install-pkg; peers are optional compilers (`@vue/compiler-sfc` for Vue 3, which `vue` already brings) |
| `vite-svg-loader` | 5.1.3 | MIT | 2026-09-01 | 7 KB | deps: svgo ^3.3.5, debug; peer `vue >=3.2.13` |
| `@iconify/vue` | 5.0.5 | MIT | 2026-11-01 | 182 KB | runtime component; fetches icon data from an API by default: not suitable (offline/local rule) |

The heavy packages are dev-time only: the bundle contains only the SVGs that are imported.

The raw SVG from `@material-symbols/svg-400` is `<svg xmlns=... width="48" height="48" viewBox="0 -960 960 960"><path d="..."/></svg>` with **no `fill` attribute**, so it renders black unless CSS sets `fill: currentColor` (and size should be set by CSS).

## 2. Usage
- `@material-symbols/svg-400` + `vite-svg-loader`: `import Home from '@material-symbols/svg-400/rounded/home.svg?component'`, then `<Home />`. Vite config: `plugins: [vue(), svgLoader()]`. Types: needs a `declare module '*.svg?component'` in `env.d.ts` (vite-svg-loader documents `vite-svg-loader` client types; exact line UNVERIFIED). Colour: add global CSS `svg { fill: currentColor }` (or per component). svgo runs at build.
- `unplugin-icons` + `@iconify-json/material-symbols`: `import Home from '~icons/material-symbols/home-rounded'`. Vite config: `Icons({ compiler: 'vue3' })`. Types: add `"types": ["unplugin-icons/types/vue"]` to the app tsconfig or a triple-slash reference. The generated component uses `fill="currentColor"` by default (Iconify convention; UNVERIFIED by build). Fill variants are `home-rounded-fill`.
- Both inline the SVG into the JS chunk of the importing component: bundle cost about 0.3 to 1 KB per icon (UNVERIFIED, not measured).
- A virtual module such as `~icons/...` is not resolved by vitest unless the plugin is in the vitest config. The repo's `vite.config.ts` uses `defineConfig` from `vitest/config` and one `plugins` array, so the same plugin applies to tests (jsdom). With `vite-svg-loader`, `?component` imports go through the same plugin, so likewise.

## 3. Repo facts
`apps/web`: Vite `^8.3.5`, `@vitejs/plugin-vue ^6.0.10`, `vitest ^5.0.5` (jsdom), `vue-tsc ^3.3.14`, `typescript ^6.0.3`. No icon package yet. Vite 8 support of `unplugin-icons` 24 and `vite-svg-loader` 5.1.3: UNVERIFIED (neither declares a vite peer range in the registry output; not built).

## 4. Recommendation
**`unplugin-icons` + `@iconify-json/material-symbols`** (two devDependencies): one import syntax, `currentColor` by default, per-icon tree-shaking, and plugin types. Alternative with a similar dep count is `@material-symbols/svg-400` + `vite-svg-loader`, which pulls svgo and needs a CSS fill rule; it is the better pick if the exact upstream file names must be kept. Either way, wrap icons in one `<Icon name>`-style map file (a single module that imports the used icons) so unused icons stay out of the bundle and the list can be audited. Verify with the four CI commands (`lint`, `typecheck`, `test`, `build`) in a spike before adopting.

## UNVERIFIED
Vite 8 compatibility; exact TypeScript type setup; `currentColor` default of the generated component; bundle size per icon; svgo effect on the `viewBox`.
