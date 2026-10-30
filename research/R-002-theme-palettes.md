---
id: R-002
title: Theme palettes (eight families) for M3.6
date: 2026-10-29
method: Sonnet 5.5 sub-agent (medium effort); shallow git clones of each project's repository, values read from the source files (no memory); Dracula spec page fetched from draculatheme.com/spec; contrast checked with the app's own formula
verified: every colour below was read from the cloned sources on 2026-10-29. Gruvbox ships no LICENSE file in the clone (MIT comes from package.json). Rose Pine's dist/json dawn text (#464261) disagrees with its source/index.ts (#575279); the source value is used
informs: [D-079]
---

# R-002: Theme palettes

Data lives in `apps/web/src/themes/families/<id>.ts`. Each file repeats the mapping and marks derived values.

## Mapping (all families)
`bg` = base background; `surface`, `surface2` = next two levels away from it; `border` = subtle line; `text` = main foreground; `muted` = secondary foreground; `onAccent` = colour that reads on the blue hue; eight hues from the palette's named colours (nearest published colour when one is missing).

## Families
| Family | Source | Licence | Variants used | Not used |
|---|---|---|---|---|
| Catppuccin | github.com/catppuccin/palette, `palette.json` | MIT | Latte; Mocha, Frappé, Macchiato | none |
| Nord | github.com/nordtheme/nord, `src/nord.css` | MIT | Nord (one variant) | none published |
| Dracula | draculatheme.com/spec, github.com/dracula/dracula-theme README | MIT | Dracula; Alucard (light) | none |
| Gruvbox | github.com/morhetz/gruvbox, `colors/gruvbox.vim` | MIT (package.json) | Light, Dark (medium contrast) | hard and soft contrast variants (same hues, only the bg0 differs) |
| Solarized | github.com/altercation/solarized, README | MIT | Light, Dark | none |
| Tokyo Night | github.com/folke/tokyonight.nvim, `extras/lua/tokyonight_*.lua` | Apache-2.0 | Day (light); Night, Storm, Moon | none |
| Rosé Pine | github.com/rose-pine/palette, `source/index.ts` | MIT | Dawn (light); Main, Moon | none |
| Kanagawa | github.com/rebelot/kanagawa.nvim, `lua/kanagawa/colors.lua`, `themes.lua` | MIT | Lotus (light); Wave, Dragon | none |

Variant order puts the default of each mode first (Catppuccin: Latte, Mocha; Tokyo Night: Day, Night; Rosé Pine: Dawn, Main; Kanagawa: Lotus, Wave; Gruvbox and Solarized: Light, Dark).

