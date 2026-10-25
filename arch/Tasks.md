---
title: Tasks — lists, recurrence, assignment, notifications
scope: task lists, recurring definitions, occurrences and steps, assignment strategies, completion policy, priority, routines, weather/season triggers, ntfy notifications
status: draft
updated: 2026-10-24
drawio: Tasks.drawio
drawio_src: src/Tasks.py
depends_on: [D-006, D-010, D-013, D-017, D-019, D-023, D-027, D-029, D-031, D-035]
milestone: M2
---

# Tasks: lists, recurrence, assignment, notifications (DRAFT v0)

This doc covers M2 (D-023): task lists, recurring work, the fixed/rotation/pool strategies (D-013), completion from phones and the dashboard, and ntfy notifications (D-010).

**v0 is preliminary.** It fixes what earlier decisions already settle. Where behaviour depends on the open batch 14 questions (Q-035 to Q-045), the doc says so and describes the recommended option as a placeholder. It will be re-derived from scratch once those answers are processed. The M2 plan (`plan/M2.md`) waits for that.

Out of scope for M2: tasks extracted from email and "respond to email" tasks (M3, `Ingest.md`), the System One urgency score (M4, D-027), and smart assignment (M6).

## 1. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-tasks` (new) | Lists, definitions, occurrences, steps. Recurrence (Q-035), missed-occurrence handling (Q-036), the `AssignmentStrategy` trait with `fixed`, `rotation`, `pool` (D-013, Q-037, Q-038), completion policy checks, the deterministic priority score (Q-040), routines (Q-039), season windows (Q-041). Jobs `tasks.materialize`, `tasks.tick` | fa-domain, fa-store, fa-jobs |
| `fa-notify` (new) | `Notifier` trait with the ntfy implementation (D-010), per-member preferences, quiet hours, batching (Q-042). Job `notify.send`. Other features (M3 email, re-auth alerts) reuse it | fa-domain, fa-store, fa-jobs, fa-connectors |
| `fa-connectors` (extended) | `ntfy` client: HTTP publish to `<base>/<topic>` with `Title`, `Priority`, `Tags`, `Click` headers and a bearer access token. No DB access | (external crates only) |
| `fa-weather` (extended) | `task` rules now fire: the evaluator calls `fa-tasks` through a `TaskSink` trait defined in `fa-domain`, so feature crates still don't depend on each other | fa-domain |
| `fa-api` (extended) | `/api/v1/tasks*`, `/lists*`, `/definitions*`, `/routines*`, `/notify-prefs*` routes, plus `tasks.updated` on the SSE hub (ChangeBus, Feeds.md §6) | fa-tasks, fa-notify |

The wiring in `fa-server` provides `fa-tasks` as the `TaskSink` implementation for `fa-weather`.

## 2. Data model (page "Model")
This refines the Tasks package of [Domain.md](Domain.md). All tables are household-scoped with RLS and follow its persistence conventions (UUID ids, `version` for optimistic concurrency, `archived_at` soft delete, audit log).

| Table | Key columns | Notes |
|---|---|---|
| `task_list` | name, kind: `chores\|routine\|homework\|general` (+ `email` in M3), completion_policy: `assignee\|adult_only` (D-013), visible_on_dashboard, color?, sort | Per-list permission and display flags wait for Q-043 and Q-044 |
| `task_definition` | list, title, notes?, steps_template[], schedule (Q-035: an RRULE *or* an `after_done` interval), lead_time, on_missed (Q-036), strategy: `fixed\|rotation\|pool` (`smart` reserved for M6), rotation_cursor, priority_flag: `normal\|high`, season_window? (Q-041), last_done_at?, next_due_at, paused, version | The template for recurring work |
| `task_definition_candidate` | definition, member, ord | The ordered member set. `fixed` has exactly one |
| `task` | list, definition?, title, notes?, due_at?, visible_from?, status: `open\|done\|skipped`, origin: `manual\|system\|weather` (+ `email` in M3), source_ref?, priority_flag, completed_at?, completed_by?, completed_via_device?, claimed_by?/claimed_until? (Q-038), version | One occurrence. The score is computed at read time, not stored |
| `task_assignee` | task ↔ member | Filled by the strategy when the occurrence is created |
| `task_step` | task, ord, text, done_at?, done_by? | Copied from `steps_template`. Homework and routine steps |
| `member_away` | member, from, until, note? | Only if Q-037 picks absences |
| `notify_pref` | member, channel: `ntfy`, topic, token (Secret), triggers[], quiet_from?, quiet_until?, copy_to_member? | D-010 and Q-042 |
| `notification` | member, trigger, dedupe_key (unique), payload, created_at, sent_at?, error? | Outbox for batching and retries |

