---
title: Architecture Index
updated: 2026-11-07
---

# Architecture

Each design is a pair: `X.md` and `X.drawio` (UML 2). The diagram is **generated** from `src/X.py` using `tools/drawio.py`. Edit the source, run `python3 arch/src/X.py`, and never hand-edit the .drawio file.

| Design | Scope | Status | Diagram |
|---|---|---|---|
| [Overall](Overall.md) | Whole system: scope, deployment, components, roadmap (D-001 to D-097) | draft v8.1 (M4 two tracks and LLM slice, D-093 to D-097) | [Overall.drawio](Overall.drawio) |
| [Domain](Domain.md) | UML class model by package, persistence conventions, RBAC matrix | draft v5.2 (M5 Meals package and RBAC rows, D-109 to D-116; v5.1 RBAC: D-068, D-070, D-071) | [Domain.drawio](Domain.drawio): 3 pages |
| [Platform](Platform.md) | Workspace layout, libs, API, security, jobs, deployment, backups, dev, M0 DoD | draft v4.2 (D-102 ntfy generic text; v4.1 M4: fa-llm, fa-recipe, fa-eval, LLM endpoint security; v4 M3.5: compose, optional Tailscale sidecar, HTTP default, first run, runtime settings) | [Platform.drawio](Platform.drawio): Packages, Deployment, Security |
| [Feeds](Feeds.md) | Calendar read (Google, CalDAV, ICS), OAuth accounts, weather, rule DSL, advisories, ChangeBus | draft v1.2 (D-099 CalDAV host scope; v1.1 advisories clear themselves, D-068) | [Feeds.drawio](Feeds.drawio): Components, Model, Sync sequence |
| [Tasks](Tasks.md) | Task lists, recurrence, fixed/rotation/pool, absences, completion, Now list, routines, weather tasks, calendar display, ntfy | draft v1.2 (D-098 mail-task visibility, D-102 ntfy generic text; v1.1 M2) | [Tasks.drawio](Tasks.drawio): Components, Model, Complete sequence |
| [Ingest](Ingest.md) | Mail sync (Gmail API, IMAP), rules pipeline engine, classification and extraction, orgs/contacts/enrollments, bills, event dedupe/routing/writes, approval inbox, email tasks, labelling view | draft v1.3 (D-098, D-100, D-101: task visibility, IMAP PEM, size limit; v1.2 M4 `llm` stage, LLM extraction, ai_settings; v1.1 inbox app-only) | [Ingest.drawio](Ingest.drawio): Components, Model, Process sequence |
| [Frontend](Frontend.md) | Vue PWA structure, design system (ui/ library, tokens, icons), navigation, apply-changes rule, routes, dashboard layout model, settings screens, live data flow | draft v5.2 (M5 Food views, kiosk Food view, `meals` panel, grocery offline queue, D-109 to D-118; v5.1 D-103 to D-106; v5 M4 design system) | [Frontend.drawio](Frontend.drawio): Components, Clients, Live update |
| [AI](AI.md) | System 2 over an OpenAI-compatible endpoint, fa-llm and Provider, `llm` pipeline stage and `mail.llm` job, recipe extraction, eval, llm_call metrics; M4.1 roadmap | draft v1.1 (D-104 confirmations; v1 M4, D-093, D-094) | [AI.drawio](AI.drawio): Components, Deployment, Sequence |
| [Meals](Meals.md) | Recipes and import, ingredient catalogue and normaliser, meal plan, grocery generation, the living list with the staples check, dashboard panels, API, RBAC (M5) | draft v1.1 (D-015, D-021, D-108 to D-122, R-007) | [Meals.drawio](Meals.drawio): Components, Model, Generate sequence |

Planned next, in roadmap order (D-023, D-066, D-094, D-108): M5 → `Meals.md` (draft v1.1); M4.1 → `AI.md` update (System One, registry, sysprobe, GPU agent, full eval); M7 → `Goals.md` (D-050).
