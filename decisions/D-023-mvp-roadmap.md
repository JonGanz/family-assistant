---
id: D-023
title: MVP milestones and build order
status: accepted
date: 2026-10-23
source_questions: [Q-023]
supersedes: 
superseded_by:
affects: [arch/Overall.md, all future arch docs]
---

# D-023: MVP milestones and build order

## Context
The user accepted the proposed order (option A).

## Decision
1. **M0 Skeleton:** Rust workspace, Postgres and migrations, config import/export, auth and RBAC, device sessions, Vue shell, docker-compose, backups.
2. **M1 Dashboard core:** members, weather (with rules), calendar read (Google, CalDAV, ICS), dashboard with keep-alive.
3. **M2 Tasks:** lists, recurrence, fixed/rotation/pool strategies, dashboard completion, ntfy.
4. **M3 Email:** Gmail ingest, rules pipeline, bills, org and teacher linking, event proposals, approval inbox, 'Fix category'.
5. **M4 AI:** System 1 (D-022), GPU agent, WoL, LLM queue, System 2 enrichment, weather advisories.
6. **M5 Food:** recipes, meal plan, living grocery list (D-021).
7. **M6:** smart assignment, learned calendar routing, TUI.

## Consequences
- The detailed arch docs are written in milestone order. M0 needs `Domain.md` (class model) and `Platform.md` (workspace, security, deployment).
- Each milestone ends with a demo on the real hardware.

## Alternatives considered
Food or email earlier. Not chosen.
