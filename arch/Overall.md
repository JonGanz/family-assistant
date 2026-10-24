---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-23
drawio: Overall.drawio
depends_on: [D-001, D-002, D-003, D-004, D-005, D-006, D-007, D-008, D-009, D-010]
---

# Overall Architecture (DRAFT v1)

v1 re-derives the design from PROJECT.md and D-001 to D-010. Choices that still depend on an open question are tagged like **[Q-0xx]**.

## Principles
1. **It works without AI, and AI makes it better** (D-001, D-002). Each feature has a deterministic path. LLM enrichment is queued and runs when the GPU node is available.
2. **Members come first, and everything is configured** (D-004). People, sources, schools, and rules are runtime data. No PII or secrets in the repo.
3. **One Rust backend serves thin clients** (D-003): a Vue PWA, the dashboard route, and a ratatui TUI.
4. **Local and private** (D-010). Access is over Tailscale only. The only outbound traffic goes to data sources such as Gmail, calendars, Open-Meteo, and recipe sites.

## Deployment (UML deployment view, page "Deployment" in Overall.drawio)
| Node | Always on | Runs |
|---|---|---|
| **Core node**: Ubuntu Server | yes | Docker Compose with `fa-server` (Rust binary, `api` + `worker` modes), `postgres:16` + pgvector, `ntfy`, `backup` (nightly pg_dump to the NAS), tailscale |
| **GPU node**: gaming PC with RTX 3070 8 GB | no | Ollama (about 7–8B Q4 instruct model and an embedding model). Reached over the tailnet or LAN. Wake and schedule policy is **[Q-019]** |
| **NAS** | yes | Backup target (D-005) |
| **Clients** | n/a | Echo Show 15 (`/dashboard`, **[Q-010]**), Android/desktop browsers (PWA), terminal TUI |

## Components (UML component view, page "Components")
Rust workspace crates. Each one is a component.

| Crate / component | Responsibility | Key libs |
|---|---|---|
| `fa-api` | HTTP + SSE API, session auth, RBAC policy layer, OpenAPI | axum, tower-sessions, argon2, utoipa |
| `fa-worker` | Scheduler and job runner (Postgres `SKIP LOCKED` queue). Polls sources and runs task ticks | tokio, sqlx |
| `fa-domain` | Entities, policies, shared DTOs (exported to TS via `ts-rs`) | serde |
| `fa-connectors` | `MailConnector` (Gmail REST), `CalendarConnector` (Google, CalDAV, ICS), `WeatherConnector` (Open-Meteo) | reqwest, oauth2, quick-xml, icalendar |
| `fa-pipeline` | Email: normalise → rules → extract → link members → emit (events, tasks, bills, deals). Records enrichment state | regex, rule DSL (YAML/TOML), pdf-extract |
| `fa-llm` | LLM gateway: GPU node health, queue draining, JSON-schema prompts, model registry | reqwest (Ollama HTTP API) |
| `fa-tasks` | Recurrence (RRULE), last-done tracking, assignment strategies, priority scoring (due date, season, weather) **[Q-012]** | rrule |
| `fa-calendar` | Event dedupe, confidence gate, target routing (explicit, learned, LLM), approval inbox (D-008) | |
| `fa-meals` | Recipe import (JSON-LD/microdata, with LLM fallback), meal plan, grocery list builder **[Q-014]** | scraper |
| `fa-notify` | `Notifier` trait with an ntfy implementation and a per-member topic and rule set (D-010) | reqwest |
| `fa-store` | SQLx repositories, migrations, encrypted secret storage | sqlx, aes-gcm |
| `web` (Vue 3 + TS) | PWA (login per user) and the `/dashboard` kiosk route (device token, member picker) | vite, pinia, vue-router, vite-plugin-pwa |
| `fa-tui` (stretch) | Terminal client | ratatui |

## Core domain model (draft. To be split into Domain.md and a UML class diagram)
- `Household` 1—* `Member{role: adult|child, display, avatar, birthdate?, grade?, org_links[], interests[], hobbies[], responsibilities[], notify_prefs}`
- `UserAccount{member, password_hash}` and `Device{kind: dashboard, token}` (D-006)
- `Org{type: school|daycare|club|biller, domains[], senders[]}` and `OrgContact{org, person, role: teacher|coach..., members[]}`. Together these map a teacher to a child.
- `MailSource{gmail, owners: Member[*], read_only=true, backfill_days, state: ok|needs_reauth}` (D-007)
- `Message{source, headers, category, confidence, rule_hits[], enrichment, member_links[], extracted{bill?, events[], tasks[], deals[]}}`
- `CalendarSource{kind: google|caldav|ics, writable}` 1—* `Calendar{members[]}`
- `Event{calendar?, members[], origin: synced|extracted, confidence, status: proposed|approved|written, dedupe_key}` and `RoutingRule{match, target_calendar, learned: bool, hits}`
- `Task{list, recurrence?, last_done, strategy: fixed|rotation|pool|smart, assignees[], due, priority, origin: manual|email|system, steps[]}`
- `Bill{payee, amount, due, message, task}` (D-009)
- `Recipe`, `MealPlanEntry`, `GroceryList`, `WeatherSnapshot`
- `LlmJob{kind, subject_ref, schema, status, attempts}` and `AuditEntry{actor_member, device?, action}`

## Key flows
1. **Email ingest** (D-002, D-007, D-009): the worker polls Gmail history → normalise → **rule stage**, which uses Gmail category labels, List-Unsubscribe, Org sender and domain maps, and keyword rules → deterministic extractors (ICS attachments, date and time patterns, biller templates, PDF text) → link members (owners, OrgContact, grade or age terms) → write the results with `enrichment=pending` if the rules weren't confident → `LlmJob` queued → the GPU node drains the queue later → the results are merged and may raise confidence → events and tasks are emitted. If `read_only=false`, the configured Gmail actions run.
2. **Event from email** (D-008): candidate → dedupe across all linked calendars → routing (explicit, then learned, then LLM) → if confidence ≥ the threshold, write to the target calendar. Otherwise it goes to the approval inbox and a notification is sent. Each approval updates the learned rules.
3. **Recurring task**: a worker tick evaluates recurrence and last_done → priority score → assignment strategy → the "Now" list → a notification if the task is high priority.
4. **Dashboard action** (D-006): device session → tap the task → member picker (PIN **[Q-018]**) → API call made as that member, with the device recorded in the audit log.
5. **OAuth expiry** (D-007): `invalid_grant` → the source is set to `needs_reauth` → an ntfy notification is sent to the source owners with a re-auth link.
6. **Recipe import**: URL → fetch → JSON-LD `Recipe` → microdata → LLM fallback job → ingredient normaliser → stored.

## Configuration (D-004)
How config is bootstrapped and managed is **[Q-017]**. Either way, secrets stay out of the repo and are kept encrypted in Postgres with a key outside the repo.

## Open dependencies
Q-010 to Q-016 (Batch 1, still open) and Q-017 to Q-020 (Batch 2). See `design/_index.md`.
