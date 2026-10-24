---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-23
drawio: Overall.drawio
drawio_src: src/Overall.py
depends_on: [D-001, D-002, D-003, D-004, D-005, D-006, D-007, D-008, D-009, D-010, D-011, D-012, D-013, D-014, D-015, D-016, D-017, D-018, D-019, D-020]
---

# Overall Architecture (DRAFT v2)

v2 re-derives the design from PROJECT.md and D-001 to D-020. Every Batch 1 and 2 question is now resolved. The open items are Batch 3 (Q-021 to Q-023).

## Scope
**In:** members and roles; Gmail ingest and classification; bills; school, daycare, and teacher linking; calendars (Google, CalDAV, ICS) with event extraction; weather; tasks (recurring, email-derived, weather and season driven); meal plan, recipes, grocery lists; dashboard, PWA, TUI (stretch); notifications.

**Out:** deals (D-012), school portal APIs (D-014), budgeting (D-009), exports to grocery apps (D-015), Alexa/voice (D-011), gamification (D-013).

## Principles
1. **It works without AI, and AI makes it better** (D-001, D-002, D-016).
2. **Members come first, and everything is configured.** Files bootstrap the config, and then the UI owns it (D-004, D-018).
3. **One Rust backend serves thin Vue/ratatui clients** (D-003).
4. **Local and private.** Access is over Tailscale only (D-010).

## Deployment (page "Deployment")
| Node | Always on | Runs |
|---|---|---|
| **Core node**: Ubuntu Server | yes | Docker Compose with `fa-server` (api + worker), `postgres:16` + pgvector, `ntfy`, `backup` to NAS, tailscale, and *optionally* `ollama-cpu` with a model of about 1.5B for triage (D-016). Specs are in **[Q-022]** |
| **GPU node**: Linux gaming PC with RTX 3070 8 GB | no | Ollama with a 7–8B Q4 model and embeddings, plus the `fa-gpu-agent` systemd service. It is woken by WoL on a schedule or when the queue grows, gives way to games, and auto-suspends if the system woke it (D-020) |
| **NAS** | yes | Nightly pg_dump target (D-005) |
| **Echo Show 15** | yes | Silk browser on `/dashboard` with keep-alive (D-011) |
| **Phones and desktops** | n/a | PWA (login per user) and the ntfy Android app |

## Components (page "Components")
| Crate | Responsibility |
|---|---|
| `fa-api` | Axum REST + SSE, sessions, central RBAC policy, device sessions with adult PIN step-up (D-006, D-019) |
| `fa-config` | Versioned TOML schema, import and merge, redacted export (D-018) |
| `fa-worker` | Scheduler and Postgres `SKIP LOCKED` job runner |
| `fa-connectors` | Gmail REST (with history and OAuth re-auth detection), Google Calendar, CalDAV, ICS, Open-Meteo |
| `fa-pipeline` | Email normalise → rules → deterministic extractors (ICS, dates, biller templates, PDF) → member linking → emit events, tasks, and bills. Spam and promos are ignored (D-012) |
| `fa-calendar` | Dedupe, confidence gate, routing (explicit, learned, LLM), approval inbox (D-008) |
| `fa-tasks` | Recurrence, last-done tracking, `fixed`, `rotation`, and `pool` strategies (with `smart` later), completion policy and dashboard override, priority scoring (D-013) |
| `fa-weather` | Snapshots, deterministic `WeatherRule`s, LLM advisories and suggested rules for an adult to review (D-017) |
| `fa-meals` | Recipe import (JSON-LD, microdata, LLM fallback), meal plan horizon, grocery generation and merging (D-015) |
| `fa-llm` | Gateway and router: `triage` goes to CPU Ollama (if enabled) and everything else to the GPU queue. Uses JSON-schema prompts (D-016) |
| `fa-gpu-node` | WoL sender, agent heartbeat state, wake policy (D-020) |
| `fa-notify` | `Notifier` trait with an ntfy implementation (D-010) |
| `fa-domain`, `fa-store` | Entities, DTOs (`ts-rs` to TS), SQLx repositories, encrypted secrets |
| `fa-gpu-agent` | Separate binary on the GPU node: detects gaming, unloads Ollama models, sends heartbeats, auto-suspends |
| `web` | Vue 3 + TS PWA with the `/dashboard` kiosk route (keep-alive, SSE reconnect, SW cache) |
| `fa-tui` | ratatui (stretch) |

## Domain model (draft, will move to Domain.md as a UML class diagram)
- `Household{location, tz, meal_plan_horizon_weeks, dashboard_completion_override, llm_settings}` 1—* `Member{role, avatar, birthdate?, grade?, interests[], hobbies[], responsibilities[], notify_prefs, pin_hash? (adults)}`
- `UserAccount`, `Device{kind: dashboard}`, `AuditEntry{actor, device?, action}`
- `Org{type: school|daycare|club|biller, domains[], senders[], ics_urls[]}` with `OrgContact{role: teacher|coach…, members[]}`
- `MailSource{owners[*], read_only, backfill_days, state}` and `Message{category, confidence, rule_hits[], enrichment, member_links[], extracted{bill?, events[], tasks[]}}`
- `CalendarSource{kind: google|caldav|ics}`, `Calendar{members[]}`, `Event{status: proposed|approved|written, confidence, dedupe_key}`, `RoutingRule{learned}`
- `TaskList{completion_policy: assignee|adult_only}` 1—* `Task{recurrence?, last_done, strategy: fixed|rotation|pool|smart, assignees[], due, priority, origin, steps[]}`
- `Bill{payee, amount, due, task}`
- `Recipe`, `MealPlanEntry{date, slot}`, `GroceryList`/`GroceryItem{source_meals[], manual}` (list model is **[Q-021]**)
- `WeatherSnapshot`, `WeatherRule{condition, window, action, status: active|suggested}`, `Advisory`
- `LlmJob{kind: triage|classify|extract|route|recipe|advisory, backend, status}`, `GpuNodeState`

## Key flows
1. **Email:** poll → rules → extractors → link members → save with `enrichment=pending` if needed → (optional) CPU triage, which can raise an immediate ntfy notification → GPU queue → merge → emit events, tasks, and bills.
2. **Event:** dedupe → route → if over the confidence threshold, write it. Otherwise it goes to the approval inbox. Approving on the dashboard needs an adult PIN. Approvals train the learned rules.
3. **GPU wake:** the schedule or queue triggers WoL → the agent reports `available` → the gateway drains the queue → the queue is empty and the system did the waking → the agent suspends the node. If a game starts, the agent reports `busy_gaming`, models are unloaded, and draining pauses.
4. **Recurring task:** a tick → priority (due date, season, weather rules) → strategy picks who → the Now list → notification.
5. **Dashboard:** tap the item → member picker → completion (no PIN, override on) or approval (adult PIN) → API call as that member with the device recorded.
6. **Weather:** poll → evaluate rules → alerts and tasks. Each season, the LLM creates advisories and suggests rules for an adult to review.
7. **Grocery:** pick upcoming meals → normalise and merge ingredients → list. Manual items can be added from anywhere.

## Next designs
`Domain.md` (class diagram), `Ingest.md` (email and calendar pipeline, with sequence diagrams), `Tasks.md`, and `Frontend.md` (dashboard and PWA). The order depends on **[Q-023]**.
