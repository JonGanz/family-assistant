---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-30
drawio: Overall.drawio
drawio_src: src/Overall.py
depends_on: [D-001, D-002, D-003, D-004, D-005, D-006, D-007, D-008, D-009, D-010, D-011, D-012, D-013, D-014, D-015, D-017, D-018, D-019, D-020, D-021, D-023, D-024, D-025, D-026, D-027, D-028, D-029, D-031, D-032, D-033, D-034, D-035, D-036, D-037, D-038, D-039, D-040, D-041, D-042, D-043, D-044, D-045, D-046, D-047, D-048, D-049, D-050, D-051, D-052, D-053, D-054, D-055, D-056, D-057, D-058, D-059, D-060, D-061, D-062, D-063, D-064, D-065, D-066, D-067, D-068, D-069, D-070, D-071, D-072, D-073, D-074, D-075, D-076, D-077]
research: [R-001]
---

# Overall Architecture (DRAFT v8)

v8 re-derives the design from PROJECT.md and all accepted decisions (D-001 to D-077). D-016, D-022, and D-030 are superseded. New in v8: simple onboarding (D-065, D-072 to D-077), the M3.5 milestone (D-066), the M4/M4.1 split (D-067), an at-a-glance dashboard without the approval inbox (D-068), and member interests as LLM-only inputs (D-071). Detail is in [Platform.md](Platform.md) and [Frontend.md](Frontend.md).

## Scope
**In:** members and roles; email ingest (Gmail API or IMAP, D-052) and classification; bills; school, daycare, and teacher linking; calendars (Google, CalDAV, ICS) with event extraction; weather with rules and advisories; tasks and routines; goals with progress; meal plan, recipes, the living grocery list; configurable AI decision pipelines; dashboard, PWA, TUI (stretch); ntfy.

**Out:** deals (D-012), school portal APIs (D-014), budgeting (D-009), grocery exports (D-015), Alexa (D-011), gamification (D-013), dietary preferences (deferred, D-021).

## Principles
1. **It works without AI, and AI makes it better.** Every decision is a **configurable pipeline** of stages: rules → kNN → System One → LLM → human. Every pipeline ends in a deterministic or human stage (D-002, D-024, D-029).
2. **Hardware-adaptive.** Models are registered backends on whatever nodes exist. The system probes the hardware and suggests profiles, and nothing is hard-wired to the reference machines (D-026).
3. **Members come first, and everything is configured in the UI.** A first-run wizard sets up the household (D-065); the UI owns configuration after that, including non-infrastructure server settings (D-075). There is no household file import or export (D-089). There is no PII in the repo (D-004).
4. **Our code is Rust and Vue.** Third-party model servers can be containerised Python sidecars (D-003, D-025).
5. **Local and private.** Nothing is exposed to the internet. The whole app is served on the home network over plain HTTP by default; HTTPS and access from outside come from an optional Tailscale sidecar or the hoster's own proxy (D-010 as amended by D-065, D-072). If in doubt, make it a toggle.
6. **Keeps only what helps.** Email content is kept only as far as it helps local learning, under a configurable retention policy. Every derived item keeps a provenance link back to its source, and Gmail stays the system of record (D-034).
7. **Two commands to install.** We ship an amd64 image on GHCR and one compose file: download it, `docker compose up -d`, open the browser (D-033, D-065, D-077). Secrets are generated on first start (D-074). Hosters may still deploy however they like.
8. **Open source.** The repo and images are public, AGPL-3.0-or-later. No household data goes in the repo, docs, or logs (D-035).

## Deployment (page "Deployment"): reference hardware
| Node | Always on | Runs |
|---|---|---|
| **Core node**: Ubuntu Server | yes | Docker Compose: `init` (one-shot, generates the master key and DB password into a `secrets` volume, D-074), **one `app` container** (`fa-server serve all`: api and worker, D-076), `postgres:16` + pgvector, `ntfy`, `backup` (to a volume or the NAS). Optional profile `tailscale`: a Tailscale sidecar that serves HTTPS on the tailnet (D-065). The app port is plain HTTP on the LAN (D-072). **Backends:** M4 adds one CPU System One backend; M4.1 adds `embed` and the registry (D-067) |
| **GPU node(s)**: 0 or more. Reference is a Linux gaming PC with RTX 3070 8 GB | no | M4: `llm` (Ollama 7–8B Q4), used when the PC is awake, with rules as the fallback. M4.1: `fa-gpu-agent` (hardware report, gaming detection, suspend), WoL, `s1-large` (D-020, D-026, D-067) |
| **NAS** (optional) | yes | Nightly pg_dump when `BACKUP_DIR` points at it; otherwise a local volume (D-005) |
| **Echo Show 15** | yes | Silk browser on `/dashboard` over the LAN, with keep-alive (D-011). At-a-glance only: date, time, weather, agenda, tasks, routines (D-068) |
| **Phones and desktops** | n/a | The web app (installable as a PWA over HTTPS) and the ntfy app; on the LAN, or through Tailscale from outside |

