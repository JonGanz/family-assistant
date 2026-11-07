---
id: R-006
title: Neovim theme palettes (richer neutrals for the app themes)
date: 2026-11-07
method: Sonnet 5.5 sub-agent; palette source files fetched raw from GitHub (raw.githubusercontent.com) on 2026-11-07 and read; commit = default-branch HEAD at fetch time (GitHub API, queried right after the fetches); contrast computed with the WCAG formula the app uses, against the app's current neutrals in apps/web/src/themes/families/*.ts
verified: true for every palette value in the tables (read from the source files named below). The slot mapping, the fallback mixes and the contrast ratios are proposals and computed results, not source values. Tokyo Night Day is read from the generated extras/lua/tokyonight_day.lua because colors/day.lua only inverts Night at runtime. Seoul256 values come from the xterm index table in the plugin (indices read from the Normal, CursorLine, Pmenu, Visual and Comment groups)
informs: [D-106]
---

# R-006: Neovim palettes for richer app neutrals

Goal: bolster the 7 neutrals of `ThemeVariant` (bg, surface, surface2, border, muted, text, onAccent) with the extra layers, foreground levels and selection colours that the Neovim ports publish.

## Sources read
| Family | Repo, file (all exist) | Commit | Licence |
|---|---|---|---|
| Catppuccin | catppuccin/nvim, `lua/catppuccin/palettes/{latte,frappe,macchiato,mocha}.lua` | edefef779ab0 | MIT |
| Dracula | Mofiqul/dracula.nvim, `lua/dracula/palette.lua` | ae752c13e95f | MIT |
| Gruvbox | ellisonleao/gruvbox.nvim, `lua/gruvbox.lua` (`Gruvbox.palette`, bg0..4 / fg0..4 maps) | 48262cda6a8a | MIT |
| Kanagawa | rebelot/kanagawa.nvim, `lua/kanagawa/colors.lua` (palette) and `themes.lua` (`ui` ramps `bg_m3..bg_p2`, `float`, `bg_visual`) | bb85e4bfc8d8 | MIT |
| Nord | gbprod/nord.nvim, `lua/nord/colors.lua` | 87394d4fc35c | WTFPL (the Nord palette itself is MIT) |
| Rosé Pine | rose-pine/neovim, `lua/rose-pine/palette.lua` | ff483051a47e | MIT |
| Seoul256 | junegunn/seoul256.vim, `colors/seoul256.vim`, `colors/seoul256-light.vim` | 0357ff3e44fa | MIT (stated in the file header; the GitHub licence API reports none) |
| Solarized | maxmx03/solarized.nvim, `lua/solarized/palette/init.lua` (dark), `palette/solarized-light.lua` | a8085e29883d | MIT |
| Tokyo Night | folke/tokyonight.nvim, `lua/tokyonight/colors/{storm,night,moon,day}.lua`; Day values from `extras/lua/tokyonight_day.lua` | cdc07ac78467 | Apache-2.0 |

Notes: `maxmx03/solarized.nvim` was chosen over `ishan9299/nvim-solarized-lua` (not read). Official Alucard: none. `Mofiqul/dracula.nvim` has no light variant (searched its tree). Two community ports exist (`jaljoue/dracula-alucard.nvim`, `delvedor/dracula-alucard.nvim`, seen in a GitHub search only, not read), so Alucard keeps the spec values from R-002. `gbprod/nord.nvim` has no light variant, so Nord Snow Storm has no Neovim source. Gruvbox ships hard, medium and soft; the app uses medium (`dark0`, `light0`).

## Proposed new optional slots
All optional; each falls back when absent. Layer order is page first, then steps away from the page in the palette's own direction (dark: lighter; light: darker, except Rosé Pine Dawn, whose `surface` is lighter first).

| Slot | Meaning | Fallback (variants with no richer palette) |
|---|---|---|
| `layers: [L0, L1, L2, L3]` | page, card, raised card, dialog or float. L0 equals `bg` | `[bg, mix(bg,text,5%), mix 10%, mix 15%)]` (use 3/6/9% for Seoul256, see contrast) |
| `textStrong` | headings, brightest foreground | `text` |
| `textSubtle` | secondary text, replaces `muted` where present | `muted` |
| `comment` | placeholder, disabled, timestamps; decorative, not for body text | `muted` |
| `selection` | selected row, text selection (Neovim Visual) | `mix(bg, blue, 25%)` |
| `highlight` | hover row, cursor line | `mix(bg,text,6%)` |
| `accents?` | extra named hues (teal, sky, maroon, etc.), optional | none; the 8 hues stay |

