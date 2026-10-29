---
id: Q-073
title: Which colour themes does the app ship, and how do light and dark pair up?
status: resolved
batch: 19
area: frontend
created: 2026-10-29
resolved: 2026-10-29
decisions: [D-079]
---

# Q-073: Which colour themes does the app ship, and how do light and dark pair up?

## Context
Today the theme is only `auto|light|dark` over a few CSS variables. The proposal is to replace that with a catalogue of well-known terminal palettes. Each has several neutral levels (background, surfaces, borders, muted text, text) and a fixed set of named hues (red, orange, yellow, green, cyan, blue, purple, pink). Switching theme re-tints the whole app. The user has asked for Kanagawa to be included. All candidates are permissively licensed, so the public repo (D-035) can include them.

## Question
**Which themes ship, and how does `auto` (follow the device's light/dark setting) work?**

## Options
- A. **Ship this set: Catppuccin (Latte light; Frappé, Macchiato and Mocha dark), Nord, Dracula, Gruvbox (light and dark), Solarized (light and dark), Tokyo Night, Rosé Pine, Kanagawa.** Each theme is marked light or dark. `auto` uses the household's chosen light theme and chosen dark theme and follows the device (recommended).
- B. A smaller set (name which ones).
- C. **Configurable, default A:** the catalogue is data that can be edited, and the shipped set is the default.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option A. If a theme doesn't have a dark and/or light theme, then use the one it has the entire time (ie, disregard time of day -- show a dark theme in the middle of the day if the selected theme only has dark mode).
