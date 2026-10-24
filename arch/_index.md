---
title: Architecture Index
updated: 2026-10-23
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, roadmap (D-001 to D-029) | draft v5 | [Overall.drawio](Overall.drawio) |
| [Domain](Domain.md) | UML class model by package, persistence conventions, RBAC matrix | draft v1 (M0) | [Domain.drawio](Domain.drawio): 3 pages |
| [Platform](Platform.md) | Workspace layout, libs, API, security, jobs, deployment, backups, dev, M0 DoD | draft v1 (M0) | [Platform.drawio](Platform.drawio): Packages, Deployment, Security |

Planned next, in roadmap order (D-023): M1 → `Frontend.md`; M2 → `Tasks.md`; M3 → `Ingest.md`; M4 → `AI.md` (registry, pipelines, eval); M5 → `Meals.md`.
