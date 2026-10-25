---
id: D-046
title: Tasks in calendar views: per-task display mode; the agenda is reserved for key dates
status: accepted
date: 2026-10-25
source_questions: [Q-044]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Frontend.md]
---

# D-046: Tasks in calendar views: per-task display mode; the agenda is reserved for key dates

## Context
The user picked option C and refined it:
- Some homework is tied to a date that matters on its own, such as a school presentation. Work leads up to it, and it should **always** show on the calendar, even after the work is done.
- Other homework, like reading, only needs signing off. Once it is done, it no longer needs to show.
- Chores belong on the task list, not the calendar. The answer's first sentence says chores "would be something we need to see in the calendar", but the next sentence says "we just want to see that on the task list". This decision reads the two together as *not in the calendar*. It is a configurable default either way.
- The agenda view is for big items: the day's events and larger school or work items.

## Decision
- Each task has `calendar_display: hidden | until_done | always`. The value is copied from its list's default when the task is created, and can be changed on the task.
  - `until_done`: shown on the Calendar page and the week panel until it is completed.
  - `always`: a **key date**. It stays on the calendar after completion (shown as done) and is the only kind of task that appears in the **agenda** panel.
- List defaults:
  - homework: `until_done`;
  - chores, routines and general: `hidden`.
- When a task is marked as a key date, its preparation work can be added as **steps** or as separate one-off tasks due earlier.

## Consequences
- The agenda panel shows events plus `always` tasks only.
- This is a per-task setting, finer than the per-list option offered in Q-044, because homework needs both behaviours.

## Alternatives considered
Tasks never in the calendar (A), or all dated tasks in the calendar (B). Not chosen.