## Colour names per token
- **Catppuccin:** bg base, surface mantle, surface2 crust, border surface1, muted subtext1 (Latte) or subtext0 (dark), text text, onAccent crust (dark) or white (Latte, not in palette); hues red, peach, yellow, green, teal, blue, mauve, pink.
- **Nord:** bg nord0, surface nord1, surface2 nord2, border nord3, muted nord4, text nord6, onAccent nord0; hues nord11, nord12, nord13, nord14, cyan nord8, blue nord9, purple nord15, pink nord15 (no pink in Nord).
- **Dracula / Alucard:** bg Background; surface and surface2 Background Dark and Darker (spec; Alucard uses Background Lighter and Light); border Current Line (Dracula) or Background Darker (Alucard); muted and text Comment and Foreground; onAccent Background; blue and purple both the spec's AnsiBlue (Dracula has no blue); other hues by name.
- **Gruvbox:** dark: bg dark0, surface dark1, surface2 dark2, border dark3, muted light2, text light1, onAccent dark0; hues bright_* (cyan bright_aqua, purple neutral_purple, pink bright_purple). Light: mirrored with light0..3, muted dark2, text dark1, hues faded_* (pink neutral_purple).
- **Solarized:** only two background tones, so surface2 repeats surface (base02 or base2). Border base01 (dark) or base1 (light); hues yellow, orange, red, green, cyan, blue, violet, magenta (as pink).
- **Tokyo Night:** bg bg, surface bg_dark, surface2 bg_dark1, border fg_gutter (Day: border), muted fg_dark, text fg; hues red, orange, yellow, green, cyan, blue, purple = magenta, pink = magenta2 (Moon: its own `purple`, a pink-violet).
- **Rosé Pine:** bg base, surface surface, surface2 overlay, border highlightMed, muted subtle, text text, onAccent base; red love, orange rose (no orange in the palette), yellow gold, green pine, cyan foam, blue foam (the project's ANSI blue; same as cyan), purple iris, pink love (same as red). Rosé Pine names only six hues, so two pairs repeat.
- **Kanagawa:** bg sumiInk3 / lotusWhite3 / dragonBlack3, surface and surface2 the next two of the same ramp, border sumiInk6 / lotusViolet1 / dragonBlack6, muted oldWhite / lotusGray2 / dragonGray, text fujiWhite / lotusInk1 / dragonWhite; hues waveRed, surimiOrange, carpYellow, springGreen, waveAqua2, crystalBlue, oniViolet, sakuraPink (Lotus and Dragon use their own lotus* and dragon* equivalents; Lotus yellow is lotusYellow2, Dragon purple is dragonViolet, Dragon blue is dragonBlue2).

## Adjusted values (contrast rules)
Rules: text and muted at least 4.5:1 on bg, surface, surface2; hues at least 3:1 on bg; onAccent at least 4.5:1 on blue. Swaps to another published colour (not in the table below): Gruvbox muted (dark3 to dark2 on light; light3 to light2 on dark), Solarized Light text (base01 to base02), Solarized Dark muted and text (base0 to base1, base1 to base2). The rest were mixed toward black or white in 1% steps until the rule passed (the "Ratio before" column is the worst case over the surfaces or the bg). Catppuccin Latte hues are the T-04 stub values.

| Family | Variant | Token | Published / mapped | Used | Ratio before |
|---|---|---|---|---|---|
| catppuccin | latte | orange | `#fe640b` | `#c2410c` | 2.64 |
| catppuccin | latte | yellow | `#df8e1d` | `#8f5a0a` | 2.31 |
| catppuccin | latte | green | `#40a02b` | `#2f7d1f` | 2.96 |
| catppuccin | latte | pink | `#ea76cb` | `#b8449a` | 2.34 |
| dracula | dracula | muted | `#6272a4` | `#8390b7` | 3.03 |
| dracula | alucard | muted | `#6c664b` | `#676147` | 4.19 |
| solarized | light | muted | `#586e75` | `#566c73` | 4.39 |
| solarized | light | yellow | `#b58900` | `#b38800` | 2.98 |
| solarized | light | green | `#859900` | `#849700` | 2.97 |
| solarized | light | cyan | `#2aa198` | `#299e95` | 2.93 |
| solarized | light | onAccent | `#fdf6e3` | `#1e1e1b` | 3.41 |
| solarized | dark | onAccent | `#002b36` | `#00222b` | 4.08 |
| tokyo-night | day | text | `#3760bf` | `#2e51a0` | 3.54 |
| tokyo-night | day | muted | `#6172b0` | `#475380` | 2.79 |
| tokyo-night | day | onAccent | `#ffffff` | `#141414` | 4.02 |
| rose-pine | dawn | muted | `#797593` | `#6a6781` | 3.67 |
| rose-pine | dawn | orange | `#d7827e` | `#c67874` | 2.60 |
| rose-pine | dawn | yellow | `#ea9d34` | `#be7f2a` | 2.05 |
| rose-pine | dawn | onAccent | `#faf4ed` | `#232221` | 3.14 |
| rose-pine | moon | muted | `#908caa` | `#a3a0b8` | 3.61 |
| kanagawa | lotus | muted | `#716e61` | `#5f5c51` | 3.45 |