**Invariants (from D-013 and Domain.md):**
- A `fixed` definition has exactly one candidate. `rotation` and `pool` have at least one.
- At most one **open** occurrence per definition, unless `on_missed = pile_up` (Q-036).
- The rotation cursor moves only under the rule chosen in Q-037. It moves inside the same transaction as the completion.
- Completing an occurrence stamps `last_done_at` and, for `after_done` schedules, sets `next_due_at` from it.

## 3. Behaviour
### 3.1 Materialising occurrences
- `tasks.materialize` runs every minute, alongside `calendar.sync_due`. For each active definition whose `next_due_at - lead_time` has passed, inside the season window, with no open occurrence (or `pile_up`), it creates a `task`, its steps, and its assignees. Idempotency key `tasks.materialize:{definition}:{next_due_at}`.
- `tasks.tick` (also every minute) applies `on_missed`, expires claims, and queues `due_soon` / `overdue` notifications.
- Both publish `tasks.updated` on the ChangeBus so open phones and dashboards refresh.

### 3.2 Completion
- Policy check (D-013): `assignee` lets the assignee or any adult complete; `adult_only` needs an adult's own session.
- Dashboard: while `household.dashboard_completion_override` is on (the default), anyone can complete from a paired device with the member picker and no PIN (D-019). The audit log records the member and the device.
- An **undo** is a normal reopen within 10 minutes that also rolls back the cursor and `last_done_at` changes of that completion. This is a design choice of this doc, not a decision.

### 3.3 Priority (deterministic, before M4)
Score = due proximity (overdue ranks highest) + `priority_flag` + origin boost (`weather`). The weights and the size of the Now list are settings (Q-040). In M4 the `task.urgency` pipeline (D-027, D-029) adds a term.

### 3.4 Weather and seasons
A `weather_rule` with action `task` (Feeds.md §2) creates an occurrence from its `task_definition` through `TaskSink`, deduplicated per rule and day. Whether it creates or suggests is Q-041.

### 3.5 Notifications (ntfy)
- The `ntfy` container is already in the compose sample (`binwiederhier/ntfy`, Platform.md). The server should run with `auth-default-access: deny-all`. Each member's topic gets an access token; the token is a `Secret`, and the member's phone subscribes with it over the tailnet.
- Triggers, defaults, quiet hours, and children's notifications wait for Q-042. Messages go into the `notification` outbox; `notify.send` batches by member within a short window and retries with backoff. Notifications are never sent with household specifics in the topic name.
- `fa-notify` is generic: M3 adds time-sensitive email and approval triggers, and the existing `needs_reauth` source state gets a trigger in M2 if cheap.

## 4. API (sketch; the M2 plan fixes the table)
| Method and path | Who | Purpose |
|---|---|---|
| `GET /api/v1/tasks?member=&list=&status=&now=true` | any signed-in, dashboard | Lists with the computed score. Children see their own and household tasks (D-006) |
| `POST /api/v1/tasks` | Q-043 | One-off task |
| `POST /api/v1/tasks/{id}/complete`, `/reopen`, `/skip`, `/claim` | policy (§3.2) | Completion, undo, skip (adult), claim |
| `PATCH /api/v1/tasks/{id}/steps/{step}` | as completion | Tick a step |
| `GET/POST/PATCH /api/v1/lists`, `/definitions` | adults (Q-043) | Management |
| `GET/PUT /api/v1/members/{id}/notify-prefs`, `POST .../test` | the member, adults for children | Preferences, send a test |

## 5. Frontend (summary; Frontend.md gets the detail)
- App shell: **Tasks** page (my tasks, by list), **Settings → Tasks** (lists, definitions, templates per Q-045), **Settings → Notifications**.
- Kiosk panels: `now` (the Now list), `member_tasks` (per-member columns, from the existing `members` panel), `routine` (Q-039). Unknown panel kinds are already ignored by older clients.

## 6. Open questions (batch 14)
| Q | Affects |
|---|---|
| Q-035 recurrence anchor | `task_definition.schedule`, §3.1 |
| Q-036 missed occurrences | `on_missed`, §3.1 |
| Q-037 rotation rules | cursor rule, `member_away` |
| Q-038 pool claiming | `claimed_by`, §4 `/claim` |
| Q-039 routines | routine model, `routine` panel |
| Q-040 Now list | score, settings, `now` panel |
| Q-041 weather and seasons | `season_window`, `task_mode` |
| Q-042 notifications | `notify_pref`, triggers, quiet hours |
| Q-043 permissions | RBAC matrix rows, list flags |
| Q-044 tasks in calendar | agenda/week panels, list flag |
| Q-045 starter templates | template data file, example TOML |
