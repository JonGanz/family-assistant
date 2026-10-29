---
id: D-079
title: The app ships a catalogue of popular terminal themes, as built-in data
status: accepted
date: 2026-10-29
source_questions: [Q-073, Q-076]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-079: The app ships a catalogue of popular terminal themes, as built-in data

## Context
The user wants the app's colours to be aesthetic and to have depth: several neutral levels for the interface, plus named hues. Popular terminal palettes already have exactly that shape. The user asked for Kanagawa to be included (Q-073 follow-up, 2026-10-29).

## Decision
- The app ships these theme families: **Catppuccin** (Latte light; Frappé, Macchiato and Mocha dark), **Nord**, **Dracula**, **Gruvbox** (light and dark), **Solarized** (light and dark), **Tokyo Night**, **Rosé Pine** and **Kanagawa**. A family contributes the light and/or dark variants its project publishes.
- **A family with only one variant uses it all the time.** If a family has only a dark variant, the app shows it dark at noon too; the time of day and the device's light/dark setting are ignored for it (the user's rule).
- For a family with both modes, the mode is `auto` (follow the device's light/dark setting), `light` or `dark`.
- Each variant defines the same fixed token set: neutral levels (background, surfaces, borders, muted text, text) and eight named hues (red, orange, yellow, green, cyan, blue, purple, pink). A palette without a native hue supplies its nearest colour.
- **Built-in themes only (Q-076, option A).** The theme format is plain data, so custom themes can be added later without a redesign.

## Consequences
- Replaces the `auto|light|dark` theme and the few colour variables in `styles/base.css`.
- Each variant is checked against contrast rules when the app is built (see `arch/Frontend.md` §3d), because the palettes were not designed for this UI.
- The kiosk uses the same catalogue (D-081).
- Palette values and licences are taken from each project's published palette and recorded with their source; the plan task verifies them.

## Alternatives considered
- A smaller set: rejected, since the user asked for popular themes and Kanagawa on top.
- Household-editable themes (Q-076 option B, or C as a toggle): not now; the data format keeps the door open.
