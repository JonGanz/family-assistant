---
id: Q-039
title: Children's morning (and other) routines
status: open
batch: 14
area: domain
created: 2026-10-24
resolved:
decisions: []
---

# Q-039: Children's morning (and other) routines

## Question
PROJECT.md lists the children's morning routine (get dressed, eat, brush teeth, pack lunch). A routine differs from a chore: it repeats every day (or on school days), has an order, and has a target time ("out the door at 7:45").

**How should routines work?**

## Options
- A. A routine is just a daily recurring task with steps (`TaskStep`). It shows as one item with a step checklist.
- B. A routine is its own kind: a per-member checklist with a **time window** (starts at, done by), optional per-step target times, and **day filters** (weekdays, school days from a calendar, weekends). The dashboard shows a big, touch-friendly checklist per child with a progress bar.
- C. **B, with configurable day filters and an optional dashboard panel, default weekdays and the panel shown in the morning window** (recommended). The panel appears automatically during a routine's window on the devices that enable it. Steps are checked off without a PIN (D-019).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