The final model choices come from `fa-eval` on the household's labelled sample (D-024, D-028).

## Components (page "Components")
| Crate | Responsibility |
|---|---|
| `fa-api` | Axum REST + SSE, sessions, RBAC, device sessions with adult PIN step-up (D-006, D-019) |
| `fa-config` | Process config and runtime settings (env > DB > default, D-075); no household file import or export (D-089) |
| `fa-auth` | Passwords, sessions, devices, PIN step-up, `Policy::authorize` (see Platform.md §5) |
| `fa-jobs` | Postgres job queue and cron schedule (see Platform.md §7) |
| `fa-connectors` | Gmail, IMAP, Google Calendar, CalDAV, ICS, Open-Meteo |
| `fa-pipeline` | **Decision pipeline engine** (D-029): runs each decision point's stage list with thresholds and `on_unavailable` handling. Also handles versioning and dry-run. Email ingest uses it, followed by S2 extraction → emit events, tasks, and bills |
| `fa-rules` | Deterministic rulesets (sender and Org maps, headers, keywords, biller templates) and extractors (ICS, dates, PDF) |
| `fa-decide` | `DecisionBackend` trait with a `/v1/systemone` client (`choice`, `score`, `noul`) and the kNN stage over pgvector (D-024, D-028) |
| `fa-llm` | System 2 client (Ollama): JSON-schema prompts for extraction, routing hints, recipes, advisories. Uses a queue when the backend is `on_demand` |
| `fa-backends` (M4.1) | **Backend registry**: availability tracking, preference-list routing, the model catalogue, and the profile suggester from node hardware reports (D-026) |
| `fa-sysprobe` (M4.1) | Core node hardware probe |
| `fa-eval` (M4 basic, M4.1 full) | Replays the labelled sample against backends and pipelines (accuracy, calibration, latency). Powers dry-run (D-028, D-029) |
| `fa-calendar` | Dedupe, approval inbox, writes. Routing is the `event.calendar_route` pipeline (D-008, D-027) |
| `fa-tasks` | Recurrence, fixed/rotation/pool strategies (smart later), completion policy, priority (urgency from the `task.urgency` pipeline) (D-013) |
| `fa-weather` | Snapshots, `WeatherRule`s, S2 advisories and suggested rules (D-017) |
| `fa-meals` | Recipes, meal plan, the living grocery list with the staples check (D-015, D-021) |
| `fa-gpu-node` (M4.1) | WoL and agent state for each GPU node (D-020) |
| `fa-notify` | `Notifier` trait with an ntfy implementation (D-010) |
| `fa-domain`, `fa-store` | Entities, DTOs (`ts-rs`), SQLx repositories, encrypted secrets |
| `fa-gpu-agent` (M4.1) | Separate binary on each GPU node |
| `web` | Vue 3 + TS PWA: first-run wizard, at-a-glance dashboard, app pages, adult-only Settings (incl. Members, System, **Pipeline settings**, **Backends**) (D-065, D-070, D-075) |
| `fa-tui` | ratatui (stretch) |

## Domain model
The detailed model is in **[Domain.md](Domain.md)**. Summary:
- `Household{location, tz, meal_plan_horizon_weeks, dashboard_completion_override}` 1—* `Member{role, avatar, birthdate?, grade?, interests[], hobbies[], responsibilities[], notify_prefs, pin_hash?}`. Interests, hobbies and responsibilities are inputs to LLM features only (D-071); members are edited by adults only (D-070)
- `UserAccount`, `Device{kind: dashboard}`, `AuditEntry{actor, device?, action}`
- `Org{type, domains[], senders[], ics_urls[]}` and `OrgContact{role, members[]}`
- `MailSource{owners[*], read_only, backfill_days, state}`
- `Message{category, confidence, decided_by_stage, member_links[], embedding?, extracted{bill?, events[], tasks[]}}` and `CategoryLabel{message, decision_point, value, source: rule|s2|user, in_eval_sample}`
- `CalendarSource`, `Calendar`, `Event{status, confidence, dedupe_key}`
- `TaskList{completion_policy}` 1—* `Task{recurrence?, last_done, strategy, assignees[], due, priority, urgency?, origin, steps[]}`, and `Bill`
- `Recipe`, `Ingredient{is_staple}`, `MealPlanEntry`, `GroceryList` 1—* `GroceryItem`
- `WeatherSnapshot`, `WeatherRule{status}`, `Advisory` (not dismissable; cleared when no longer relevant, D-068)
- `InstanceSetting{key, value, secret?}` (system-wide runtime settings, D-075)
- **AI:**
  - `Node{kind: core|gpu, hardware{cpu, ram, vram}, state}`
  - `Backend{node, kind: system_one|llm|embedding, endpoint, model, dialect, capabilities[], availability}`
  - `CatalogModel{name, kind, min_ram/vram, license}`
  - `DecisionPoint{key}` 1—* `PipelineVersion{active, created_by}` 1—* `Stage{order, kind: rules|knn|system_one|llm|human, backend_prefs[], accept_threshold, on_unavailable, enabled}`
  - `Decision{subject, decision_point, stage, value, probabilities[]}`
  - `LlmJob`, `EvalRun{target, accuracy, ece, p50_ms}`

