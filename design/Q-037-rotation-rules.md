---
id: Q-037
title: Rotation details: who did it, skips, absences
status: open
batch: 14
area: domain
created: 2026-10-24
resolved:
decisions: []
---

# Q-037: Rotation details: who did it, skips, absences

## Question
D-013 defines `rotation` as an ordered set of members, where the task moves on after each completion. Some edge cases are still open:

1. **Someone else does it.** Riley's turn, but Alex does it. Does the rotation still move past Riley (Riley's turn is used up), or does Riley stay next (Alex covered a turn)?
2. **Skip.** An adult skips an occurrence. Does the turn move on?
3. **Absence.** Should a member be skippable for a date range (away at camp, ill)? Their turns would go to the next person.

**How should rotation handle these?**

## Options
- A. Simple: the rotation moves on after every completion or skip, whoever did it. No absences.
- B. Fair: the rotation follows the **assignee**. If someone else covers, the assignee keeps their turn ("Alex covered Riley"). A skip doesn't move the turn. Members can be marked away for a date range and are skipped while away.
- C. **Configurable per definition, default A for who-did-it and skips, plus member absences as a household feature** (recommended): `advance_on: any_completion | assignee_completion` and `skip_advances: true|false`, plus an `away` date range on a member that every strategy honours (and smart assignment uses later).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
