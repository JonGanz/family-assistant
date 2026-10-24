---
title: Design Questions Index
updated: 2026-10-23
---

# Design Questions

Open questions live here. Resolved ones move to `resolved/`.

## Template
```
---
id: Q-NNN
title: ...
status: open
batch: N
area: infra|tech|domain|ingest|frontend|security
created: YYYY-MM-DD
resolved:
decisions: []
---
# Q-NNN: title
## Context
## Question
## Options (optional)
## Answer
<!-- answer below this line -->
```

## Batch 1 (2026-10-23), still open
| ID | Title | Area |
|---|---|---|
| [Q-010](Q-010-echo.md) | Echo Show 15 kiosk approach | frontend |
| [Q-011](Q-011-deals.md) | Deals of interest from spam/promotions | domain |
| [Q-012](Q-012-tasks.md) | Task assignment & accountability | domain |
| [Q-013](Q-013-school.md) | School / daycare portals | ingest |
| [Q-014](Q-014-food.md) | Meal planning & grocery details | domain |
| [Q-015](Q-015-llm-latency.md) | Acceptable LLM latency & batching (largely settled by D-002; confirm) | tech |
| [Q-016](Q-016-weather.md) | Weather location & use | domain |

## Batch 2 (2026-10-23), open
| ID | Title | Area |
|---|---|---|
| [Q-017](Q-017-config-source.md) | Configuration source of truth | domain |
| [Q-018](Q-018-dashboard-identity.md) | Dashboard member selection security | security |
| [Q-019](Q-019-gpu-node-availability.md) | GPU node wake / schedule policy | infra |
| [Q-020](Q-020-core-node-specs.md) | Core node hardware & CPU fallback model | infra |

## Resolved
| ID | Title | Decisions |
|---|---|---|
| [Q-001](resolved/Q-001-hosting.md) | Host machine & uptime | D-002 |
| [Q-002](resolved/Q-002-stack.md) | Language / framework preferences | D-003 |
| [Q-003](resolved/Q-003-family.md) | Family composition | D-004 |
| [Q-004](resolved/Q-004-datastore.md) | Datastore & backups | D-005 |
| [Q-005](resolved/Q-005-auth.md) | Users, login & privacy | D-006 |
| [Q-006](resolved/Q-006-email.md) | Email accounts & processing scope | D-007 |
| [Q-007](resolved/Q-007-calendar.md) | Calendar sources & write-back target | D-008 |
| [Q-008](resolved/Q-008-bills.md) | Bill handling depth | D-009 |
| [Q-009](resolved/Q-009-access.md) | Remote access & notifications | D-010 |
