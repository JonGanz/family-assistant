---
title: Domain Model
scope: domain (all bounded contexts, persistence conventions)
status: draft
updated: 2026-10-26
drawio: Domain.drawio
drawio_src: src/Domain.py
depends_on: [D-004, D-006, D-007, D-008, D-009, D-013, D-037, D-038, D-039, D-040, D-041, D-044, D-045, D-046, D-050, D-051, D-015, D-017, D-018, D-019, D-021, D-024, D-026, D-027, D-028, D-029, D-031, D-034, D-036, D-052, D-053, D-054, D-055, D-056, D-057, D-058, D-059, D-060, D-061, D-062, D-063, D-068, D-070, D-071, D-078]
milestone: M0
---

# Domain Model (DRAFT v5.1)

This is the full UML class model, split into packages (bounded contexts). Each package below matches a page in `Domain.drawio`. The model covers every milestone, but **M0 only implements Identity and Audit**. The other packages appear in migrations when their milestone arrives. Items marked **[Q-0xx]** depend on an open question.

## Persistence conventions (all tables)
| Rule | Detail |
|---|---|
| IDs | `UUIDv7` (time-ordered) via the `uuid` crate, in the `uuid` column type. Each Rust newtype has its own ID type (`MemberId(Uuid)`) |
| Tenancy | Every household-scoped table has `household_id NOT NULL`. **Postgres row-level security** checks `household_id = current_setting('app.household_id')`, which is set for each transaction by `fa-store`. This is defence in depth on top of the policy layer (D-004, D-006) |
| Time | Instants are `timestamptz` (UTC). All-day dates are `date`. Display uses `Household.tz`. Recurrence is evaluated in the household tz (`chrono-tz`) |
| Enums | `text` + `CHECK` in SQL, mapped to Rust enums via `sqlx::Type`. This makes migrations easier than native PG enums |
| Flexible data | `jsonb` only for opaque external payloads (Gmail headers, weather blobs, stage config). Never for data we query relationally |
| Audit columns | `created_at`, `updated_at`, and `created_by` (MemberId, nullable for system) on mutable tables |
| Deletion | Config entities (members, lists, rules, sources) are soft-deleted with `archived_at`. **Email content** follows the configurable retention policy (D-034). Weather snapshots are hard-deleted by a retention job |
| Provenance | Every derived entity has a `SourceRef{kind: email\|ics\|weather\|manual\|system, message_id?, external_url?}`, which the UI shows as a 'from ↗' link (D-030 → D-034) |
| Secrets | Never stored in an entity column. They go through `Secret` (envelope-encrypted, see Platform.md) |
| Versioning | Optimistic concurrency with a `version int` on entities edited from several clients (tasks, grocery items, pipelines) |

## Package: Identity & Access (M0)
| Class | Key attributes | Notes |
|---|---|---|
| `Household` | name, tz, location{lat, lon, label}, meal_plan_horizon_weeks, dashboard_completion_override | One per install by default. The schema allows several (D-004) |
| `Member` | display_name, role: `adult\|child`, avatar, color (one of 8 hue names: red, orange, yellow, green, cyan, blue, purple, pink; D-080), birthdate?, grade?, aliases[] (M3), gender? `female\|male\|other` (M3, optional, used only for mail relevance, D-055), relevance_tags[] (M3, D-055), interests[], hobbies[], responsibilities[] (read only by LLM features from M4, never as rules inputs, D-071), pin_hash? (adults only) | The central entity. Has no login unless it has a `UserAccount`. **Only adults create or edit members** (D-070); children see the member list on the dashboard |
| `UserAccount` | 0..1 per Member. username, password_hash (argon2id), disabled_at? | Adults set up and reset child accounts. Child passwords need 4+ characters, adults 12+ (D-031) |
| `Session` | account, created/last_seen, user_agent, expires_at | Stored in Postgres. Rolls for 180 days for children and 30 days for adults. Adults can revoke child sessions (D-031) |
| `Device` | kind: `dashboard`, name, token_hash, last_seen, revoked_at? | A shared kiosk identity, not a user (D-006) |
| `DeviceElevation` | device, member (adult), expires_at | Created by a PIN step-up and lasts about 5 minutes (D-019) |
| `NotifyPref` | member, topic (random), triggers, quiet hours override, copy_to_member? | D-010, D-044. Detail in [Tasks.md](Tasks.md) §5 |
| `AuditEntry` | actor_member?, device?, action, subject_type, subject_id, before/after jsonb, at | Append-only, with no UPDATE or DELETE grants |
| `Secret` | owner_type, owner_id, kind, ciphertext, nonce, key_version | Envelope encryption |

