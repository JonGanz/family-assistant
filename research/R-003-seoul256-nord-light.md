---
id: R-003
title: seoul256 theme family and a light Nord (Snow Storm)
date: 2026-10-29
method: Sonnet 5.5 sub-agent; shallow git clones of junegunn/seoul256.vim and nordtheme/nord, values read from the source files (no memory); contrast checked with the app's own formula
verified: every colour was read from the cloned sources on 2026-10-29. seoul256.vim has no LICENSE file; the README says MIT. nordtheme/nord has a `license` file (MIT, Sven Greb)
informs: [D-079]
---

# R-003: seoul256 and Nord Snow Storm

Extends R-002 (same token mapping and contrast rules). Data: `apps/web/src/themes/families/seoul256.ts` and `nord.ts`.

## seoul256
Sources: `colors/seoul256.vim`, `colors/seoul256-light.vim`, README (github.com/junegunn/seoul256.vim). The plugin is low contrast and takes a background index through `g:seoul256_background`: dark 233 to 239 (default 237), light 252 to 256 (default 253). One variant per level: `dark-237` (default), `dark-233`..`dark-239`, `light-253` (default), `light-252`, `light-254` to `light-256`. Order: default first in each mode.

### xterm to hex
The plugin sets GUI colours from its own table `s:rgb_map` (used unless `g:seoul256_srgb=1`; it is a slightly shifted version of xterm, for example 237 is `#4b4b4b` there and `#3a3a3a` in xterm-256). This note uses that table, and the standard xterm-256 table for indexes it lacks (232, 242 to 248 and 256 are not in the plugin map; 256 means white, as the plugin does). Colours the plugin uses: 233 `#171717`, 234 `#252525`, 235 `#333233`, 236 `#3f3f3f`, 237 `#4b4b4b`, 238 `#565656`, 239 `#616161`, 241 `#757575`, 249 `#bfbfbf`, 252 `#d9d9d9`, 253 `#e1e1e1`, 254 `#e9e9e9`, 255 `#f1f1f1`, 256/231 `#ffffff`; hues 161 `#e12672`, 125 `#bf2172`, 168 `#e17899`, 173 `#e19972`, 131 `#be7572`, 179 `#dfbc72`, 94 `#9a7200`, 108 `#98bc99`, 65 `#719872`, 73 `#6fbcbd`, 23 `#007173`, 68 `#719cdf`, 25 `#0074be`, 103 `#999abd`, 96 `#9a7599`.

### Token mapping
- bg: the level (B). text: Normal foreground (252 dark, 239 light).
- Dark: surface B-1 and surface2 B-2 (16 when B is 233 or less), as CursorLine and CocFloating. Light: surface B+1, surface2 B+2 (as Conceal and Todo).
- border: TabLine background (dark B+4, light B-4).
- muted: nearest neutral (249 dark, 241 light). The plugin has no muted foreground; its Comment (65, green-grey) fails contrast on every level.
- onAccent: the background colour (derived where it fails on blue).
- Hues, from highlight groups (dark/light): red Exception (161) / Todo (125); orange Define (173/131); yellow Type (179/94); green diffAdded (108/65); cyan Constant (73/23); blue Repeat, GitGutterChange (68) / SpellCap (25); purple Boolean (103) / Identifier (96); pink Keyword (168). The plugin has no group that is orange or pink by name, so these are the closest published colours.

## Nord Snow Storm
Sources: `src/nord.css`, `readme.md`, `changelog.md`, `license` (github.com/nordtheme/nord). The repository publishes no light UI palette. The CSS comments describe Snow Storm (nord4 to nord6) as "semi-light background depending on the theme shading design", so the variant is a reading of that: bg nord6 `#eceff4`, surface nord5 `#e5e9f0`, surface2 nord4 `#d8dee9`, text nord0 `#2e3440`, muted nord3 `#4c566a`, onAccent nord6. Hues are the dark mapping (nord11, 12, 13, 14, nord8 cyan, nord10 blue `#5e81ac` instead of nord9, nord15 purple and pink); the dark mapping's blue nord9 is not used here because nord10 is the darker published blue. Border is nord3 mixed 55% toward nord6 (derived). The dark `nord` variant is unchanged.

## Adjusted values (contrast rules)
Rules as in R-002. Values were mixed toward white (dark) or black (light) in 1% steps until the rule passed; "Ratio before" is the worst case over the surfaces or bg.

