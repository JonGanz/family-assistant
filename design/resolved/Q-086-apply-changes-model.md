---
id: Q-086
title: How pages apply changes
status: resolved
batch: 22
area: frontend
created: 2026-11-04
resolved: 2026-11-04
decisions: [D-096]
---

# Q-086: How pages apply changes

## Question
Settings and edit pages apply changes inconsistently today (some save on change, some need a button). Which single model applies everywhere?

## Options
- A. Toggles and single choices apply instantly with an Undo snackbar; multi-field forms use a sticky Save/Discard bar with a leave guard (suggested)
- B. Configurable, default Save bar
- C. Save bar everywhere
- D. Autosave per field

## Answer
<!-- answer below this line -->
A. Toggles instant with Undo, forms with a Save/Discard bar (2026-11-04, in session).