**Authorization (RBAC, D-006):**
| Action | child | adult | dashboard device | device + adult PIN |
|---|---|---|---|---|
| Read own tasks and calendar | ✓ | ✓ | ✓ (all members) | ✓ |
| Read all household data | ✗ | ✓ | dashboard subset | dashboard subset |
| Complete a task or routine step | own (if the list policy allows) | ✓ | ✓ when the override is on (D-013) | ✓ |
| Add a one-off task | own, on lists with `child_can_add = own` (D-045) | ✓ | on lists with `dashboard_add` | ✓ |
| Manage lists, definitions, routines, absences; skip | ✗ | ✓ | ✗ | ✗ |
| Add a grocery item | ✓ | ✓ | ✓ | ✓ |
| Manage members (details, accounts, PINs) (D-070) | ✗ | ✓ for children, themself and an **unclaimed** adult (no account yet, or still on the password another adult set, D-088); never a claimed adult. PINs: own only | ✗ | ✗ |
| See members (names, avatars, colours) | on the dashboard | ✓ | ✓ | ✓ |
| Read the approval inbox; approve items; mail actions (D-068, D-078) | ✗ | items from sources they may see | ✗ (pending count per adult only) | ✗ (count only) |
| Household and child settings, pipelines | ✗ | ✓ | ✗ | ✗ |
| Another adult's personal settings and sources | ✗ | ✗ | ✗ | ✗ |

Weather advisories have no dismiss action for anyone: they clear themselves (D-068, [Feeds.md](Feeds.md) §4). Devices never see email-derived data (D-068).

## Package: Organizations & Mail (M3)
Table-level detail is in [Ingest.md](Ingest.md) §3.

| Class | Key attributes |
|---|---|
| `Org` | kind: `school\|daycare\|club\|biller\|other`, name, domains[], senders[], calendar_source? (ICS, D-014), default_calendar?, contact_mode: `suggest\|auto\|manual` (D-056) |
| `OrgContact` | org, name, email, role: `teacher\|coach\|admin\|office\|other` |
| `ContactSuggestion` | org, email, name?, member?, message, status: `proposed\|accepted\|rejected\|expired` (D-056) |
| `Enrollment` | member ↔ org, grade?, class?, from?, until?, contacts[] (OrgContact). This is how a teacher's email gets linked to a child |
| `MailSource` | provider: `gmail\|imap` (D-052), address, owners: Member[1..*], visibility: `owners\|household` (D-063), read_only=true, backfill_days, query?, poll_minutes, Gmail: oauth_account, history_id?; IMAP: host, port, username, secret, folders[], archive_folder?; state: `ok\|needs_reauth\|error\|paused` (D-007) |
| `Message` | source, remote_id, thread_id?, external_url?, received_at, gmail_labels[], from, to[], subject (**metadata, always kept**), category?, time_sensitive?, is_bill?, embedding? (kept while kNN is enabled), **policy-governed content:** body_text?, snippet?, headers jsonb?, attachments jsonb?, is_learning_example, in_eval_sample, processing_state: `pending\|processed\|expired`, content_purged_at? (D-034) |
| `MessageMember` | message ↔ member, decided_by: `rule\|user` |
| `Category` | key, label, behaviour: `ignore\|keep\|extract`, notify_time_sensitive, auto_actions[], email_tasks: `suggest\|create\|off` (D-054, D-058, D-060) |
| `MailRule` | decision_point, priority, conditions[], value, confidence, origin: `builtin\|user` |
| `Bill` | message?, org?, payee, amount_minor, currency, due_on?, account_hint?, status: `proposed\|open\|paid\|dismissed`, confidence, task (adult pool, D-059), source_ref |
| `GmailAction` | message, action: `archive\|mark_read\|label`, automatic, done_by?, done_at, undone_at? (D-058; also used for IMAP) |

