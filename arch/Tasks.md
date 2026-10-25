---
title: Tasks — lists, recurrence, assignment, routines, notifications
scope: task lists, recurring definitions, occurrences and steps, assignment strategies, absences, completion policy, the Now list, routines, season and weather triggers, calendar display, ntfy notifications
status: draft
updated: 2026-10-25
drawio: Tasks.drawio
drawio_src: src/Tasks.py
depends_on: [D-006, D-010, D-013, D-017, D-018, D-019, D-023, D-027, D-029, D-031, D-035, D-036, D-037, D-038, D-039, D-040, D-041, D-042, D-043, D-044, D-045, D-046, D-047, D-048, D-049, D-050]
milestone: M2
---

# Tasks: lists, recurrence, assignment, routines, notifications (DRAFT v1.1)

This doc covers M2 (D-023): task lists, recurring work, the fixed/rotation/pool strategies, routines, the Now list, completion from phones and the dashboard, and ntfy notifications. v1 is derived from the requirements and D-013 and D-037 to D-050. It replaces the preliminary v0.

**Not in M2:** tasks extracted from email and "respond to email" tasks (M3, `Ingest.md`), the System One urgency term (M4, D-027), smart assignment (M6), starter templates (D-047), and **goals** (D-048, D-051: a separate concept built in M7, D-050).

## 1. Concepts
| Concept | What it is | Decided by |
|---|---|---|
| **List** | A named group of tasks with a completion policy, a calendar display default, and flags for who can add to it | D-013, D-045, D-046 |
| **Definition** | A template for recurring work: schedule, strategy, candidates, missed handling, season window | D-013, D-037 to D-039, D-043, D-049 |
| **Task** | One occurrence (from a definition, a weather rule, or added by hand), with optional steps | D-013 |
| **Routine** | A per-member, ordered, timed checklist that runs on qualifying days, with a progress bar | D-041 |
| **Absence** | A member is away for a date range. Every strategy skips them | D-039 |
| **Now list** | The top N open tasks by a deterministic score | D-042 |
| **Notification** | A message to a member's ntfy topic, triggered by task and routine events | D-010, D-044 |

Routines are their own kind, not tasks with steps (D-041): they run every qualifying day, never roll over, and are shown as a live progress panel.

## 2. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-tasks` (new) | Lists, definitions, occurrences, steps, routines and runs, absences. Scheduling (§4.1), the `AssignmentStrategy` trait with `fixed`, `rotation`, `pool` (§4.2), completion policy (§4.3), season windows (§4.5), the school-day check (§4.6, reading `event` through fa-store). Writes notification rows to the outbox. Jobs `tasks.materialize`, `tasks.tick`, `tasks.digest`, `tasks.from_weather`, `routines.materialize` | fa-domain, fa-store, fa-jobs |
| `fa-notify` (new) | Drains the `notification` outbox (§5): applies preferences and quiet hours, batches, copies to an adult, and publishes through the `Notifier` trait (ntfy implementation). Job `notify.send`. Any crate can produce notifications by inserting outbox rows; M3 adds email and approval triggers | fa-domain, fa-store, fa-jobs, fa-connectors |
| `fa-connectors` (extended) | `ntfy` client: publish to `<base>/<topic>` with `Title`, `Priority`, `Tags`, `Click` headers and a bearer access token. No DB access | (external crates only) |
| `fa-weather` (extended) | Rules with action `task` now fire by enqueuing a `tasks.from_weather` job. `weather.evaluate` also evaluates every definition's `season_condition` and stores the result on the definition (§4.5) | fa-domain, fa-store, fa-jobs |
| `fa-api` (extended) | Routes in §6, plus `tasks.updated` and `routines.updated` on the SSE hub (ChangeBus, Feeds.md §6). Validates `season_condition` with `fa_weather::parse` | fa-tasks, fa-notify, fa-weather |

