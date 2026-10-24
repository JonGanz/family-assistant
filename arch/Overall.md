---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-23
drawio: Overall.drawio
drawio_src: src/Overall.py
depends_on: [D-001, D-002, D-003, D-004, D-005, D-006, D-007, D-008, D-009, D-010, D-011, D-012, D-013, D-014, D-015, D-017, D-018, D-019, D-020, D-021, D-023, D-024, D-025, D-026, D-027, D-028, D-029, D-030, D-031, D-032, D-033]
research: [R-001]
---

# Overall Architecture (DRAFT v6)

v6 re-derives the design from PROJECT.md and all accepted decisions (D-001 to D-033). D-016 and D-022 are superseded. Open: Q-032 (email metadata kept), Q-033 (repo visibility).

## Scope
**In:** members and roles; Gmail ingest and classification; bills; school, daycare, and teacher linking; calendars (Google, CalDAV, ICS) with event extraction; weather with rules and advisories; tasks; meal plan, recipes, the living grocery list; configurable AI decision pipelines; dashboard, PWA, TUI (stretch); ntfy.

**Out:** deals (D-012), school portal APIs (D-014), budgeting (D-009), grocery exports (D-015), Alexa (D-011), gamification (D-013), dietary preferences (deferred, D-021).

## Principles
1. **It works without AI, and AI makes it better.** Every decision is a **configurable pipeline** of stages: rules → kNN → System One → LLM → human. Every pipeline ends in a deterministic or human stage (D-002, D-024, D-029).
2. **Hardware-adaptive.** Models are registered backends on whatever nodes exist. The system probes the hardware and suggests profiles, and nothing is hard-wired to the reference machines (D-026).
3. **Members come first, and everything is configured.** Files bootstrap the config, the UI owns it after that, and there is no PII in the repo (D-004, D-018).
4. **Our code is Rust and Vue.** Third-party model servers can be containerised Python sidecars (D-003, D-025).
5. **Local and private.** Access is over Tailscale only (D-010). If in doubt, make it a toggle.
6. **Holds as little as possible.** Email content is purged once processed. Every derived item keeps a provenance link back to its source, and Gmail stays the system of record (D-030).
7. **The hoster deploys.** We ship images and a compose sample with profiles, and don't prescribe how it is run (D-033).

## Deployment (page "Deployment"): reference hardware
| Node | Always on | Runs |
|---|---|---|
| **Core node**: Ubuntu Server | yes | Docker Compose with `fa-server` (api + worker, including `fa-sysprobe`), `postgres:16` + pgvector, `ntfy`, `backup` to NAS, tailscale. **Suggested backends:** `s1-small` (System One sidecar, e.g. GLiNER2.5-Decide on CPU) and `embed` (a small embedding model) |
| **GPU node(s)**: 0 or more. Reference is a Linux gaming PC with RTX 3070 8 GB | no | `fa-gpu-agent` (hardware report, gaming detection, suspend). **Suggested backends:** `s1-large` (e.g. Strands 2B or Kev 4B sidecar) and `llm` (Ollama 7–8B Q4). Woken by WoL (D-020, D-026) |
| **NAS** | yes | Nightly pg_dump (D-005) |
| **Echo Show 15** | yes | Silk browser on `/dashboard` with keep-alive (D-011) |
| **Phones and desktops** | n/a | PWA and the ntfy app |

The final model choices come from `fa-eval` on the household's labelled sample (D-024, D-028).

## Components (page "Components")
| Crate | Responsibility |
|---|---|
| `fa-api` | Axum REST + SSE, sessions, RBAC, device sessions with adult PIN step-up (D-006, D-019) |
| `fa-config` | Versioned TOML import and export. Includes pipelines and backends (D-018, D-029) |
| `fa-auth` | Passwords, sessions, devices, PIN step-up, `Policy::authorize` (see Platform.md §5) |
| `fa-jobs` | Postgres job queue and cron schedule (see Platform.md §7) |
| `fa-connectors` | Gmail, Google Calendar, CalDAV, ICS, Open-Meteo |
| `fa-pipeline` | **Decision pipeline engine** (D-029): runs each decision point's stage list with thresholds and `on_unavailable` handling. Also handles versioning and dry-run. Email ingest uses it, followed by S2 extraction → emit events, tasks, and bills |
| `fa-rules` | Deterministic rulesets (sender and Org maps, headers, keywords, biller templates) and extractors (ICS, dates, PDF) |
| `fa-decide` | `DecisionBackend` trait with a `/v1/systemone` client (`choice`, `score`, `noul`) and the kNN stage over pgvector (D-024, D-028) |
| `fa-llm` | System 2 client (Ollama): JSON-schema prompts for extraction, routing hints, recipes, advisories. Uses a queue when the backend is `on_demand` |
| `fa-backends` | **Backend registry**: availability tracking, preference-list routing, the model catalogue, and the profile suggester from node hardware reports (D-026) |
| `fa-sysprobe` | Core node hardware probe |
| `fa-eval` | Replays the labelled sample against backends and pipelines (accuracy, calibration, latency). Powers dry-run (D-028, D-029) |
| `fa-calendar` | Dedupe, approval inbox, writes. Routing is the `event.calendar_route` pipeline (D-008, D-027) |
| `fa-tasks` | Recurrence, fixed/rotation/pool strategies (smart later), completion policy, priority (urgency from the `task.urgency` pipeline) (D-013) |
| `fa-weather` | Snapshots, `WeatherRule`s, S2 advisories and suggested rules (D-017) |
| `fa-meals` | Recipes, meal plan, the living grocery list with the staples check (D-015, D-021) |
| `fa-gpu-node` | WoL and agent state for each GPU node (D-020) |
| `fa-notify` | `Notifier` trait with an ntfy implementation (D-010) |
| `fa-domain`, `fa-store` | Entities, DTOs (`ts-rs`), SQLx repositories, encrypted secrets |
| `fa-gpu-agent` | Separate binary on each GPU node |
| `web` | Vue 3 + TS PWA: dashboard, labelling view, **Pipeline settings**, **Backends** screens |
| `fa-tui` | ratatui (stretch) |

