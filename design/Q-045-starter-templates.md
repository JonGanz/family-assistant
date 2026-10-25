---
id: Q-045
title: Starter chores and routines
status: open
batch: 14
area: frontend
created: 2026-10-24
resolved:
decisions: []
---

# Q-045: Starter chores and routines

## Question
New households start empty. The project could ship a **library of starter templates**: common chores with suggested intervals (clean toilets every 7 days, change bed sheets every 14 days, water plants every 3 days, replace HVAC filter every 90 days) and a sample morning routine. Templates would be generic (no household specifics, D-035).

**Should there be starter templates?**

## Options
- A. No. Households add their own.
- B. Yes, a template picker in Settings → Tasks. Adults tick the ones they want, then assign members.
- C. **B, plus the example household TOML gets a few chores and a routine so the local setup shows tasks right away** (recommended). The template library is a data file in the repo that anyone can extend.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
We'll go with option A for now. 