Feature crates still never depend on each other. They meet only through the store and the job queue: weather → tasks is a `tasks.from_weather` job plus the `condition_ok` column, tasks → calendar is a read of `event` through fa-store, and anything → notify is the `notification` outbox. That keeps the api and worker processes independent, with nothing to wire in `fa-server` beyond registering handlers.

## 3. Data model (page "Model")
This refines the Tasks package of [Domain.md](Domain.md). All tables are household-scoped with RLS and follow its persistence conventions (UUID ids, `version` for optimistic concurrency, `archived_at` soft delete, audit log).

### 3.1 Tables
| Table | Key columns | Notes |
|---|---|---|
| `household` (+cols) | `now_list_size` (7), `now_weights jsonb` (§4.4), `quiet_from` (21:00), `quiet_until` (07:00) | Editable in Settings → Household. Quiet hours are a household default that a member can override |
| `task_list` | name, kind: `chores\|homework\|general` (+ `email` in M3), completion_policy: `assignee\|adult_only` (D-013), `child_can_add: own\|none`, `dashboard_add`, `calendar_default: hidden\|until_done\|always`, `claim_ttl` (default: end of day), visible_on_dashboard, color?, sort, version | Defaults by kind in §3.2 |
| `task_definition` | list, title, notes?, steps_template text[], `schedule_kind: rrule\|after_done`, rrule?, interval?, dtstart, lead_time, `on_missed: roll_over\|auto_skip\|pile_up` (default `roll_over` for `after_done`, `auto_skip` for `rrule`, D-038, D-049), strategy: `fixed\|rotation\|pool` (`smart` reserved for M6), `advance_on: any_completion\|assignee_completion`, `skip_advances`, rotation_cursor, priority_flag: `normal\|high`, season_from?/season_until? (month-day), season_condition? (rule DSL), condition_ok? and condition_checked_at? (written by `weather.evaluate`), last_done_at?, next_due_at, paused, version | D-037 to D-039, D-043, D-049. `rrule` and `interval` are exclusive (CHECK) |
| `task_definition_candidate` | definition, member, ord | Ordered. `fixed` has exactly one |
| `task` | list, definition?, weather_rule?, title, notes?, due_at?, visible_from?, status: `open\|done\|skipped\|suggested`, origin: `manual\|system\|weather` (+ `email` in M3), source_ref?, priority_flag, calendar_display: `hidden\|until_done\|always`, created_by?, completed_at?, completed_by?, completed_via_device?, skipped_by? (`null` = system), claimed_by?, claimed_until?, version | One occurrence. The score is computed at read time. `suggested` is a weather task waiting for an adult (D-043) |
| `task_assignee` | task ↔ member | Set by the strategy when the occurrence is created |
| `task_step` | task, ord, text, done_at?, done_by? | Copied from `steps_template`, or added by hand (homework preparation, D-046) |
| `member_away` | member, from_date, until_date, note? | D-039 |
| `routine` | name, members[] (`routine_member`), steps (`routine_step`: ord, text, target_time?), starts_at, done_by, `day_filter: daily\|weekdays\|weekends\|school_days` (default `weekdays`), school_calendar?, `no_school_match` text[] (default `["no school", "holiday", "inset", "closed"]`), show_on_dashboard, version | D-041 |
| `routine_run` | routine, member, date, status: `open\|done\|skipped`, completed_at? | Unique `(routine, member, date)` |
| `routine_run_step` | run, step, done_at?, done_by?, done_via_device? | Progress = done steps / all steps |
| `notify_pref` (+cols) | member, topic (random, §5.2), triggers text[] (§5), due_soon_minutes (120), nudge_minutes (10), quiet_from?/quiet_until? (override), `copy_to_member?` | D-044. Extends the M0 table |
| `notification` | member, trigger, subject_type, subject_id, dedupe_key (unique), title, body, click_url?, not_before, sent_at?, attempts, last_error? | Outbox (§5) |
| `weather_rule` (+col) | `task_mode: create\|suggest` (default `create`) | D-043, Feeds.md §2 |

