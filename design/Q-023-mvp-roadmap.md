---
id: Q-023
title: MVP scope & build order
status: open
batch: 3
area: plan
created: 2026-10-23
resolved:
decisions: []
---

# Q-023: MVP scope & build order

## Question
Proposed milestones:
1. **M0 Skeleton:** Rust workspace, Postgres, config import, auth/RBAC, Vue shell, docker-compose, backups.
2. **M1 Dashboard core:** members, weather, calendars (Google/CalDAV/ICS read), dashboard with keep-alive.
3. **M2 Tasks:** lists, recurrence, fixed/rotation/pool, dashboard completion, ntfy.
4. **M3 Email:** Gmail ingest, rules pipeline, bills, school/teacher linking, event proposals and the approval inbox.
5. **M4 LLM:** GPU agent, WoL, queue, enrichment, CPU triage, weather advisories.
6. **M5 Food:** recipes, meal plan, grocery.
7. **M6:** smart assignment, learned routing, TUI.

Do you agree with this order, or should something move earlier (e.g. food or email)?

## Options
- A: accept order
- B: reorder (describe)

## Answer
<!-- answer below this line -->
Option A. Random thought for Email: would it make sense to leverage a System 1 model hosted locally to categorize emails that can't be deterministically categorized first?
