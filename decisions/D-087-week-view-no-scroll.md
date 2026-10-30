---
id: D-087
title: The Week view never scrolls; a cell that is too full folds items into "+N"
status: accepted
date: 2026-10-30
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-087: Week view cells fold into "+N"

## Context
Direct user instruction (2026-10-30, no question), refining D-082 and D-084: the Week view must not scroll. The user proposed splitting a busy cell into columns and, when that is not enough, a "+X" slot that opens everything. The user then confirmed the defaults below.

## Decision
- Rows share the height equally; a cell never grows and nothing scrolls.
- A cell measures itself. Items fill one full-width column first. Only when they do not fit that way are they split into two half-width columns (names ellipsised), and only if each half stays at least 9rem wide; otherwise the cell keeps its full-width column. A slot is a fixed 3.2rem.
- Items are ordered events first (all-day first, then by start time), then tasks, so "+N" hides the least time-bound items.
- When they still do not all fit, the last slot (the bottom row's final half when split) is a "+N" button. Tapping it opens a modal with everything in that cell (one person, one day: events and tasks); each event opens its detail modal.

## Consequences
- With many people the cells are narrow and show "+N" sooner; the layout does not shrink titles to fit.
- Capacity is computed in the browser from measured sizes (`dayCellFit.ts`); until a cell is measured, everything is drawn.
