---
id: D-083
title: Add seoul256 to the theme catalogue and a light variant (Snow Storm) to Nord
status: accepted
date: 2026-10-29
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-083: Add seoul256 and Nord Snow Storm

## Context
Direct user instruction (2026-10-29, no question), extending D-079: add the seoul256 theme, use Nord's light palette as Nord's light variant, and offer variants when a device's theme is customised.

## Decision
- **seoul256** joins the catalogue with every background level the plugin supports: dark 233 to 239 (default 237) and light 252 to 256 (default 253). Colours come from the plugin's own GUI table (research/R-003).
- **Nord** gains a light variant, **Snow Storm** (nord4 to nord6 as backgrounds, Polar Night as text). Nord publishes no light UI palette, so its hues are darkened as needed for the contrast rules (R-003).
- The variant picker appears on the Appearance page and in the layout editor whenever the chosen mode has more than one variant (seoul256 has seven dark and five light).

## Consequences
- D-079's rule "a single-variant family is used at all times" no longer applies to Nord, which now has a mode choice. A saved Nord choice with `mode: auto` follows light and dark from now on.
- Adjusted values are listed in R-003. All variants pass the contrast rules in `themes.test.ts`.
