---
id: T-05
title: "Palette data: eight theme families"
milestone: M3.6
wave: 2
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["apps/web/src/themes/families/**", "research/**"]
decisions: [D-079]
---

# T-05: Palette data: eight theme families

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Web theme contract* (`ThemeFamily`, the token set, the contrast rules)
- `apps/web/src/themes/{types,index}.ts`, the stub `themes/families/index.ts`, `themes/themes.test.ts` (from T-04)
- `research/_index.md` (the next `R-NNN`) and one existing `R-*.md` for the format

## You own
- `apps/web/src/themes/families/**`
- `research/R-NNN-theme-palettes.md` and its line in `research/_index.md`

## Work
1. **Research from the official sources** (use WebFetch or `gh`/git on each project's repository; never from memory): Catppuccin (Latte, Frappé, Macchiato, Mocha), Nord, Dracula, Gruvbox (light and dark), Solarized (light and dark), Tokyo Night (the variants the project publishes), Rosé Pine (main, moon, dawn), Kanagawa (Wave, Dragon, Lotus). Record in the research note: for each family, the source URL, the licence, the variants found, and the exact colour names used for each token. Mark a variant you could not verify as missing and leave it out.
2. One file per family in `themes/families/<id>.ts` (default export a `ThemeFamily`), plus `themes/families/index.ts` exporting `FAMILIES`. Replace the stub; keep Catppuccin's Latte and Mocha values identical to the stub.
3. **Mapping** (write it in the research note and as a comment atop each file): `bg` is the palette's base background; `surface` and `surface2` the next two levels away from it; `border` a subtle line colour; `text` the main foreground; `muted` the secondary foreground; `onAccent` the colour that reads on the blue hue. The eight hues come from the palette's named colours; where one is missing (Nord has no pink, Gruvbox no cyan name, and so on) use the nearest published colour and comment it.
4. **Make the contrast test pass** (`themes/themes.test.ts`). If a published value fails a rule, adjust the smallest thing that fixes it, in the data (for example the `muted` shade), and note each adjustment with its original value in a comment and in the research note. Never relax the test.
5. A family with a single published variant is fine: it just has one entry (the app always uses it, D-079).

## Acceptance
- [ ] `corepack pnpm@9 test` passes, including the contrast and structure tests over all families; `typecheck` and `lint` pass
- [ ] `python3 tools/fa.py check` passes (research front matter: `id, title, date, method, verified, informs: []`; `informs: [D-079]`)
- [ ] report: the final family and variant list, every adjusted value, any variant left out and why

## Outcome (T-05 report)
- Eight families in `apps/web/src/themes/families/`, sources and licences in `research/R-002-theme-palettes.md` (every adjusted value listed with its original). Variants: Catppuccin (Latte, Mocha, Frappé, Macchiato), Nord (dark only), Dracula (Dracula, Alucard), Gruvbox (Light, Dark; hard and soft contrast left out), Solarized (Light, Dark), Tokyo Night (Day, Night, Storm, Moon), Rosé Pine (Dawn, Main, Moon), Kanagawa (Lotus, Wave, Dragon).
- Several hues repeat where a palette lacks them (Dracula blue = purple, Nord pink = purple, Rosé Pine pink = red and blue = cyan, Solarized surface2 = surface); noted in the files.
- The contrast test is unchanged; failing values were adjusted in the data.