## Key flows
1. **Decision pipeline (generic):** for each enabled stage in order: is a backend available (preference list)? If not, apply `on_unavailable` (skip, wait, or queue). Run it. If the result is ≥ the threshold, accept and record the `Decision`. Otherwise go to the next stage. The last stage is always deterministic or human.
2. **Email:** poll (content held, `pending`) → the `email.category`, `email.time_sensitive`, `email.is_bill`, and `email.member_link` pipelines run (a time-sensitive result sends a notification now) → for extraction-worthy categories, S2 extraction is queued while the deterministic extractors give a first pass → emit events, tasks, and bills, each with a `SourceRef` (Gmail link) → `processed` → the retention policy keeps or purges content (D-034). Corrections go into `CategoryLabel`. Purged content is re-fetched from Gmail when needed.
3. **Event:** dedupe → `event.calendar_route` pipeline → write it, or send it to the approval inbox, which adults work through in the app (`/inbox`); the dashboard never shows it (D-068).
0. **First run:** `docker compose up -d` → `init` creates secrets → the first browser visit finds no household and opens `/setup` (open to the first visitor, D-073) → household, first adult, family, access and notifications, master-key backup → setup closes for good.
4. **Hardware onboarding (M4.1):** a node reports its hardware → the profile suggester proposes backends from the catalogue and a pipeline preset → an adult accepts it in the Backends screen.
5. **Pipeline change:** edit in Pipeline settings → dry-run on the eval sample and recent history → compare before and after → save a new version (rollback is possible).
6. **GPU wake (M4.1):** schedule or queue → WoL → agent `available` → on-demand backends come up → queued stages drain → auto-suspend. A game starting means `busy_gaming`.
7. **Recurring task, Dashboard, Weather, Grocery:** as in v4 (see D-013, D-019, D-017, D-021).

Platform details (layout, security, deployment, backups) are in **[Platform.md](Platform.md)**.

## Roadmap (D-023, adjusted by D-029, D-050, D-066 and D-067)
| Milestone | Content | Arch docs needed first |
|---|---|---|
| M0 | Skeleton: workspace, DB, config, auth/RBAC, devices, Vue shell, compose, backups | `Domain.md`, `Platform.md` |
| M1 | Members, weather, calendar read, dashboard | `Frontend.md`, `Feeds.md` |
| M2 | Tasks + ntfy | `Tasks.md` |
| M3 | Email ingest, **pipeline engine with the 'Rules only' preset**, bills, linking, approvals, **labelling view** | `Ingest.md` |
| M3.5 | **Onboarding** (D-065): one compose file with an optional Tailscale sidecar, optional HTTPS, generated secrets, the first-run wizard, Settings, System. **Shakedown** (D-066): H-02..H-11 on the real server through the new install, a week of approval-inbox measurement, the M3 known gaps | `Platform.md` v4, `Frontend.md` v4 |
| M4 | **Minimal AI slice** (D-067): one CPU System One backend and one Ollama model behind the existing `fa-pipeline` stages, aimed at the decision points with the most inbox rejections in M3.5; basic eval; first LLM features using member interests (D-071) | `AI.md` |
| M4.1 | Backend registry and profile suggester, sysprobe, GPU agent wake/yield and gaming detection, kNN, `s1-large`, full eval, **Pipeline and Backends settings**, S2 advisories | `AI.md` update |
| M5 | Recipes, meal plan, grocery | `Meals.md` |
| M6 | Smart assignment, learned calendar routing | updates |
| M7 | Goals: count or quantity targets, weekly cadence with reset, progress bars (D-048, D-051) | `Goals.md` |
| M8 | TUI (stretch) | updates |

The M6 to M8 split is D-050; M3.5 and the M4/M4.1 split are D-066 and D-067.
