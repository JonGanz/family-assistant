---
id: Q-041
title: Seasonal and weather-triggered tasks
status: open
batch: 14
area: domain
created: 2026-10-24
resolved:
decisions: []
---

# Q-041: Seasonal and weather-triggered tasks

## Question
PROJECT.md wants tasks managed by time of year and weather. M1 already lets a `WeatherRule` have the action `task` (Feeds.md); M2 makes it fire. Seasonal chores (clean gutters in autumn, mow weekly from April to October) need a **season window**.

**How should these work?**

## Options
- A. Weather rules create tasks automatically. Season windows are month ranges on a definition (outside the window, no occurrences).
- B. Weather and season only **suggest** tasks: they appear in a "Suggested" section until an adult accepts them.
- C. **Configurable per rule or definition, default A** (recommended): weather rules have `task_mode: create | suggest` (default `create`), and a season window is a month/day range with an optional weather condition ("mow, April–October, not if rain > 5 mm today").
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C. 
