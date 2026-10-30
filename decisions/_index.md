---
title: Decisions Index
updated: 2026-10-26
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
| [D-030](D-030-email-provenance-purge.md) | Email content is transient: purge after full processing, keep provenance links | superseded → D-034 | Q-028 | arch/Domain.md, arch/Platform.md, arch/Overall.md |
| [D-031](D-031-child-login.md) | Children sign in with an adult-set username + simple password; long-lived device session | accepted | Q-029 | arch/Platform.md, arch/Domain.md |
| [D-032](D-032-monorepo-github-actions.md) | Single repo (design + code) on GitHub with GitHub Actions CI | accepted | Q-030 | arch/Platform.md |
| [D-033](D-033-compose-sample-hoster-deploys.md) | Ship a complete docker-compose sample; deployment/update strategy is the hoster's choice | accepted | Q-031 | arch/Platform.md |
| [D-034](D-034-configurable-mail-retention.md) | Email retention is a configurable policy; content kept while it helps local learning; provenance always | accepted | Q-032 | arch/Domain.md, arch/Platform.md, arch/Overall.md |
| [D-035](D-035-agpl-public.md) | Public repository and images under AGPL-3.0-or-later | accepted | Q-033 | arch/Platform.md |
| [D-036](D-036-event-visibility.md) | Show all events; adults' private events appear as Busy | accepted | Q-034 | arch/Feeds.md, arch/Frontend.md, arch/Domain.md |
| [D-037](D-037-recurrence-anchor.md) | Recurrence is per definition: schedule (RRULE) or after-done interval, default after-done | accepted | Q-035 | arch/Tasks.md, arch/Domain.md |
| [D-038](D-038-missed-occurrences.md) | Missed occurrences: configurable on_missed, roll-over by default for after-done tasks | accepted | Q-036 | arch/Tasks.md |
| [D-039](D-039-rotation-rules.md) | Rotation: configurable advancing and skips; member absences as a household feature | accepted | Q-037 | arch/Tasks.md, arch/Domain.md |
| [D-040](D-040-pool-claiming.md) | Pool tasks: check off without claiming; optional claim that expires | accepted | Q-038 | arch/Tasks.md |
| [D-041](D-041-routines.md) | Routines: per-member timed checklists with day filters and a dashboard progress panel | accepted | Q-039 | arch/Tasks.md, arch/Domain.md, arch/Frontend.md |
| [D-042](D-042-now-list.md) | The Now list: deterministic score with configurable weights and size | accepted | Q-040 | arch/Tasks.md, arch/Frontend.md |
| [D-043](D-043-seasonal-weather-tasks.md) | Season windows and weather-triggered tasks: create by default, suggest configurable | accepted | Q-041 | arch/Tasks.md, arch/Feeds.md |
| [D-044](D-044-task-notifications.md) | Task notifications: per-member triggers, quiet hours, children's copies, batching | accepted | Q-042 | arch/Tasks.md, arch/Domain.md |
| [D-045](D-045-task-permissions.md) | Task permissions: adults manage lists and definitions; per-list flags for children and the dashboard | accepted | Q-043 | arch/Tasks.md, arch/Domain.md |
| [D-046](D-046-tasks-in-calendar.md) | Tasks in calendar views: per-task display mode; the agenda is reserved for key dates | accepted | Q-044 | arch/Tasks.md, arch/Frontend.md |
| [D-047](D-047-no-starter-templates.md) | No starter task templates for now | accepted | Q-045 | arch/Tasks.md |
| [D-048](D-048-goals-concept.md) | Goals with progress are a separate concept from tasks (scope pending) | accepted | Q-036 | arch/Overall.md, arch/Tasks.md |
| [D-049](D-049-missed-schedule-default.md) | Missed schedule-anchored tasks auto-skip by default | accepted | Q-046 | arch/Tasks.md |
| [D-050](D-050-roadmap-goals.md) | Roadmap: M6 smart assignment, M7 goals, M8 TUI | accepted | Q-047 | arch/Overall.md, README.md |
| [D-051](D-051-goals-model.md) | Goals: count or quantity measures, weekly cadence with reset, set by adults, progress bars | accepted | Q-048 | arch/Domain.md, arch/Overall.md, arch/Goals.md (future) |
| [D-052](D-052-mail-connectors.md) | Mail connectors: Gmail API and IMAP, chosen per source, default Gmail API | accepted | Q-049 | arch/Ingest.md, arch/Domain.md |
| [D-053](D-053-mail-scope.md) | Each source reads everything except Spam and Trash, narrowed by an optional filter | accepted | Q-050 | arch/Ingest.md |
| [D-054](D-054-mail-categories.md) | Default mail categories and behaviours, all editable | accepted | Q-051 | arch/Ingest.md |
| [D-055](D-055-child-gender-and-tags.md) | Children get an optional gender and free-form relevance tags | accepted | Q-052 | arch/Ingest.md, arch/Domain.md |
| [D-056](D-056-contact-suggestions.md) | New org contacts are suggested by default, configurable per org | accepted | Q-053 | arch/Ingest.md |
| [D-057](D-057-event-auto-add-m3.md) | M3 adds no extracted event automatically; every event waits for approval | accepted | Q-054 | arch/Ingest.md |
| [D-058](D-058-gmail-auto-actions.md) | Automatic mail actions are configurable per category, default labels only | accepted | Q-055 | arch/Ingest.md |
| [D-059](D-059-bills-simple.md) | Bills: pay tasks go to the adult pool, no autopay handling, paid when the task is done | accepted | Q-056 | arch/Ingest.md |
| [D-060](D-060-email-tasks.md) | Email tasks are suggested by default, configurable per category | accepted | Q-057 | arch/Ingest.md |
| [D-061](D-061-approval-inbox-defaults.md) | Approval notifications batched plus digest, configurable per adult; proposals expire by kind | accepted | Q-058 | arch/Ingest.md, arch/Frontend.md |
| [D-062](D-062-labelling-defaults.md) | Labelling covers all four email decision points with a suggested balanced queue | accepted | Q-059 | arch/Ingest.md, arch/Frontend.md |
| [D-063](D-063-mail-visibility.md) | Messages are visible to source owners, or all adults for household sources | accepted | Q-060 | arch/Ingest.md, arch/Domain.md |
| [D-064](D-064-lan-dashboard.md) | Optional dashboard-only listener on the home network; everything else stays tailnet-only | accepted | (user instruction) | arch/Platform.md, arch/Frontend.md |
| [D-065](D-065-simple-onboarding.md) | Simple onboarding: one compose file, optional Tailscale sidecar, optional HTTPS, first-run wizard | accepted |  | arch/Platform.md, arch/Frontend.md, arch/Overall.md, README.md, deploy/README.md |
| [D-066](D-066-m35-shakedown-first.md) | M3.5 is onboarding plus a real-hardware shakedown, before M4 | accepted | Q-061 | arch/Overall.md, plan/M3.5.md, README.md |
| [D-067](D-067-m4-minimal-slice.md) | M4 ships a minimal AI slice first; the registry and platform pieces move to M4.1 | accepted | Q-062 | arch/Overall.md, README.md |
| [D-068](D-068-dashboard-at-a-glance.md) | The dashboard is at-a-glance only; advisories clear themselves and can't be dismissed | accepted | Q-063 | arch/Frontend.md, arch/Feeds.md, arch/Ingest.md, arch/Domain.md |
| [D-069](D-069-trusted-proxies-explicit.md) | Trusted proxies: loopback by default, plus an explicit list that the compose file sets | accepted | Q-064 | arch/Platform.md |
| [D-070](D-070-members-adult-settings.md) | Members are managed on an adult-only Settings screen; children see members on the dashboard | accepted | Q-065 | arch/Frontend.md, arch/Domain.md |
| [D-071](D-071-interests-llm-only.md) | Member interests, hobbies and responsibilities feed LLM features only | accepted | Q-066 | arch/Domain.md, arch/Overall.md |
| [D-072](D-072-http-full-app.md) | Plain HTTP serves the entire app; HTTPS is the hoster's choice | accepted | Q-067 | arch/Platform.md, arch/Frontend.md |
| [D-073](D-073-open-first-run.md) | The first-run wizard is open to the first visitor | accepted | Q-068 | arch/Platform.md, arch/Frontend.md |
| [D-074](D-074-generated-secrets.md) | Secrets are generated on first start; the wizard requires a master-key backup | accepted | Q-069 | arch/Platform.md, arch/Frontend.md |
| [D-075](D-075-settings-in-ui.md) | Every non-infrastructure server setting is editable in Settings, System | accepted | Q-070 | arch/Platform.md, arch/Frontend.md |
| [D-076](D-076-single-container.md) | One app container; no split layout | accepted | Q-071 | arch/Platform.md |
| [D-077](D-077-ghcr-amd64.md) | Release images on GHCR, amd64 only for now | accepted | Q-072 | arch/Platform.md, README.md |
| [D-078](D-078-private-approvals.md) | Approvals are private to the adults who may see the source mail; the dashboard shows only a count per adult | accepted | Q-063 | arch/Ingest.md, arch/Domain.md, arch/Frontend.md |
| [D-079](D-079-theme-catalogue.md) | The app ships a catalogue of popular terminal themes, as built-in data | accepted | Q-073, Q-076 | arch/Frontend.md |
| [D-080](D-080-member-hue-colours.md) | Members choose a named hue; existing members migrate to blue | accepted | Q-074 | arch/Frontend.md, arch/Domain.md |
| [D-081](D-081-theme-per-device.md) | The theme is chosen per device, with an app-shipped default and no household setting | accepted | Q-075 | arch/Frontend.md |
| [D-082](D-082-dashboard-views.md) | The dashboard has a navigation bar with Home and Week views, and the weather panel carries advisories | accepted | (instruction) | arch/Frontend.md |
