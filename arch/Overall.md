---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-23
drawio: Overall.drawio
drawio_src: src/Overall.py
depends_on: [D-001, D-002, D-003, D-004, D-005, D-006, D-007, D-008, D-009, D-010, D-011, D-012, D-013, D-014, D-015, D-017, D-018, D-019, D-020, D-021, D-023, D-024]
research: [R-001]
---

# Overall Architecture (DRAFT v4)

v4 re-derives the design from PROJECT.md and all accepted decisions (D-001 to D-024; D-016 → D-022 → D-024 superseded). System 1 is now a **System One decision model** (R-001). Open: Batch 4 (Q-024 to Q-027) on its runtime, placement, scope, and the kNN layer.

## Scope
**In:** members and roles; Gmail ingest and classification; bills; school, daycare, and teacher linking; calendars (Google, CalDAV, ICS) with event extraction; weather with rules and advisories; tasks (recurring, email-derived, weather and season driven); meal plan, recipes, the living grocery list; dashboard, PWA, TUI (stretch); ntfy notifications.

**Out:** deals (D-012), school portal APIs (D-014), budgeting (D-009), grocery exports (D-015), Alexa/voice (D-011), gamification (D-013), dietary preferences (deferred in D-021).

## Principles
1. **It works without AI, and AI makes it better.** The intelligence comes in tiers: **rules → System 1 (System One decision models: typed answers with probabilities) → System 2 (generative LLM for extraction)** (D-001, D-002, D-024). Each tier handles only what the one before it left undecided.
2. **Members come first, and everything is configured.** Files bootstrap the config, the UI owns it after that, and there is no PII in the repo (D-004, D-018).
3. **One Rust backend serves thin Vue/ratatui clients** (D-003).
4. **Local and private.** Access is over Tailscale only (D-010).
5. **If in doubt, make it a toggle.** Optional and hardware-dependent features have a switch, and the system suggests a sensible default.

