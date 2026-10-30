---
id: D-084
title: The Week view shows calendar weeks from a configurable first day, with week navigation and a hide-past toggle
status: accepted
date: 2026-10-29
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Domain.md]
---

# D-084: Week view weeks, navigation and hide-past

## Context
Direct user instruction (2026-10-29, no question), extending D-082: the Week view should show the current calendar week, let the viewer move between weeks, and optionally hide what has passed. The household chooses the first day of the week.

## Decision
- **Household setting `week_start`**: `monday` or `sunday`, default `monday` (configurable, default Monday). Set in Settings, Household. It is carried in the dashboard snapshot's `household` object.
- The Week view shows the **calendar week** that starts on that day and holds today (the day keeps its highlight). The week is no longer "today plus six".
- A bottom row has previous and next arrows and a box showing the shown week: "Current" for the week holding today, otherwise the date range. The shown week is local to the screen and resets on reload.
- `GET /api/v1/dashboard` takes an optional `from` (a date in the household time zone) that starts the event window there, so any week can be fetched. The view fetches its own week and refetches on the same live updates as the snapshot.
- A **"Hide past events" switch** (off by default) hides events whose end has passed (all-day events once their last day is over). Tasks are not hidden for being late: with the switch on, a task due today or earlier goes only once it is completed; open tasks stay.
- Weather icons in the day cell are drawn at a fixed size.

## Consequences
- Past weeks show events from the server's retained calendar window (`calendar_past_days`), and no forecast.
- Completed tasks in other weeks appear only if the server's snapshot rules include them (calendar-displayed tasks in the window).
