---
id: Q-043
title: Who can create and edit tasks and lists
status: resolved
batch: 14
area: security
created: 2026-10-24
resolved: 2026-10-25
decisions: [D-045]
---

# Q-043: Who can create and edit tasks and lists

## Question
D-006 and the RBAC matrix in `Domain.md` cover reading and completing tasks, but not creating or editing them.

**Who can create and edit what?**

## Options
- A. Adults create and edit everything. Children can only complete tasks.
- B. Adults manage lists and recurring definitions. Children can add **one-off** tasks to their own list ("bring a permission slip", homework) and edit their own one-off tasks. Anyone can add one-off tasks from the dashboard to a list marked `dashboard_add`.
- C. **Configurable per list, default B** (recommended): each list has `child_can_add: own | none` and `dashboard_add: true|false` (default true for a "General" list and false otherwise).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
We'll go option C.
