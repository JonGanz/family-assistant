---
id: Q-035
title: When the next occurrence of a recurring task is due
status: open
batch: 14
area: domain
created: 2026-10-24
resolved:
decisions: []
---

# Q-035: When the next occurrence of a recurring task is due

## Question
M2 adds recurring task definitions (D-013, D-023). There are two common ways to schedule the next occurrence:

- **Schedule-anchored:** follows the calendar ("every Saturday", "1st of the month"). Missing one week doesn't move the next one.
- **Completion-anchored:** counts from when it was last done ("7 days after the toilet was last cleaned"). PROJECT.md's toilet example reads like this, and `Domain.md` currently assumes it.

Some chores fit one way better than the other: bins go out on collection day, while cleaning fits "every N days since last time".

**How should the next occurrence be scheduled?**

## Options
- A. Always completion-anchored (as Domain.md assumes now).
- B. Always schedule-anchored (an RRULE, like calendar events).
- C. **Configurable per definition, default completion-anchored** (recommended): each definition picks `schedule` (an RRULE) or `after_done` (an interval like 7 days). The UI offers both, and new chores start as `after_done`.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C.
