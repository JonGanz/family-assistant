---
id: Q-125
title: Settings save and the API restart
status: open
batch: 27
area: tech
created: 2026-11-23
resolved:
decisions: []
---

# Q-125: Settings save and the API restart

## Context
Saving a System setting in `serve all` waits for in-flight jobs, so the API is down for up to 4 minutes (`bins/fa-server/src/serve.rs:392`) (B-01). Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
What restarts when a setting is saved?

## Options
- A. Restart only the API; jobs keep running and pick up settings at their next claim (recommended)
- B. Restart everything, but cancel jobs after a deadline (configurable, default 30 s)
- C. Keep as is and show a banner

## Answer
<!-- answer below this line -->

