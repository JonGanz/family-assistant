---
id: Q-078
title: Visibility of tasks made from a private mailbox's mail
status: resolved
batch: 21
area: tasks
created: 2026-11-02
resolved: 2026-11-07
decisions: [D-098]
---

# Q-078: Visibility of tasks made from a private mailbox's mail

## Question
Code review 2 (C2-21) found that tasks created from mail skip the mailbox visibility that skip and accept enforce: any adult can list, edit or delete a suggested or open task derived from another adult's owners-only mailbox, and the title is the mail subject. Once such a task is accepted, should it be an ordinary household task (every adult and the dashboard see it), or keep following the mailbox's visibility until completed?

## Options
- A. Household task once accepted; follows the mailbox while suggested (suggested default)
- B. Always follows the mailbox visibility until completed
- C. Always a household task, even while suggested
- D. Configurable per mail source, default A

## Answer
<!-- answer below this line -->
Option D.
