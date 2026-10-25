---
id: Q-046
title: Default for missed schedule-anchored tasks
status: open
batch: 15
area: domain
created: 2026-10-25
resolved:
decisions: []
---

# Q-046: Default for missed schedule-anchored tasks

## Question
D-038 makes `on_missed` configurable per definition, and completion-anchored (`after_done`) tasks roll over by default, as you answered in Q-036. You didn't say what **schedule-anchored** (`rrule`) tasks should do by default. Examples: bins out on collection day, water plants every Monday and Thursday.

If the bins weren't taken out on Tuesday, keeping that occurrence open after collection day is useless. For "water plants Monday and Thursday", a missed Monday is still worth doing on Tuesday.

**What default should new schedule-anchored definitions get?** (Adults can change it per definition either way.)

## Options
- A. `roll_over` (same as after-done tasks): the missed one stays open and overdue until done or skipped.
- B. `auto_skip`: when the next scheduled one comes due, the missed one is marked skipped (logged) and replaced.
- C. **Configurable, default B** (recommended): new `rrule` definitions start as `auto_skip`, since a schedule usually means the date matters. The edit form explains the choice.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C. 
