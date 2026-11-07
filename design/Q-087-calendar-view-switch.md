---
id: Q-087
title: Where the Calendar's view switch goes, and the default view
status: open
batch: 23
area: frontend
created: 2026-11-04
resolved:
decisions: []
---

# Q-087: Where the Calendar's view switch goes, and the default view

## Question
The Calendar has Agenda, Week and Month views. At 360 px the switch does not fit next to the menu button and title in the top bar. Which view opens by default?

## Options
- A. Switch in a row under the top bar; default Agenda on phones, Week on tablets and desktop (applied)
- B. Switch in the top bar's overflow menu; same defaults
- C. Configurable per device, default as A

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-10). Month scrolls sideways when narrow; '+N more' in Month has no per-day detail yet.

## Answer
<!-- answer below this line -->
Option A is valid, but we should also maintain what the last state was and return to that view when returning. This will allow a user to always be brought back to the last view they used, without needing an unnecessary configuration option.