### 3.2 Defaults by list kind
| Kind | completion_policy | child_can_add | dashboard_add | calendar_default |
|---|---|---|---|---|
| `chores` | `assignee` | `none` | false | `hidden` |
| `homework` | `assignee` | `own` | false | `until_done` |
| `general` | `assignee` | `none` | **true** | `hidden` |

All of these are editable per list (D-045, D-046). A task can change its own `calendar_display`. Marking homework as a **key date** sets `always` (D-046).

### 3.3 Invariants
- A `fixed` definition has exactly one candidate. `rotation` and `pool` have at least one.
- At most one open (or suggested) occurrence per definition, unless `on_missed = pile_up`. Enforced in code: materialisation locks the definition row (`FOR UPDATE SKIP LOCKED`) before checking.
- The rotation cursor moves only inside the transaction that completes or skips an occurrence, and only as `advance_on` and `skip_advances` allow.
- Completing an occurrence sets `last_done_at` on its definition, and for `after_done` sets `next_due_at = last_done_at + interval`. For `rrule`, `next_due_at` is the next RRULE instance after the current one's due time, regardless of completion.

## 4. Behaviour
### 4.1 Scheduling occurrences
- `tasks.materialize` runs every minute. For each active, unpaused definition where `now >= next_due_at - lead_time`, the season window allows the date, and the `season_condition` passes (§4.5):
  1. apply `on_missed` to an existing open occurrence (`roll_over`: create nothing; `auto_skip`: mark it `skipped` with `skipped_by = null`; `pile_up`: keep it);
  2. create the `task` with `visible_from = next_due_at - lead_time`, `due_at = next_due_at`, steps, and the list's `calendar_default`;
  3. choose assignees with the strategy (§4.2);
  4. for `rrule`, advance `next_due_at` to the next instance.
  The idempotency key is `tasks.materialize:{definition}:{due_at}`.
- `tasks.tick` (every minute) expires claims, and enqueues `overdue` and `due_soon` notifications (deduplicated per task and trigger).
- `routines.materialize` runs at 00:05 household time. It creates a `routine_run` per member for each routine whose day filter passes today (§4.6), and auto-skips yesterday's unfinished runs.
- Every change publishes `tasks.updated` or `routines.updated` on the ChangeBus so open phones and dashboards refresh.

### 4.2 Assignment strategies (D-013, D-039, D-040)
| Strategy | On create | On complete or skip |
|---|---|---|
| `fixed` | The one candidate. If they are away, the task keeps them and is flagged (`assignee_away`) so adults see it | — |
| `rotation` | The candidate at `rotation_cursor`, skipping members who are away on the due date | `any_completion`: move on after any completion. `assignee_completion`: move on only when the assignee completes. A skip moves on only if `skip_advances` |
| `pool` | All candidates who are not away. Anyone in the pool can complete it. A member can claim it (`claimed_by`, `claimed_until = now + list.claim_ttl`), and the claim can be released | — |

`smart` (M6, D-050) slots in behind the same trait and reads absences and the calendar.

### 4.3 Completion, skip, undo
- **Policy (D-013):** `assignee` allows the assignee (or any pool member) and any adult. `adult_only` requires an adult's own session.
- **Dashboard:** while `household.dashboard_completion_override` is on (the default), anyone can complete tasks and routine steps from a paired device with the member picker and no PIN (D-019). The audit log records the member and the device.
- **Skip:** adults only. It records `skipped_by`.
- **Undo** (a design choice of this doc): a reopen within 10 minutes of completion reverts that completion's cursor, `last_done_at`, and `next_due_at` changes, using the before-image in the audit entry. Later reopens just reopen the task.

