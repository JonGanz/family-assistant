---
id: Q-040
title: The "now important" list: what goes on it and in what order
status: resolved
batch: 14
area: domain
created: 2026-10-24
resolved: 2026-10-25
decisions: [D-042]
---

# Q-040: The "now important" list: what goes on it and in what order

## Question
PROJECT.md wants tasks pushed into a "now important" list. In M4 a System One model will add an urgency score (D-027). M2 needs a **deterministic** priority first.

The proposed score uses: how close the due time is (and whether it's overdue), the definition's lead time ("show 2 days before due"), a manual priority flag (`normal | high`), and the origin (weather-triggered tasks rank high).

**What should the "Now" list show, and how should it be configured?**

## Options
- A. Everything due today or overdue, sorted by due time. No score.
- B. The top N by the score above (N = 7), across all members.
- C. **Configurable, default B** (recommended): the score weights and N are household settings (editable in the UI, D-029 style), and each dashboard device can filter by member or list. Phones show the current user's Now list first, then the household's.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C. I see you've picked up on me wanting this to be highly configurable. 