The eval sample and user labels are `Decision` rows from the `human` stage plus `Message.in_eval_sample` (D-028, D-062). A message with a user decision or `in_eval_sample` is a learning example (D-034).

**Message lifecycle (D-034):** `pending` (content held) → all decision points are final and S2 extraction is done → `processed`. The daily `mail.retention` job then applies `retention.mail.content_policy`:
- `purge_after_processing`: purge all content.
- `keep_for_learning` (the default while kNN or eval is enabled): keep content for learning examples (user-corrected messages, the eval sample, and the newest N rule/S2-labelled messages per category), and purge the rest.
- `keep_all`.

A message still pending after `retention.mail.max_pending_days` (default 14) becomes `expired`, its pending stages are skipped, and then the policy applies. Purged content is re-fetched from the mailbox when needed. Embeddings are dropped when kNN is disabled.

`Household.settings.retention.mail{content_policy, examples_per_category=200, max_pending_days=14}`

The decision results for a message (category, time_sensitive, member links, is_bill) are **`Decision` rows** in the AI package. A message keeps denormalised `category` and `time_sensitive` columns for fast queries, and the pipeline keeps them updated.

## Package: Calendar (M1 read, M3 write)
| Class | Key attributes |
|---|---|
| `OAuthAccount` | provider: `google`, owner Member, email, scopes[], secret, state (shared with Gmail in M3) |
| `CalendarSource` | kind: `google\|caldav\|ics`, owner Member?, oauth_account? / url, username? / secret? (CalDAV password or the ICS URL), writable, poll_minutes, state |
| `Calendar` | source, remote_id, name, enabled, members[] (M:N), sync_state, is_default_target |
| `Event` | calendar?, remote_uid?, instance_key, title, start_at, end_at, all_day (+ start_date/end_date), location?, is_private, cancelled, members[] (M:N, M3), origin: `synced\|extracted\|manual`, status? (`null` when synced; `proposed\|approved\|written\|rejected`), confidence?, source_ref, dedupe_key |

Synced events are stored **one row per occurrence** (recurrences are expanded in the sync window). Visibility (D-036): everything is shown, but adults' source-private events appear as Busy to the dashboard and children. Full table detail, sync, and visibility rules are in [Feeds.md](Feeds.md) §2–3.

## Package: Tasks (M2)
| Class | Key attributes | Notes |
|---|---|---|
| `TaskList` | name, kind: `chores\|homework\|general` (+ `email` in M3), completion_policy: `assignee\|adult_only`, child_can_add, dashboard_add, calendar_default, claim_ttl | D-013, D-045, D-046 |
| `TaskDefinition` | list, title, schedule: `rrule` or `after_done` interval, lead_time, on_missed, strategy: `fixed\|rotation\|pool\|smart`, candidates[] (ordered Members), rotation_cursor, advance_on, skip_advances, priority_flag, season window and condition?, last_done_at, next_due_at | D-037 to D-039, D-043 |
| `Task` | list, definition?, weather_rule?, title, notes, steps[] (TaskStep), assignees[], due_at?, visible_from?, priority_flag, urgency? (1–5, M4, D-027), status: `open\|done\|skipped\|suggested`, origin: `manual\|email\|system\|weather`, calendar_display, source_ref, completed_by?, completed_via_device?, claimed_by? | One occurrence. The Now score is computed at read time (D-042) |
| `TaskStep` | task, order, text, done_at? | Homework steps and preparation for key dates |
| `Routine`, `RoutineStep`, `RoutineRun`, `RoutineRunStep` | members, window, day filter (school days from a calendar), steps with target times; one run per member per qualifying day | D-041 |
| `MemberAway` | member, from, until | D-039. Every strategy honours it |

Full tables, scheduling, strategies, and notifications are in [Tasks.md](Tasks.md).

**Invariants:**
- A `rotation` definition advances `rotation_cursor` only on completion or skip, as `advance_on` and `skip_advances` allow (D-039).
- A `fixed` definition has exactly one candidate.
- Completing a task stamps `last_done_at` on its definition. For `after_done` schedules the next occurrence is due `interval` after that date; `rrule` schedules follow the calendar (D-037).