## Domain model
The detailed model is in **[Domain.md](Domain.md)**. Summary:
- `Household{location, tz, meal_plan_horizon_weeks, dashboard_completion_override}` 1—* `Member{role, avatar, birthdate?, grade?, interests[], hobbies[], responsibilities[], notify_prefs, pin_hash?}`
- `UserAccount`, `Device{kind: dashboard}`, `AuditEntry{actor, device?, action}`
- `Org{type, domains[], senders[], ics_urls[]}` and `OrgContact{role, members[]}`
- `MailSource{owners[*], read_only, backfill_days, state}`
- `Message{category, confidence, decided_by_stage, member_links[], embedding?, extracted{bill?, events[], tasks[]}}` and `CategoryLabel{message, decision_point, value, source: rule|s2|user, in_eval_sample}`
- `CalendarSource`, `Calendar`, `Event{status, confidence, dedupe_key}`
- `TaskList{completion_policy}` 1—* `Task{recurrence?, last_done, strategy, assignees[], due, priority, urgency?, origin, steps[]}`, and `Bill`
- `Recipe`, `Ingredient{is_staple}`, `MealPlanEntry`, `GroceryList` 1—* `GroceryItem`
- `WeatherSnapshot`, `WeatherRule{status}`, `Advisory`
- **AI:**
  - `Node{kind: core|gpu, hardware{cpu, ram, vram}, state}`
  - `Backend{node, kind: system_one|llm|embedding, endpoint, model, dialect, capabilities[], availability}`
  - `CatalogModel{name, kind, min_ram/vram, license}`
  - `DecisionPoint{key}` 1—* `PipelineVersion{active, created_by}` 1—* `Stage{order, kind: rules|knn|system_one|llm|human, backend_prefs[], accept_threshold, on_unavailable, enabled}`
  - `Decision{subject, decision_point, stage, value, probabilities[]}`
  - `LlmJob`, `EvalRun{target, accuracy, ece, p50_ms}`

## Key flows
1. **Decision pipeline (generic):** for each enabled stage in order: is a backend available (preference list)? If not, apply `on_unavailable` (skip, wait, or queue). Run it. If the result is ≥ the threshold, accept and record the `Decision`. Otherwise go to the next stage. The last stage is always deterministic or human.
2. **Email:** poll (content held, `pending`) → the `email.category`, `email.time_sensitive`, `email.is_bill`, and `email.member_link` pipelines run (a time-sensitive result sends a notification now) → for extraction-worthy categories, S2 extraction is queued while the deterministic extractors give a first pass → emit events, tasks, and bills, each with a `SourceRef` (Gmail link) → `processed` → content purged (D-030). Corrections go into `CategoryLabel`. Eval and dry-run re-fetch content from Gmail.
3. **Event:** dedupe → `event.calendar_route` pipeline → write it, or send it to the approval inbox (adult PIN on the dashboard).
4. **Hardware onboarding:** a node reports its hardware → the profile suggester proposes backends from the catalogue and a pipeline preset → an adult accepts it in the Backends screen.
5. **Pipeline change:** edit in Pipeline settings → dry-run on the eval sample and recent history → compare before and after → save a new version (rollback is possible).
6. **GPU wake:** schedule or queue → WoL → agent `available` → on-demand backends come up → queued stages drain → auto-suspend. A game starting means `busy_gaming`.
7. **Recurring task, Dashboard, Weather, Grocery:** as in v4 (see D-013, D-019, D-017, D-021).

Platform details (layout, security, deployment, backups) are in **[Platform.md](Platform.md)**.

## Roadmap (D-023, adjusted by D-029)
| Milestone | Content | Arch docs needed first |
|---|---|---|
| M0 | Skeleton: workspace, DB, config, auth/RBAC, devices, Vue shell, compose, backups | `Domain.md`, `Platform.md` |
| M1 | Members, weather, calendar read, dashboard | `Frontend.md` |
| M2 | Tasks + ntfy | `Tasks.md` |
| M3 | Email ingest, **pipeline engine with the 'Rules only' preset**, bills, linking, approvals, **labelling view** | `Ingest.md` |
| M4 | Backend registry, System One sidecars, kNN, S2, eval, **Pipeline and Backends settings**, GPU agent, WoL, advisories | `AI.md` |
| M5 | Recipes, meal plan, grocery | `Meals.md` |
| M6 | Smart assignment, TUI | updates |