| Variant | Token | Published / mapped | Used | Ratio before |
|---|---|---|---|---|
| seoul256 dark-237 | red | `#e12672` | `#eb6ea1` | 1.96 |
| seoul256 dark-237 | onAccent | `#4b4b4b` | `#333333` | 3.13 |
| seoul256 dark-235 | red | `#e12672` | `#e33179` | 2.87 |
| seoul256 dark-236 | red | `#e12672` | `#e75490` | 2.37 |
| seoul256 dark-236 | onAccent | `#3f3f3f` | `#333333` | 3.77 |
| seoul256 dark-238 | muted | `#bfbfbf` | `#cbcbcb` | 3.99 |
| seoul256 dark-238 | red | `#e12672` | `#ee85b0` | 1.65 |
| seoul256 dark-238 | blue | `#719cdf` | `#82a8e3` | 2.63 |
| seoul256 dark-238 | purple | `#999abd` | `#a3a4c4` | 2.70 |
| seoul256 dark-238 | pink | `#e17899` | `#e68ca8` | 2.57 |
| seoul256 dark-238 | onAccent | `#565656` | `#3c3c3c` | 3.03 |
| seoul256 dark-239 | text | `#d9d9d9` | `#dcdcdc` | 4.39 |
| seoul256 dark-239 | muted | `#bfbfbf` | `#dcdcdc` | 3.37 |
| seoul256 dark-239 | red | `#e12672` | `#f29dc0` | 1.39 |
| seoul256 dark-239 | orange | `#e19972` | `#e5a786` | 2.65 |
| seoul256 dark-239 | green | `#98bc99` | `#9bbe9c` | 2.95 |
| seoul256 dark-239 | cyan | `#6fbcbd` | `#79c1c2` | 2.83 |
| seoul256 dark-239 | blue | `#719cdf` | `#97b7e8` | 2.22 |
| seoul256 dark-239 | purple | `#999abd` | `#b3b3ce` | 2.27 |
| seoul256 dark-239 | pink | `#e17899` | `#eaa1b8` | 2.17 |
| seoul256 dark-239 | onAccent | `#616161` | `#474747` | 3.02 |
| seoul256 light-253 | muted | `#757575` | `#636363` | 3.52 |
| seoul256 light-253 | orange | `#be7572` | `#b16d6a` | 2.69 |
| seoul256 light-253 | green | `#719872` | `#668967` | 2.49 |
| seoul256 light-253 | purple | `#9a7599` | `#987497` | 2.99 |
| seoul256 light-253 | pink | `#e17899` | `#bd6581` | 2.19 |
| seoul256 light-253 | onAccent | `#e1e1e1` | `#f4f4f4` | 3.79 |
| seoul256 light-252 | text | `#616161` | `#5f5f5f` | 4.39 |
| seoul256 light-252 | muted | `#757575` | `#5f5f5f` | 3.26 |
| seoul256 light-252 | orange | `#be7572` | `#ab6967` | 2.49 |
| seoul256 light-252 | green | `#719872` | `#618362` | 2.31 |
| seoul256 light-252 | purple | `#9a7599` | `#926f91` | 2.77 |
| seoul256 light-252 | pink | `#e17899` | `#b4607a` | 2.02 |
| seoul256 light-252 | onAccent | `#d9d9d9` | `#f4f4f4` | 3.51 |
| seoul256 light-254 | muted | `#757575` | `#696969` | 3.80 |
| seoul256 light-254 | orange | `#be7572` | `#b8716f` | 2.89 |
| seoul256 light-254 | green | `#719872` | `#6a8f6b` | 2.69 |
| seoul256 light-254 | pink | `#e17899` | `#c46885` | 2.35 |
| seoul256 light-254 | onAccent | `#e9e9e9` | `#f4f4f4` | 4.08 |
| seoul256 light-255 | muted | `#757575` | `#6e6e6e` | 4.08 |
| seoul256 light-255 | green | `#719872` | `#6e936f` | 2.89 |
| seoul256 light-255 | pink | `#e17899` | `#cd6d8b` | 2.53 |
| seoul256 light-255 | onAccent | `#f1f1f1` | `#f4f4f4` | 4.38 |
| seoul256 light-256 | pink | `#e17899` | `#da7494` | 2.86 || nord snow-storm | orange | `#d08770` | `#b97864` | 2.47 |
| nord snow-storm | yellow | `#ebcb8b` | `#9b865c` | 1.35 |
| nord snow-storm | green | `#a3be8c` | `#7c906a` | 1.77 |
| nord snow-storm | cyan | `#88c0d0` | `#66909c` | 1.74 |
| nord snow-storm | blue | `#5e81ac` | `#506e92` | 3.50 (onAccent) |
| nord snow-storm | purple, pink | `#b48ead` | `#a07e9a` | 2.46 |
| nord snow-storm | border | `#4c566a` | `#a4aab6` | not a contrast rule: a lighter line, chosen by eye |
