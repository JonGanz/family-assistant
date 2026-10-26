---
title: Architecture Index
updated: 2026-10-25
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, roadmap (D-001 to D-035) | draft v7 | [Overall.drawio](Overall.drawio) |
| [Domain](Domain.md) | UML class model by package, persistence conventions, RBAC matrix | draft v4 (M2 tasks) | [Domain.drawio](Domain.drawio): 3 pages |
| [Platform](Platform.md) | Workspace layout, libs, API, security, jobs, deployment, backups, dev, M0 DoD | draft v3 (M0) | [Platform.drawio](Platform.drawio): Packages, Deployment, Security |
| [Feeds](Feeds.md) | Calendar read (Google, CalDAV, ICS), OAuth accounts, weather, rule DSL, advisories, ChangeBus | draft v1 (M1) | [Feeds.drawio](Feeds.drawio): Components, Model, Sync sequence |
| [Tasks](Tasks.md) | Task lists, recurrence, fixed/rotation/pool, absences, completion, Now list, routines, weather tasks, calendar display, ntfy | draft v1.1 (M2, D-037 to D-051) | [Tasks.drawio](Tasks.drawio): Components, Model, Complete sequence |
| [Ingest](Ingest.md) | Gmail sync, rules pipeline engine, classification and extraction, orgs/contacts/enrollments, bills, event dedupe/routing/writes, approval inbox, email tasks, labelling view | draft v0 (M3, batch 16 open) | [Ingest.drawio](Ingest.drawio): Components, Model, Process sequence |
| [Frontend](Frontend.md) | Vue PWA structure, routes, dashboard layout model, settings screens, live data flow | draft v2 (M2) | [Frontend.drawio](Frontend.drawio): Components, Clients, Live update |

Planned next, in roadmap order (D-023): M4 → `AI.md` (registry, pipelines, eval); M5 → `Meals.md`; M7 → `Goals.md` (D-050).
