---
id: T-21
title: "Extended theme palettes from the Neovim ports"
milestone: M4
wave: 5
depends_on: [T-19]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/themes/**", "apps/web/src/ui/tokens.css", "apps/web/src/ui/tokens.test.ts", "apps/web/src/styles/base.css"]
decisions: [D-106]
---

# T-21: Extended theme palettes

You are a sub-agent on milestone M4 (UI revamp) of the Family Assistant project, a self-hosted family organiser with a Vue 3 frontend in `apps/web`. The user wants the app themes to carry the richer palettes of their Neovim ports, for better card backgrounds and text colours (D-106).

## Read first
- `plan/_rules-common.md`, `plan/M4/_rules.md`
- `decisions/D-106-neovim-theme-palettes.md`, `research/R-006-neovim-theme-palettes.md` (palettes, proposed mapping, contrast tables), `research/R-002-theme-palettes.md`
- `apps/web/src/themes/types.ts`, `themes/index.ts` (`applyTheme`), `themes/families/*.ts`, `themes/themes.test.ts`, `ui/tokens.css`, `ui/tokens.test.ts`
- `arch/Frontend.md` §1.2 (tokens) and §3d (themes)

## You own
The paths in `owns`. Do not touch views or other `ui/` files (T-20 runs in parallel).

## Work
1. **Types.** Extend `ThemeVariant` with an optional `extended`:
   ```ts
   {
     layers: [string, string, string, string]
     textStrong?: string
     textSubtle?: string
     comment?: string
     selection?: string
     highlight?: string
     accents?: Record<string, string>
   }
   ```
   `layers[0]` equals `neutrals.bg`.
2. **Data.** Fill `extended` for every variant R-006 covers, with values copied verbatim from R-006, and a comment naming the palette key on each line as the files do today. Leave out Alucard and Nord Snow Storm. Seoul256 gets none, but its fallback mixes shrink to 3/6/9 % (R-006 §Seoul256). Do not alter palette hex values for contrast. The published accents stay; the Q-092 revert is already in the tree.
3. **`applyTheme`:**
   - When `extended` is present, write `--color-layer-0..3`, `--color-text-strong`, `--color-text-subtle`, `--color-comment`, `--color-selection` and `--color-highlight`.
   - When it is absent, write derived values: layers as the current `color-mix` steps, and the text slots fall back to `text`/`muted`.
   - Then pick a **secondary-text token per layer**: `textSubtle` if it reaches 4.5:1 on that layer, else `muted` if it does, else `text`. Write it as `--color-on-layer-N-subtle`. Reuse the contrast helper the tests use (move it into `themes/` if needed).
4. **`ui/tokens.css`:**
   - Map the surface container tones onto the layers: page = 0, card = 1, raised = 2, dialog or sheet = 3. Keep the token names components use.
   - Secondary text on each container uses the matching `--color-on-layer-N-subtle`.
   - `comment` is used only for placeholder and disabled text.
   - Selection (`::selection`) and the list hover or highlight use the new slots.
5. **Tests:**
   - Every variant has a well-formed `extended` or none.
   - For every variant and layer, `text` and the chosen subtle token reach at least 4.5:1.
   - Keep the existing contrast tests. The accent-as-text failure in the three light themes is accepted (D-106); do not add a test that forbids it.

## Acceptance
- [ ] `corepack pnpm@9 run lint`, `typecheck`, `test`, `build` in `apps/web` pass (copy each summary line)
- [ ] Prettier is clean on touched files
- [ ] Never run `scripts/dev-run.sh`, never bind 8080/5173/18091, and run no state-changing git commands
- [ ] Report: files changed, any variant that needed a fallback, and "list what outside your paths needs changing"

## Outcome (T-21 report)
- `ThemeVariant.extended` (plus an optional `layerMix` for the fallback steps) is filled verbatim from R-006 for 20 variants. Alucard, Nord Snow Storm and Seoul256 have none; Seoul256 uses `layerMix: [3, 6, 9]`.
- `applyTheme` writes `--color-layer-0..3`, the text, selection and highlight slots, and `--color-on-layer-N-subtle` (the first of textSubtle, muted and text that reaches 4.5:1, via `lib/contrast.ts`).
- `tokens.css` maps the container tones onto the layers and adds `--ui-on-surface-variant-{low,high,highest}`, `--ui-on-surface-faint`, `--ui-selection` and `--ui-highlight`. `base.css` uses them for `::selection`, placeholders and disabled text.
- Deviation: where `text` itself is under 4.5:1 on a published layer (layer 3 of Frappé, Macchiato, Kanagawa Dragon, Solarized dark and Tokyo Night Day, plus some Seoul256 mixes), `resolveExtended` pulls that layer toward `bg` in 10 % steps until it passes. The theme data stays verbatim; only the applied CSS value moves. Recorded on D-106.
- The main session scoped `--ui-on-surface-variant` to the raised tones in `UiOverlay` (dialog and sheet), the `EventCard` hover and the `WeatherCard` strip.
- Web checks: lint 0 errors, typecheck, test (606 passed), build.