### 4.4 The Now list (D-042)
- `score = w_due · f(due_at) + w_flag · [priority_flag = high] + w_weather · [origin = weather]`. `f` rises from 0 at `visible_from` to 1 at `due_at` and keeps rising while overdue, capped at 2.
- The default weights are `w_due = 1.0`, `w_flag = 0.5`, `w_weather = 0.5`. They are stored in `household.now_weights` and edited in Settings → Household. M4 adds `w_urgency · urgency/5` (D-027).
- The list holds the top `now_list_size` open tasks with `visible_from <= now`. Suggested tasks are excluded.
- **Filters:** a dashboard `now` panel can filter by member or list (stored in the device layout). On a phone the Now page shows "Mine" (the signed-in member) first, then "Household".

### 4.5 Seasons and weather (D-043)
- A **season window** is `season_from`–`season_until` as month-day values, and it may wrap the year end (Nov–Feb). Outside it, `tasks.materialize` creates nothing and moves `next_due_at` to the window's next start.
- A **season condition** is a weather DSL expression (Feeds.md) that must be **true** for the occurrence to be created (for example `sum(precip_mm, today) < 5`). `weather.evaluate` evaluates it after every fetch and writes `condition_ok` and `condition_checked_at` on the definition. `tasks.materialize` defers a due occurrence while `condition_ok = false`. A missing or stale result (older than 6 h, or no weather at all) counts as true, so tasks never stall on missing weather.
- A **weather rule** with action `task` that matches enqueues `tasks.from_weather{rule, date}` with the idempotency key `tasks.from_weather:{rule}:{date}`, which is the dedupe per rule and day. The fa-tasks handler creates a task from the rule's `task_definition` with `origin = weather` and that definition's strategy. With `task_mode = suggest`, the task is created as `suggested` and appears in a Suggested section for adults to accept (it becomes `open`) or dismiss (it becomes `skipped`).

### 4.6 Routines (D-041)
- **Day filters:** `daily`, `weekdays` (default), `weekends`, or `school_days`. `school_days` means weekdays, minus any day with an all-day event on `school_calendar` whose title contains one of `no_school_match`, case-insensitive. The keywords are editable. If no school calendar is set, `school_days` behaves like `weekdays`.
- **Window:** a routine is active from `starts_at` to `done_by`. A step can have a `target_time` ("teeth by 7:30").
- **Dashboard:** a `routine` panel appears automatically during the window on devices with it enabled. It shows one column per member, large touch targets, and a **progress bar** per member that turns green when every step is done. Outside the window it hides, or it can be pinned in the device layout.
- **Phones:** adults see each member's progress, and children see their own.
- Steps need no PIN on the dashboard (D-019).

### 4.7 Calendar display (D-046)
- `until_done` tasks with a `due_at` appear on the **Calendar page** and the **week** panel as a task chip (checkbox and member colour) until they are completed.
- `always` tasks are **key dates**. They stay after completion (shown as done) and are the only tasks shown in the **agenda** panel, which otherwise shows events.
- `hidden` tasks appear only in task views.
- Preparation for a key date is added as steps on that task or as separate earlier one-off tasks.
- Calendar visibility rules (D-036) don't apply to tasks. Task visibility follows the RBAC matrix (§7).

## 5. Notifications (D-010, D-044)
### 5.1 Triggers
| Trigger | When | Default |
|---|---|---|
| `assigned` | An occurrence is created for the member, or a rotation hands the turn to them | on |
| `overdue` | An open task passes `due_at`. Sent once per task | on |
| `digest` | A morning summary of today's tasks and routines, at the end of quiet hours | on |
| `due_soon` | A set time before `due_at` (default 2 h) | off |
| `routine_nudge` | A set time before `done_by` (default 10 min) while steps remain | off |