## Package: Goals (M7)
| Class | Key attributes | Notes |
|---|---|---|
| `Goal` | member, title, measure: `count\|quantity`, unit?, target, cadence: `once\|recurring`, period (default `week`), reset_on (weekday or date), starts_on, ends_on?, created_by (adult), show_on_dashboard | D-051. Only adults create and edit |
| `GoalProgress` | goal, period_start, amount, recorded_by, recorded_at, note? | Progress for a period is the sum of entries since the last reset |

Progress sources beyond manual entry, and the privacy of adults' goals, are settled before M7 (`Goals.md`).

## Package: Meals (M5)
| Class | Key attributes |
|---|---|
| `Recipe` | title, source_url?, servings?, image_url?, import_method: `jsonld\|microdata\|llm\|manual` |
| `RecipeIngredient` | recipe, raw_text, qty?, unit?, ingredient?, note? |
| `RecipeStep` | recipe, order, text |
| `Ingredient` | canonical_name, aliases[], is_staple, default_unit? (D-021) |
| `MealPlanEntry` | date, slot: `breakfast\|lunch\|dinner\|snack`, recipe? or free_text, servings? |
| `GroceryList` | exactly one per household |
| `GroceryItem` | list, ingredient?, text, qty?, unit?, source_meals[] (M:N MealPlanEntry), manual, added_by, added_via_device?, checked_at? |

## Package: Weather (M1)
| Class | Key attributes |
|---|---|
| `WeatherSnapshot` | fetched_at, provider, current jsonb, hourly jsonb, daily jsonb (always metric) |
| `WeatherRule` | name, condition (a small DSL such as `min(temp_c, next 12h) <= 0`), action: `alert\|task`, task_definition?, status: `active\|suggested\|disabled`, suggested_by: `default\|s2` (D-017) |
| `Advisory` | rule?, text, severity, valid_from/until, source: `rule\|s2`, dedupe_key. Not dismissable; removed when `valid_until` passes or its rule stops matching (D-068) |

The rule DSL, default rules, and evaluation are in [Feeds.md](Feeds.md) §4. M1 also adds `Household{units, calendar_past_days, calendar_future_days, weather_retention_days, private_events_as_busy, week_start}`, `MemberAvatar`, and `Device.settings` (dashboard layout, [Frontend.md](Frontend.md) §4).

## Package: AI & Pipelines (M4. M3 has rules-only)
| Class | Key attributes |
|---|---|
| `Node` | kind: `core\|gpu`, name, hardware{cpu, cores, ram_mb, gpu?, vram_mb?}, state: `available\|busy_gaming\|asleep\|unknown`, wol_mac?, last_heartbeat (D-020, D-026) |
| `Backend` | node, kind: `system_one\|llm\|embedding`, endpoint, model, dialect: `systemone_v1\|ollama`, capabilities[], availability: `always\|on_demand`, enabled |
| `CatalogModel` | name, kind, min_ram_mb, min_vram_mb, license, notes (data shipped with releases) |
| `DecisionPoint` | key, e.g. `email.category` (D-027), question_kind: `choice\|score\|noul`, options_source |
| `PipelineVersion` | decision_point, version, active, created_by, dry_run_report? |
| `Stage` | pipeline_version, order, kind: `rules\|knn\|system_one\|llm\|human`, backend_prefs[], accept_threshold, on_unavailable: `skip\|wait\|queue`, config jsonb, enabled |
| `Decision` | decision_point, subject_type, subject_id, stage, pipeline_version, value, probabilities jsonb, decided_at |
| `LlmJob` | kind, subject, backend?, status: `queued\|running\|done\|failed`, attempts, result jsonb |
| `EvalRun` | target (backend or pipeline_version), sample_size, accuracy, ece, p50_ms, report jsonb |

**Invariant:** in an active `PipelineVersion`, the last enabled stage must be `rules` or `human` (D-029).

## Package: Notifications (M2)
`Notification{member, kind, subject, title, body, priority, sent_at, channel, status}` is written before it is sent, so failures can be retried and audited.

## Open
_None._
