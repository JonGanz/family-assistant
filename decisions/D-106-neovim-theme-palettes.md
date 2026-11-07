---
id: D-106
title: Canonical accents; extended theme palettes from the Neovim ports
status: accepted
date: 2026-11-07
source_questions: [Q-092]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

> Implementation note (T-21, 2026-11-07): where the published text colour is under 4.5:1 on a published layer, the applied layer value is pulled toward bg in 10 % steps until it passes. The theme data keeps the Neovim hex verbatim.

# D-106: Canonical accents; extended theme palettes from the Neovim ports

## Context
Q-092: M4 (T-13) darkened the accent in Solarized light, Tokyo Night day and Rosé Pine dawn so it reached AA as text. The user chose to keep the canonical accents (option B). They also asked whether the themes, most of which they use in Neovim, could take on the richer palettes of their Neovim ports for better card backgrounds and text colours. R-006 read those palettes from the source repos.

## Decision
1. The three light themes keep their published accents. The M4 darkening is reverted, and accent-as-text below 4.5:1 is accepted in those variants.
2. A theme variant gains optional extended slots taken verbatim from its Neovim port (R-006):
   - `layers`: 4 steps (page = `bg`, card, raised, dialog);
   - `textStrong`, `textSubtle`, `comment`;
   - `selection`, `highlight`;
   - optional extra `accents`.
   Card and surface tokens use `layers` when present. Variants without a port (Dracula Alucard, Nord Snow Storm, Seoul256) keep the `color-mix` derivation; Seoul256 uses smaller mix steps.
3. Palette values are not altered for contrast. Instead the token layer picks, per layer, a text slot that reaches 4.5:1:
   - secondary text uses `textSubtle` where it passes and falls back to `muted`, then `text`;
   - `comment` is used only for placeholders and disabled text;
   - a tokens test checks every variant and layer pair.
   The existing R-002 adjustments to `text` and `muted` stay.

## Consequences
A web task (T-21): extend `ThemeVariant` and the family files, `tokens.css` mapping, and the contrast test. Themes look closer to the user's editor, and cards gain distinct layers. Accent-coloured text in three light variants stays under AA by the user's choice.

## Alternatives considered
Q-092 A (darkened accents, applied in M4 then reverted), C (a high-contrast toggle). For the palettes: keep the 7-neutral model and derive everything with color-mix (the M4 state), or alter Neovim colours to pass contrast (rejected, in line with choosing B).