## Deployment (page "Deployment")
| Node | Always on | Runs |
|---|---|---|
| **Core node**: Ubuntu Server, same LAN as the GPU node | yes | Docker Compose with `fa-server` (api + worker), `postgres:16` + pgvector, `ntfy`, `backup` to NAS, tailscale, **`s1-decider`** (a containerized Python sidecar serving a System One model over `/v1/systemone`, **[Q-024]**/**[Q-025]**), and an optional embedding model for kNN **[Q-027]**. Toggle on; profile suggested by the hardware probe (D-024) |
| **GPU node**: Linux gaming PC with RTX 3070 8 GB | no | System 2: Ollama with a 7–8B Q4 model, plus possibly a larger System One model **[Q-025]**, plus `fa-gpu-agent`. It is woken by WoL, gives way to games, and auto-suspends (D-020) |
| **NAS** | yes | Nightly pg_dump (D-005) |
| **Echo Show 15** | yes | Silk browser on `/dashboard` with keep-alive (D-011) |
| **Phones and desktops** | n/a | PWA (login per user) and the ntfy app |

## Components (page "Components")
| Crate | Responsibility |
|---|---|
| `fa-api` | Axum REST + SSE, sessions, central RBAC, device sessions with adult PIN step-up (D-006, D-019) |
| `fa-config` | Versioned TOML schema, import and merge, redacted export (D-018) |
| `fa-worker` | Scheduler and Postgres `SKIP LOCKED` job runner |
| `fa-connectors` | Gmail, Google Calendar, CalDAV, ICS, Open-Meteo |
| `fa-pipeline` | Email: normalise → **rules** → (kNN **[Q-027]**) → **System One decisions** (category, time-sensitive, and more **[Q-026]**) → escalate to **System 2** for low confidence and for all extraction → emit events, tasks, and bills. Handles 'Fix category' feedback |
| `fa-decide` | `DecisionBackend` trait with a `/v1/systemone` HTTP client: `choice`, `score`, and `noul` calls. One question per request, stable option order. Backends are selected by config (D-024) |
| `fa-eval` | Replays labelled household email against candidate backends and reports accuracy, calibration, and latency. Drives model choice (D-024) |
| `fa-llm` | System 2 gateway: GPU queue for extraction, routing hints, recipes, and advisories. JSON-schema prompts. Also the embeddings and pgvector kNN if Q-027 keeps them |
| `fa-sysprobe` | Probes RAM and CPU at start-up and suggests a System 1 profile (e.g. `off`, `small-cpu` such as GLiNER2.5-Decide, or `gpu-only`) (D-024) |
| `fa-calendar` | Dedupe, confidence gate, routing (explicit, learned, LLM), approval inbox (D-008) |
| `fa-tasks` | Recurrence, fixed/rotation/pool strategies (with smart later), completion policy and dashboard override, priority (D-013) |
| `fa-weather` | Snapshots, `WeatherRule`s, advisories and suggested rules (D-017) |
| `fa-meals` | Recipe import, meal plan horizon, generation into the living list, staples check (D-015, D-021) |
| `fa-gpu-node` | WoL and agent state (D-020) |
| `fa-notify` | `Notifier` trait with an ntfy implementation (D-010) |
| `fa-domain`, `fa-store` | Entities, DTOs (`ts-rs`), SQLx repositories, encrypted secrets |
| `fa-gpu-agent` | Separate binary on the GPU node |
| `web` | Vue 3 + TS PWA and the `/dashboard` kiosk route |
| `fa-tui` | ratatui (stretch) |

## Domain model (draft, moves to Domain.md in M0)
- `Household{location, tz, meal_plan_horizon_weeks, dashboard_completion_override, ai_settings{system1{enabled, profile, accept_threshold}, system2{wake_policy}}}` 1—* `Member{role, avatar, birthdate?, grade?, interests[], hobbies[], responsibilities[], notify_prefs, pin_hash?}`
- `UserAccount`, `Device{kind: dashboard}`, `AuditEntry{actor, device?, action}`
- `Org{type, domains[], senders[], ics_urls[]}` and `OrgContact{role, members[]}`
- `MailSource{owners[*], read_only, backfill_days, state}`
- `Message{category, confidence, decided_by: rule|knn|s1|s2|user, enrichment, embedding?, member_links[], extracted{bill?, events[], tasks[]}}` and `CategoryLabel{message, category, source: rule|s2|user}`, which is the kNN training set
- `CalendarSource`, `Calendar`, `Event{status, confidence, dedupe_key}`, `RoutingRule{learned}`
- `TaskList{completion_policy}` 1—* `Task{recurrence?, last_done, strategy, assignees[], due, priority, origin, steps[]}`, and `Bill`
- `Recipe`, `Ingredient{name, is_staple}`, `MealPlanEntry`, `GroceryList` (one per household) 1—* `GroceryItem{ingredient?, text, qty, source_meals[], manual, checked_at?}`
- `WeatherSnapshot`, `WeatherRule{status: active|suggested}`, `Advisory`
- `Decision{subject_ref, question_kind: choice|score|noul, options[], probabilities[], backend, model}`, `LlmJob{kind, status}` (S2 only), `EvalRun{backend, model, accuracy, ece, p50_ms}`, `GpuNodeState`, `SysProbe{ram_total, ram_avail, cores, suggested_profile}`

## Key flows
1. **Email cascade:** poll → rules settle it? If not: (**kNN** if kept, accepted if the neighbours agree) → **System One** `Choice(category)` + `Noul(time_sensitive)`. If the probability is ≥ the threshold, accept (a time-sensitive result sends a notification now). Otherwise **S2 queue**. Separately, extraction-worthy categories (school, bills, events) always queue S2 extraction, while deterministic extractors give an immediate first pass. Corrections and S2 labels go into `CategoryLabel`.
2. **Event:** dedupe → route → write it or send it to the approval inbox (adult PIN on the dashboard) → learned rules are updated.
3. **GPU wake:** schedule or queue trigger → WoL → agent reports `available` → drain → auto-suspend. If a game starts: `busy_gaming`, models unloaded, draining pauses.
4. **Recurring task:** tick → priority (due date, season, weather) → strategy picks who → Now list → notification.
5. **Dashboard:** member picker → complete (no PIN) or approve (adult PIN) → audit log.
6. **Weather:** poll → rules → alerts and tasks. Seasonal S2 advisories and suggested rules go to an adult for review.
7. **Grocery:** pick meals → normalise and merge → non-staples go onto the living list, staples go to the "Staples check" section, and marking a staple 'low' adds it to the list.
8. **Startup probe:** `fa-sysprobe` → suggested S1 profile → shown in settings.
9. **Model selection:** `fa-eval` runs the labelled sample against each candidate backend → report → an adult picks the backend and model in settings.

## Roadmap (D-023)
| Milestone | Content | Arch docs needed first |
|---|---|---|
| M0 | Skeleton: workspace, DB, config, auth/RBAC, devices, Vue shell, compose, backups | `Domain.md`, `Platform.md` |
| M1 | Members, weather, calendar read, dashboard | `Frontend.md` |
| M2 | Tasks + ntfy | `Tasks.md` |
| M3 | Email rules pipeline, bills, linking, approvals | `Ingest.md` |
| M4 | System One tier + eval harness, System 2, GPU agent, WoL, advisories | `AI.md` |
| M5 | Recipes, meal plan, grocery | `Meals.md` |
| M6 | Smart assignment, learned routing, TUI | updates |
