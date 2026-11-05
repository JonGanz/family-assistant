---
id: D-095
title: The app UI is a home-grown Material 3-style component library on the theme variables, with Material Symbols
status: accepted
date: 2026-11-04
source_questions: [Q-085]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-095: The app UI is a home-grown Material 3-style component library on the theme variables, with Material Symbols

## Context
The non-dashboard app (Today, Calendar, Tasks, Routines, Mail, Inbox, Bills, Settings) grew screen by screen with ad-hoc styles. The user asked for a revamp: a consistent design language, reusable mobile-first components, better readability and separation, a new menu, iconography, with Material UI as inspiration (its cards especially), keeping the theme colour variables (D-079). Q-085.

## Decision
- A **home-grown Vue component library** in `apps/web/src/ui/`, modelled on Material 3 patterns: card, list and list item, top app bar, navigation (bottom bar on phones, rail on tablets, drawer or rail on desktop), dialog and bottom sheet, snackbar, chips, text field, select, switch, segmented button, FAB, empty and error states.
- All colour comes from the **existing theme variables** (D-079, D-080, D-081); the library adds tokens only for shape, elevation, spacing, type scale and motion, derived per theme where colour is involved (tonal surfaces mix existing variables).
- **Icons: Material Symbols** as tree-shaken SVG (package per R-005), coloured with `currentColor`.
- **Mobile-first:** layouts start at phone width; touch targets at least 48 px.
- The kiosk dashboard is out of scope (D-068 and its views keep their own look), except where it shares a component.

## Consequences
- Every non-dashboard view moves onto the library; ad-hoc page styles go.
- One new web dependency for icons; no UI framework.

## Alternatives considered
- Lucide icons: a fine set, but a thinner line style that matches Material less well.
- Vuetify: faster breadth, but a heavy bundle and hard to map onto the terminal-theme catalogue (D-079).
