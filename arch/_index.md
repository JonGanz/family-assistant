---
title: Architecture Index
updated: 2026-10-23
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, domain draft, roadmap (D-001 to D-023) | draft v3 | [Overall.drawio](Overall.drawio) |

Planned next, in roadmap order (D-023): M0 → `Domain.md` (UML class) + `Platform.md` (workspace, security, deployment); M1 → `Frontend.md`; M2 → `Tasks.md`; M3 → `Ingest.md`; M4 → `AI.md`; M5 → `Meals.md`.