### 5.2 Delivery
- Each member's topic is a random 24-character name, generated when preferences are first saved. It acts as the capability (ntfy's standard model) and never contains household specifics. The API returns it only to that member and to adults, and `config export` leaves it out.
- `fa-server` publishes to `FA_NTFY_URL` with an optional `FA_NTFY_TOKEN` (bearer). Without `FA_NTFY_URL`, notifications are disabled and the settings screen says so. Settings → Notifications shows the subscribe URL and a QR code.
- Hosters who want more than topic secrecy enable ntfy's own access control (`auth-default-access: deny-all`, a write token for `fa-server`, read access per topic). `deploy/README.md` documents it; the app doesn't manage ntfy users.
- Producers (fa-tasks now, M3 later) insert outbox rows unconditionally, deduplicated by `dedupe_key`. `notify.send` runs every minute and decides: it drops rows whose trigger the member has turned off, holds rows during quiet hours (household default or the member's override), groups each member's remaining rows that are at least 60 seconds old into one message ("3 new tasks"), publishes, and retries failures with backoff (at most 5 attempts).
- **Children:** notifications go to the child's own topic. With `copy_to_member` set, the same message also goes to that adult, prefixed with the child's name.
- **Test:** Settings → Notifications has a "Send test" button.
- The existing `needs_reauth` state of calendar sources gets an adult-only trigger in M2 (cheap, and it uses the same path).

## 6. API (sketch; plan/M2.md fixes the final table)
| Method and path | Who | Purpose |
|---|---|---|
| `GET /api/v1/tasks?member=&list=&status=&now=true&from=&to=` | signed-in, dashboard | Lists with the computed score. `from`/`to` returns calendar-displayed tasks for the Calendar page |
| `POST /api/v1/tasks` | adults; children for `child_can_add = own` lists; dashboard for `dashboard_add` lists | One-off task |
| `PATCH /api/v1/tasks/{id}` | adults; children for their own one-offs | Edit, including `calendar_display` and steps |
| `POST /api/v1/tasks/{id}/complete`, `/reopen` | policy (§4.3) | Complete, undo |
| `POST /api/v1/tasks/{id}/skip`, `/accept` | adults | Skip, accept a suggested task |
| `POST /api/v1/tasks/{id}/claim`, `/release` | pool members | Claim |
| `PATCH /api/v1/tasks/{id}/steps/{step}` | as complete | Tick a step |
| `GET/POST/PATCH /api/v1/lists`, `/definitions`, `/routines` | adults (read: all) | Management |
| `GET /api/v1/routines/today` | signed-in, dashboard | Today's runs with progress |
| `PATCH /api/v1/routine-runs/{id}/steps/{step}` | the member, adults, dashboard | Tick a routine step |
| `GET/POST/DELETE /api/v1/members/{id}/away` | adults | Absences |
| `GET/PUT /api/v1/members/{id}/notify-prefs`, `POST .../test` | the member; adults for children | Preferences, test |

## 7. Authorization (adds to the RBAC matrix in Domain.md)
| Action | child | adult | dashboard device | device + adult PIN |
|---|---|---|---|---|
| Read tasks and routines | own and unassigned household tasks | ✓ | ✓ (all members) | ✓ |
| Complete a task or a step | own, per list policy | ✓ | ✓ while the override is on | ✓ |
| Add a one-off task | own, on `child_can_add = own` lists | ✓ | on `dashboard_add` lists | ✓ |
| Edit own one-off task | ✓ | ✓ | ✗ | ✗ |
| Manage lists, definitions, routines, absences | ✗ | ✓ | ✗ | ✗ |
| Skip, accept suggested | ✗ | ✓ | ✗ | ✗ (adult's own session) |
| Notification preferences | own | own and children's | ✗ | ✗ |

## 8. Configuration (D-018)
The household TOML gains `[[task_list]]`, `[[task_definition]]`, `[[routine]]`, and per-member `notify` tables (triggers and quiet hours, without topics: topics are regenerated on import). `config export` writes them back. The example household gets no task data (D-047).

## 9. Frontend (summary)
Frontend.md gets the M2 screens with the M2 plan:
- **App shell:** a Tasks page (Now, Mine, by list, Suggested for adults), task and definition editors, Settings → Tasks (lists, definitions, routines, absences) and Settings → Notifications.
- **Kiosk panels:** `now`, `member_tasks` (per-member columns), `routine` (progress bars), and task chips in `week` and `agenda` (§4.7).
