---
id: Q-036
title: What happens to recurring tasks that aren't done in time
status: open
batch: 14
area: domain
created: 2026-10-24
resolved:
decisions: []
---

# Q-036: What happens to recurring tasks that aren't done in time

## Question
A recurring task can pass its due time while still open. For example, "Water plants" every 3 days might not be done for a week.

**What should happen to the missed occurrence?**

## Options
- A. **Roll over** (one open occurrence): it stays open, shows as overdue, and keeps getting more urgent. No new occurrence is created until it's done or skipped.
- B. **Pile up:** each due date creates a new occurrence, so a missed week leaves several open items.
- C. **Auto-skip:** when the next one comes due, the old one is marked `skipped` (logged) and replaced.
- D. **Configurable per definition, default A** (recommended): `on_missed: roll_over | auto_skip | pile_up`. Schedule-anchored chores like bins might use `auto_skip`; cleaning would use `roll_over`.
- E. Something else (describe).

## Answer
<!-- answer below this line -->
Option A when completion-anchored. It may also be nice for a progressive task that is as an example read 10 minutes a night if not checked off may become read 10 minutes the next night and then 30 the next. Actually, that would probably be better set as a goal with progress. So ignore that. That won't be a task. We might want goals on here instead, things to track how far kids are along on homework for adults on other targets and objectives.
