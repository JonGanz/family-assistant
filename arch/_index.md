---
title: Architecture Index
updated: 2026-10-23
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, domain draft (D-001 to D-020) | draft v2 | [Overall.drawio](Overall.drawio) |

Planned next (order set by Q-023): `Domain.md` (UML class), `Ingest.md` (email/calendar sequences), `Tasks.md`, `Frontend.md` (dashboard/PWA), `Security.md` (RBAC, PIN, secrets).
