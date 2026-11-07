---
id: Q-092
title: Darker accents in three light themes for AA contrast
status: resolved
batch: 23
area: frontend
created: 2026-11-04
resolved: 2026-11-07
decisions: [D-106]
---

# Q-092: Darker accents in three light themes for AA contrast

## Question
In Solarized light, Tokyo Night day and Rosé Pine dawn the accent as text on the surface was about 3:1. Fixing it needed a darker blue than the palettes publish.

## Options
- A. Darken the accent: Solarized light #1e6ca4, Tokyo Night day #1f549c, Rosé Pine dawn #417079; on-accent back to the palette's light value (applied)
- B. Keep the canonical accents and accept the AA failure
- C. Configurable per device ('high contrast' toggle), default A

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-13). It changes the look of those themes noticeably.

## Answer
<!-- answer below this line -->
Option B. That said, the theme's selected have been (for the most part) been personally used with Neovim, where we have more than the standard terminal hue's for options, which give the editor far more depth of options. Can these theme's be bolstered with more variants from their Neovim counterparts? This would allow for better options with card backgrounds and text colors.
