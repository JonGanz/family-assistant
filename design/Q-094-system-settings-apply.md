---
id: Q-094
title: How Settings → System applies switches
status: open
batch: 23
area: frontend
created: 2026-11-04
resolved:
decisions: []
---

# Q-094: How Settings → System applies switches

## Question
D-096 says switches apply instantly with Undo, but every System save re-reads the server config and restarts parts of it.

## Options
- A. System switches are part of the page draft and save with the Save bar (applied)
- B. Instant with Undo like everywhere else (an Undo means a second restart)

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-14). Tasks settings tabs became filter chips (`?tab=` kept); empty lists show a FAB plus an empty-state button.

## Answer
<!-- answer below this line -->

