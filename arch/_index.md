---
title: Architecture Index
updated: 2026-10-26
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, roadmap (D-001 to D-077) | draft v8 (M3.5, M4 slice, M4.1) | [Overall.drawio](Overall.drawio) |
| [Domain](Domain.md) | UML class model by package, persistence conventions, RBAC matrix | draft v5.1 (RBAC: D-068, D-070, D-071) | [Domain.drawio](Domain.drawio): 3 pages |
| [Platform](Platform.md) | Workspace layout, libs, API, security, jobs, deployment, backups, dev, M0 DoD | draft v4 (M3.5: compose, optional Tailscale sidecar, HTTP default, first run, runtime settings) | [Platform.drawio](Platform.drawio): Packages, Deployment, Security |
| [Feeds](Feeds.md) | Calendar read (Google, CalDAV, ICS), OAuth accounts, weather, rule DSL, advisories, ChangeBus | draft v1.1 (advisories clear themselves, D-068) | [Feeds.drawio](Feeds.drawio): Components, Model, Sync sequence |
| [Tasks](Tasks.md) | Task lists, recurrence, fixed/rotation/pool, absences, completion, Now list, routines, weather tasks, calendar display, ntfy | draft v1.1 (M2, D-037 to D-051) | [Tasks.drawio](Tasks.drawio): Components, Model, Complete sequence |
| [Ingest](Ingest.md) | Mail sync (Gmail API, IMAP), rules pipeline engine, classification and extraction, orgs/contacts/enrollments, bills, event dedupe/routing/writes, approval inbox, email tasks, labelling view | draft v1.1 (inbox app-only, D-068) | [Ingest.drawio](Ingest.drawio): Components, Model, Process sequence |
| [Frontend](Frontend.md) | Vue PWA structure, routes, dashboard layout model, settings screens, live data flow | draft v4 (setup wizard, grouped Settings, at-a-glance dashboard) | [Frontend.drawio](Frontend.drawio): Components, Clients, Live update |

Planned next, in roadmap order (D-023, D-066, D-067): M4 → `AI.md` (minimal slice; registry, sysprobe, GPU agent and eval in M4.1); M5 → `Meals.md`; M7 → `Goals.md` (D-050).
