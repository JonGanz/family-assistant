---
id: D-082
title: The dashboard has a navigation bar with Home and Week views, and the weather panel carries advisories
status: accepted
date: 2026-10-29
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-082: The dashboard has a navigation bar with Home and Week views

## Context
Direct user instruction (2026-10-29, no question): the dashboard should be more interactive. Layouts were selectable, but only one at a time. The user also asked that advisories be combined into the weather component.

## Decision
- The dashboard shows a **navigation bar** along its shorter edge: on the **left** in landscape, along the **top** in portrait (the device's `orientation` setting decides).
- Views: **Home** (the configured panels: clock, weather, agenda, tasks, routines) and **Week** (a table with a row per day and a column per person; each cell holds that person's calendar events (colour bar, bold title, time beneath) and the tasks due that day; household-wide events and unassigned tasks have no column and are not shown. The day cell shows the day of the month (bold), the weekday, and that day's forecast icon, high and low. The table fills the view, and the footer (heartbeat, adult unlock) is hidden on this view). The view is local to the screen and not stored.
- The dashboard always fetches a full week of events, so the Week view works with any layout.
- **Advisories live in the weather panel** (beside "feels like / wind"), with a derived notice for an upcoming thunderstorm. There is no separate `advisories` panel kind.
- Panels take the height they need and the **view scrolls** instead of cropping a panel.
- The weather panel shows today's forecast (high, low, rain chance) before noon in the household time zone, and only the current conditions after noon. The hourly and multi-day strips stay.

## Consequences
- Per-device layout settings (panels, presets, agenda days, theme) still shape the Home view. The Week view is not configurable in this first pass.
- A saved layout that names `advisories` ignores it, like any unknown kind.