Justification: layers give cards real depth (the current `surface` and `surface2` are two steps, and in dark themes go darker than the page); the three text levels cover heading, body and meta; selection and highlight give list states without alpha tricks.

## Palettes and mapping (hex read from source)
Columns: L0..L3 = layers, S/H = selection/highlight, Strong/Subtle/Comment = text slots. Source names in brackets.

| Variant | L0 | L1 | L2 | L3 | S | H | Strong | Subtle | Comment |
|---|---|---|---|---|---|---|---|---|---|
| Catppuccin Latte | #eff1f5 base | #e6e9ef mantle | #dce0e8 crust | #ccd0da surface0 | #bcc0cc surface1 | #ccd0da surface0 | #4c4f69 text | #6c6f85 subtext0 | #8c8fa1 overlay1 |
| Catppuccin Frappé | #303446 base | #414559 surface0 | #51576d surface1 | #626880 surface2 | #626880 surface2 | #414559 surface0 | #c6d0f5 text | #a5adce subtext0 | #838ba7 overlay1 |
| Catppuccin Macchiato | #24273a base | #363a4f surface0 | #494d64 surface1 | #5b6078 surface2 | #5b6078 surface2 | #363a4f surface0 | #cad3f5 text | #a5adcb subtext0 | #8087a2 overlay1 |
| Catppuccin Mocha | #1e1e2e base | #313244 surface0 | #45475a surface1 | #585b70 surface2 | #585b70 surface2 | #313244 surface0 | #cdd6f4 text | #a6adc8 subtext0 | #7f849c overlay1 |
| Dracula | #282A36 bg | #3E4452 visual | #44475A selection | #4B5263 gutter_fg | #44475A selection | #3E4452 visual | #FFFFFF bright_white | #ABB2BF white | #6272A4 comment |
| Dracula Alucard | none, fallback (spec values in R-002) | | | | | | | | |
| Gruvbox light | #fbf1c7 bg0 | #ebdbb2 bg1 | #d5c4a1 bg2 | #bdae93 bg3 | #d5c4a1 bg2 | #ebdbb2 bg1 | #282828 fg0 | #504945 fg2 | #7c6f64 fg4 |
| Gruvbox dark | #282828 bg0 | #3c3836 bg1 | #504945 bg2 | #665c54 bg3 | #504945 bg2 | #3c3836 bg1 | #fbf1c7 fg0 | #d5c4a1 fg2 | #928374 gray |
| Kanagawa Lotus | #f2ecbc lotusWhite3 | #e5ddb0 lotusWhite2 | #dcd5ac lotusWhite1 | #d5cea3 lotusWhite0 | #c9cbd1 lotusViolet3 | #e7dba0 lotusWhite4 | #43436c lotusInk2 | #716e61 lotusGray2 | #8a8980 lotusGray3 |
| Kanagawa Wave | #1F1F28 sumiInk3 | #2A2A37 sumiInk4 | #363646 sumiInk5 | #54546D sumiInk6 | #223249 waveBlue1 | #2A2A37 sumiInk4 | #DCD7BA fujiWhite | #C8C093 oldWhite | #727169 fujiGray |
| Kanagawa Dragon | #181616 dragonBlack3 | #282727 dragonBlack4 | #393836 dragonBlack5 | #625e5a dragonBlack6 | #223249 waveBlue1 | #282727 dragonBlack4 | #c5c9c5 dragonWhite | #a6a69c dragonGray | #7a8382 dragonGray3 |
| Nord | #2E3440 nord0 | #3B4252 nord1 | #434C5E nord2 | #4C566A nord3 | #434C5E nord2 | #3B4252 nord1 | #eceff4 nord6 | #D8DEE9 nord4 | #616E88 (nord.nvim "light", out of palette) |
| Nord Snow Storm | none, fallback | | | | | | | | |
| Rosé Pine Dawn | #faf4ed base | #fffaf3 surface | #f2e9e1 overlay | #dfdad9 highlight_med | #dfdad9 highlight_med | #f4ede8 highlight_low | #575279 text (app value) | #797593 subtle | #9893a5 muted |
| Rosé Pine Main | #191724 base | #1f1d2e surface | #26233a overlay | #403d52 highlight_med | #403d52 highlight_med | #21202e highlight_low | #e0def4 text | #908caa subtle | #6e6a86 muted |
| Rosé Pine Moon | #232136 base | #2a273f surface | #393552 overlay | #44415a highlight_med | #44415a highlight_med | #2a283e highlight_low | #e0def4 text | #908caa subtle | #6e6a86 muted |
| Solarized light | #fdf6e3 base3 | #eee8d5 base2 | #CCCDC2 mix_base01 | #93A1A1 base1 | #CCCDC2 mix_base01 | #eee8d5 base2 | #002b36 base03 | #586E75 base01 | #93A1A1 base1 |
| Solarized dark | #002b36 base03 | #073642 base02 | #2C4E56 mix_base1 | #586E75 base01 | #0B4764 mix_blue | #073642 base02 | #fdf6e3 base3 | #93A1A1 base1 | #586E75 base01 |
| Tokyo Night Day | #e1e2e7 bg | #d0d5e3 bg_dark | #c4c8da bg_highlight | #b7c1e3 bg_visual | #b7c1e3 bg_visual | #c4c8da bg_highlight | #3760bf fg (needs adjusting) | #6172b0 fg_dark | #848cb5 comment |
| Tokyo Night Night | #1a1b26 bg | #292e42 bg_highlight | #3b4261 fg_gutter | #414868 terminal_black | #394b70 blue7 | #292e42 bg_highlight | #c0caf5 fg | #a9b1d6 fg_dark | #565f89 comment |
| Tokyo Night Storm | #24283b bg | #292e42 bg_highlight | #3b4261 fg_gutter | #414868 terminal_black | #394b70 blue7 | #292e42 bg_highlight | #c0caf5 fg | #a9b1d6 fg_dark | #565f89 comment |
| Tokyo Night Moon | #222436 bg | #2f334d bg_highlight | #3b4261 fg_gutter | #444a73 terminal_black | #394b70 blue7 | #2f334d bg_highlight | #c8d3f5 fg | #828bb8 fg_dark | #636da6 comment |

