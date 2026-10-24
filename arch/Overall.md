---
title: Overall Architecture
scope: system
status: draft
updated: 2026-10-23
drawio: Overall.drawio
depends_on: []
---

# Overall Architecture (DRAFT v0, pending Batch 1 answers)

> This is a starting proposal. Every choice here can change once the `design/` Batch 1 questions are answered. Choices that depend on a question are tagged like **[Q-003]**.

## Principles
1. **Pipeline first, LLM last.** Every ingest path runs deterministic stages first: headers, sender rules, schema.org data, ICS/regex extraction. Only items the rules can't settle go to the local LLM, and the LLM must answer in JSON that matches a schema (grammar-constrained).
2. **The family member is the main entity.** Emails, events, tasks, and meals all link to one or more `Member` records.
3. **One backend, many thin clients.** The dashboard, web/mobile app, and TUI all use the same API.
4. **Runs locally.** The only outbound calls are to the data sources themselves (Gmail, calendars, weather, recipe URLs).

## Components (UML component view, see Overall.drawio)
| Component | Responsibility | Proposed tech |
|---|---|---|
| **API Server** | REST + WebSocket/SSE, auth, domain logic | Python 3.12 + FastAPI **[Q-002]** |
| **Scheduler / Workers** | Polling (mail, calendar, weather), recurring task generation, LLM job queue | APScheduler or a lightweight queue (`arq`/`huey`) in the same codebase |
| **Ingest Connectors** | Gmail (API + OAuth, history-id incremental sync), CalDAV (`caldav`), Google Calendar API, Open-Meteo (no key) | Python adapters behind one `Connector` interface |
| **Classification Pipeline** | Email → rules → features → (LLM if needed) → category + member links + extracted entities (bills, events, tasks, deals) | Rules engine (YAML) + Ollama |
| **Task Engine** | Recurring rules (RRULE-like), last-done tracking, assignment strategies (fixed / rotation / pool / smart), priority scoring (due date, season, weather) | Deterministic scoring; LLM only for suggestions |
| **Meal / Recipe Service** | URL → `recipe-scrapers` (schema.org) → LLM fallback for pages without structured data; meal plan; grocery list with unit normalisation and merging | `recipe-scrapers`, `pint`/custom units |
| **Local LLM Runtime** | Classify, extract, summarise | Ollama: ~7–8B instruct model at Q4 (e.g. Qwen2.5-7B / Llama-3.1-8B) fits in 8 GB VRAM; `nomic-embed-text` for embeddings |
| **Datastore** | Relational data + FTS + vectors | PostgreSQL 16 + `pgvector` **[Q-004]** (SQLite + sqlite-vec is the lighter alternative) |
| **Web App (PWA)** | Mobile/desktop UI and dashboard route | SvelteKit (SPA/PWA) **[Q-002]** |
| **Dashboard (Echo Show 15)** | Read-mostly kiosk view, auto-refresh via SSE | Same web app at `/dashboard` route; keep-alive approach **[Q-010]** |
| **TUI** (stretch) | Terminal client | Python Textual on the same API |

## Core domain model (first pass)
`Household` 1—* `Member{role: adult|child, birthdate, grade, school, daycare, interests[], hobbies[], responsibilities[]}`
`Source{type: gmail|caldav|gcal|weather, credentials, owner Member?}`
`Message{source, headers, category, confidence, member_links[], extracted{bill?, events[], tasks[], deals[]}}`
`Event{calendar, members[], origin: synced|extracted, dedupe_key}`
`Task{list, recurrence?, last_done, assignment_strategy, assignees[], due, priority, origin: manual|email|system, steps[]}`
`Recipe{source_url, ingredients[], steps[]}` · `MealPlanEntry{date, slot, recipe}` · `GroceryList{until_date, items[]}`
`WeatherSnapshot{ts, current, forecast[]}`

## Key flows
1. **Email ingest:** poll Gmail → dedupe → rule stage (List-Unsubscribe, Gmail category labels, sender/domain map to school, daycare, teacher, or biller) → if still unclassified, LLM classify → extract (bill amount and due date, event datetime and location, action items) → link members (teacher→child map, grade/age keywords) → emit Events/Tasks → events are deduped against calendars before write-back **[Q-007]**.
2. **Recurring task:** a scheduler tick evaluates each recurrence and last_done → computes priority → assigns by strategy → adds the task to the "Now" list.
3. **Recipe:** URL → fetch → schema.org parse → LLM fallback → normalise ingredients → store.

## Deployment (proposal)
Docker Compose on one home host with the GPU: `api`, `worker`, `postgres`, `ollama`, `web` (static, served by API or Caddy). Remote access options are listed in **[Q-001]**/**[Q-009]**.

## Open dependencies
See `design/_index.md`, Batch 1.
