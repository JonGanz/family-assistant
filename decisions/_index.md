---
title: Decisions Index
updated: 2026-10-23
---

# Decisions

Each record is `D-NNN-slug.md`. Superseded decisions stay here and are marked with `status: superseded`.

| ID | Title | Status | Source Qs | Affects |
|---|---|---|---|---|
| [D-001](D-001-local-first-deterministic-first.md) | Local-first, deterministic-first processing | accepted | PROJECT.md | Overall |
| [D-002](D-002-hosting-split.md) | Split hosting: always-on server + on-demand GPU node; LLM is an enhancement | accepted | Q-001 | arch/Overall.md |
| [D-003](D-003-rust-vue-stack.md) | Backend in Rust (Axum/Tokio/SQLx); frontend Vue 3 + TypeScript | accepted | Q-002 | arch/Overall.md |
| [D-004](D-004-config-driven-household.md) | Household data is configuration, never in the repo; multi-family capable | accepted | Q-003 | arch/Overall.md |
| [D-005](D-005-postgres-nas-backups.md) | PostgreSQL 16 + pgvector; nightly backups to NAS | accepted | Q-004 | arch/Overall.md |
| [D-006](D-006-auth-rbac-dashboard.md) | Per-user logins with role-based access; shared dashboard with per-action member selection | accepted | Q-005 | arch/Overall.md, arch/Frontend.md (future) |
| [D-007](D-007-email-accounts-model.md) | Multiple Gmail accounts mapped to members; read-only by default with opt-in actions | accepted | Q-006 | arch/Overall.md, arch/Ingest.md (future) |
| [D-008](D-008-calendar-sources-routing.md) | Multiple calendar sources; confidence-gated auto-add; configurable + learned target routing | accepted | Q-007 | arch/Overall.md, arch/Ingest.md (future) |
| [D-009](D-009-bills-itemise-only.md) | Bills: itemise + create pay task only | accepted | Q-008 | arch/Overall.md |
| [D-010](D-010-tailscale-ntfy.md) | Remote access via Tailscale; Android notifications via self-hosted ntfy | accepted | Q-009 | arch/Overall.md |
| [D-011](D-011-echo-show-silk-dashboard.md) | Echo Show 15 dashboard via Silk browser with keep-alive; touch-enabled, no Alexa integration | accepted | Q-010 | arch/Overall.md, arch/Frontend.md (future) |
| [D-012](D-012-deals-out-of-scope.md) | Deals of interest removed from scope | accepted | Q-011 | arch/Overall.md |
| [D-013](D-013-task-assignment-completion.md) | Task assignment strategies and per-list completion policy | accepted | Q-012 | arch/Overall.md, arch/Tasks.md (future) |
| [D-014](D-014-no-school-portal-apis.md) | No school/daycare portal integrations; ICS feeds + email only | accepted | Q-013 | arch/Overall.md |
| [D-015](D-015-meal-plan-grocery.md) | Configurable meal-plan horizon; grocery lists generated from selected meals; dashboard can add items | accepted | Q-014 | arch/Overall.md |
| [D-016](D-016-batched-llm-cpu-triage.md) | LLM work is batched; optional small CPU triage model on the core node | superseded → D-022 | Q-015, Q-020 | arch/Overall.md |
| [D-017](D-017-weather-rules-advisories.md) | Weather: configurable location, deterministic threshold rules, LLM-suggested advisories | accepted | Q-016 | arch/Overall.md |
| [D-018](D-018-hybrid-config.md) | Hybrid configuration: file bootstrap/import-export, UI is source of truth thereafter | accepted | Q-017 | arch/Overall.md |
| [D-019](D-019-dashboard-pin-policy.md) | Dashboard PIN policy | accepted | Q-018 | arch/Overall.md |
| [D-020](D-020-gpu-node-agent.md) | GPU node (Linux): Wake-on-LAN, auto-wake by schedule/queue depth, yields to gaming via agent | accepted | Q-019 | arch/Overall.md |
| [D-021](D-021-living-grocery-list.md) | One living grocery list with a pantry-staples check section | accepted | Q-021 | arch/Overall.md |
| [D-022](D-022-system1-system2-classification.md) | Tiered email classification: rules → System 1 (local, always-on) → System 2 (GPU, batched) | superseded → D-024 | Q-022, Q-023 | arch/Overall.md, arch/Ingest.md (future) |
| [D-023](D-023-mvp-roadmap.md) | MVP milestones and build order | accepted | Q-023 | arch/Overall.md, all future arch docs |
| [D-024](D-024-system-one-decision-tier.md) | System 1 tier = System One decision models (pluggable DecisionBackend) | accepted | Q-023 (clarified), R-001 | arch/Overall.md |
| [D-025](D-025-python-model-sidecars.md) | Exception to D-003: third-party model servers may run as Python containers | accepted | Q-024 | arch/Overall.md |
| [D-026](D-026-hardware-adaptive-backends.md) | Hardware-adaptive model placement via a backend registry | accepted | Q-025 | arch/Overall.md, arch/AI.md (future) |
| [D-027](D-027-system-one-scope.md) | System One decision points: category, time-sensitivity, member linking, calendar routing, task urgency, bill detection | accepted | Q-026 | arch/Overall.md |
| [D-028](D-028-knn-and-eval-sample.md) | Keep the kNN correction-learning stage; build a ~200-email labelled eval sample | accepted | Q-027 | arch/Overall.md |
| [D-029](D-029-configurable-decision-pipeline.md) | Decision pipelines are data, fully configurable in a settings UI | accepted | Q-027 | arch/Overall.md, arch/AI.md (future), arch/Frontend.md (future) |
| [D-030](D-030-email-provenance-purge.md) | Email content is transient: purge after full processing, keep provenance links | accepted | Q-028 | arch/Domain.md, arch/Platform.md, arch/Overall.md |
| [D-031](D-031-child-login.md) | Children sign in with an adult-set username + simple password; long-lived device session | accepted | Q-029 | arch/Platform.md, arch/Domain.md |
| [D-032](D-032-monorepo-github-actions.md) | Single repo (design + code) on GitHub with GitHub Actions CI | accepted | Q-030 | arch/Platform.md |
| [D-033](D-033-compose-sample-hoster-deploys.md) | Ship a complete docker-compose sample; deployment/update strategy is the hoster's choice | accepted | Q-031 | arch/Platform.md |