Extra named accents worth keeping as optional `accents`: Catppuccin rosewater, flamingo, maroon, sky, sapphire, lavender; Gruvbox bright, neutral and faded sets and `orange`; Kanagawa springViolet1/2, springBlue, boatYellow, autumn* and winter* tints (winterGreen #2B3328, winterYellow #49443C, winterRed #43242B, winterBlue #252535 are ready-made tinted card backgrounds for success, warning, danger, info); Rosé Pine rose, leaf; Solarized `mix_*` tints (dark: mix_red #422D33, mix_green #274C25, mix_yellow #364725, mix_blue #0B4764; light: mix_red #f9e2d1, mix_green #f1eccc, mix_yellow #f5ebcc, mix_blue #e7ebe1) as alert backgrounds; Tokyo Night blue0..blue7, teal, green1, green2, red1, git add/change/delete, and in Day `diff.*`; Dracula bright_* colours, `menu` #21222C and `black` #191A21 (darker than bg, usable as a sunken layer).

## Variants with no richer palette
- **Dracula Alucard, Nord Snow Storm:** no Neovim palette. Use the fallback column (`color-mix` of bg toward text).
- **Seoul256 (7 dark, 5 light variants):** the plugin has no named palette, only xterm indices. Native values exist as offsets from the background index: dark `CursorLine` and `NormalFloat` = bg-1, `Pmenu`, `LineNr`, `Folded` = bg+1, `ColorColumn` = bg-1; light `CursorLine` and `NormalFloat` = bg-1, `Pmenu`, `LineNr` = bg-2; `Visual` = xterm 23 (#005f5f) dark, 152 (#afd7d7) light; `Comment` = xterm 65 (#5f875f, green-grey, fails contrast, so use the fallback `comment`). `layers` can be taken from those xterm greys (the app already does this for surface and surface2) or from the fallback mix.
- **Dracula (dark) and Nord:** published ramps are short, so L3 is a light-ish grey that only text and strong text can sit on (see contrast).

## Contrast (WCAG, text slot on each layer L0..L3; fails < 4.5)
Computed from the table above, using the app's current `text` and `muted` and the new slots. `text` and `muted` are listed so you can see which existing slot would also fail on the new layers.

| Variant | Fails (slot@layer = ratio) |
|---|---|
| Latte | textSubtle L0..L3 4.37, 4.06, 3.73, 3.20; comment all (2.83 to 2.07); muted@L3 4.05 |
| Frappé | text@L3 3.61; strong@L3 3.61; subtle L1..L3 4.26, 3.23, 2.48; comment all (3.65 to 1.63) |
| Macchiato | text/strong@L3 4.17; subtle L2, L3 3.73, 2.79; comment all (4.14 to 1.74) |
| Mocha | subtle L2, L3 4.10, 3.00; comment all (4.44 to 1.81) |
| Dracula | subtle L2, L3 4.29, 3.67; app muted L1..L3 3.08, 2.89, 2.47; comment all (3.03 to 1.66) |
| Alucard (fallback) | muted/subtle/comment@L3 4.43 |
| Gruvbox light | subtle@L3 4.05; comment all (4.29 to 2.23) |
| Gruvbox dark | subtle@L3 3.80; comment all (4.02 to 1.77) |
| Kanagawa Lotus | subtle all (4.26, 3.73, 3.45, 3.21); comment all (2.93 to 2.21); muted@L3 4.20 |
| Kanagawa Wave | subtle@L3 3.98; comment all (3.33 to 1.49) |
| Kanagawa Dragon | text/strong@L3 3.84; subtle@L3 2.62; comment L1..L3 (3.83, 3.01, 1.65) |
| Nord | comment all (2.43 to 1.44); everything else passes |
| Nord Snow Storm (fallback) | none |
| Rosé Pine Dawn | subtle all (4.02, 4.23, 3.67, 3.17); comment all (2.73 to 2.15); muted@L3 3.92 |
| Rosé Pine Main | subtle@L3 3.25; comment all (3.42 to 2.03) |
| Rosé Pine Moon | subtle L1..L3 4.46, 3.61, 3.03; comment all (3.03 to 1.89) |
| Solarized light | subtle L1..L3 4.39, 3.35, 2.01; comment all (2.48 to 1.00); muted L2, L3 3.45, 2.07 |
| Solarized dark | text@L3 4.39; subtle L2, L3 3.37, 2.01; comment all (2.79 to 1.00) |
| Tokyo Night Day | strong L1..L3 (3.99, 3.52, 3.28); subtle all (3.57 to 2.59); comment all (2.54 to 1.84); text@L3 4.20; muted L2, L3 4.48, 4.17 |
| Tokyo Night Night | subtle@L3 4.23; comment all (2.76 to 1.44) |
| Tokyo Night Storm | subtle@L3 4.23; comment all (2.35 to 1.44) |
| Tokyo Night Moon | subtle L1..L3 3.72, 2.96, 2.56; comment all (3.11 to 1.73) |
| Seoul256 dark-233, 234, 235 | none with the 5/10/15% fallback |
| Seoul256 dark-236 | subtle@L3 3.99 |
| Seoul256 dark-237, 238, 239; light-252 to 256 | subtle/muted fail on L1 or lower (and `text` itself on L1..L3 for 238, 239, 252, 253, 254); note the app's own `muted` already sits at 4.2 to 4.7 on bg, so these ratios are a ceiling, and the fallback mixes should shrink to 3/6/9% |

Selection and highlight with the body `text`: all pass (>= 4.5) except Latte text@selection 4.39, Frappé 3.61, Macchiato 4.17, Mocha 4.62 (passes), Tokyo Night Day 4.20 on selection, Kanagawa Dragon 7.74, Seoul256 light Visual (#afd7d7) with text #616161 3.99.

## Findings and recommendation
1. Recommended slot set: `layers` (4), `textStrong`, `textSubtle`, `comment`, `selection`, `highlight`, optional `accents`; all optional with the fallbacks above.
2. `comment` fails 4.5:1 in every variant that publishes it (it is a decorative colour in each theme). Use it for placeholders and disabled text only, or derive it by mixing toward `text` until 4.5 (the R-002 rule).
3. Deep layers (L3, and L2 in several themes) fail for `textSubtle`: restrict secondary text to L0 and L1 (L2 in themes that pass), or let the generator pick `text` when the ratio is below 4.5.
4. Variants whose `textSubtle` or `textStrong` fails on L0 or L1: Latte, Lotus, Rosé Pine Dawn, Solarized light, Tokyo Night Day (the Day palette needs the R-002 adjusted text first), and Dracula/Seoul256 for muted.
5. Dracula and Nord (dark) have the weakest neutrals ramp; Catppuccin, Gruvbox, Kanagawa and Rosé Pine have the best.
6. The nord.nvim port is WTFPL; the other sources are MIT or Apache-2.0 (Tokyo Night), which the family `license` field already names.
7. Rosé Pine Dawn: this Neovim file has text `#464261`, R-002's source has `#575279` (the app uses the latter); keep the app value.
