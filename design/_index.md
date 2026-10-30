---
title: Design Questions Index
updated: 2026-10-29
---

# Design Questions

Open questions live here. Resolved ones move to `resolved/`.

## Template
```
---
id: Q-NNN
title: ...
status: open
batch: N
area: infra|tech|domain|ingest|frontend|security
created: YYYY-MM-DD
resolved:
decisions: []
---
# Q-NNN: title
## Context
## Question
## Options (optional)
## Answer
<!-- answer below this line -->
```

## Open
_None. The next question number is Q-078._

## Resolved
| ID | Title | Decisions |
|---|---|---|
| [Q-001](resolved/Q-001-hosting.md) | Host machine & uptime | D-002 |
| [Q-002](resolved/Q-002-stack.md) | Language / framework preferences | D-003 |
| [Q-003](resolved/Q-003-family.md) | Family composition | D-004 |
| [Q-004](resolved/Q-004-datastore.md) | Datastore & backups | D-005 |
| [Q-005](resolved/Q-005-auth.md) | Users, login & privacy between members | D-006 |
| [Q-006](resolved/Q-006-email.md) | Email accounts & processing scope | D-007 |
| [Q-007](resolved/Q-007-calendar.md) | Calendar sources & write-back target | D-008 |
| [Q-008](resolved/Q-008-bills.md) | Bill handling depth | D-009 |
| [Q-009](resolved/Q-009-access.md) | Remote access & notifications | D-010 |
| [Q-010](resolved/Q-010-echo.md) | Echo Show 15 kiosk approach | D-011 |
| [Q-011](resolved/Q-011-deals.md) | Deals of interest from spam/promotions | D-012 |
| [Q-012](resolved/Q-012-tasks.md) | Task assignment & accountability | D-013 |
| [Q-013](resolved/Q-013-school.md) | School / daycare portals | D-014 |
| [Q-014](resolved/Q-014-food.md) | Meal planning & grocery details | D-015 |
| [Q-015](resolved/Q-015-llm-latency.md) | Acceptable LLM latency & batching | D-016 |
| [Q-016](resolved/Q-016-weather.md) | Weather location & use | D-017 |
| [Q-017](resolved/Q-017-config-source.md) | Configuration source of truth | D-018 |
| [Q-018](resolved/Q-018-dashboard-identity.md) | Dashboard member selection security | D-019 |
| [Q-019](resolved/Q-019-gpu-node-availability.md) | GPU node wake / schedule policy | D-020 |
| [Q-020](resolved/Q-020-core-node-specs.md) | Core node hardware & CPU fallback model | D-016 |
| [Q-021](resolved/Q-021-grocery-model.md) | Grocery list model, pantry & dietary needs | D-021 |
| [Q-022](resolved/Q-022-core-node-specs.md) | Core node hardware specs | D-022 |
| [Q-023](resolved/Q-023-mvp-roadmap.md) | MVP scope & build order | D-022, D-023 |
| [Q-024](resolved/Q-024-s1-python-sidecar.md) | Python runtime exception for System One models | D-025 |
| [Q-025](resolved/Q-025-s1-placement.md) | Where System One models run | D-026 |
| [Q-026](resolved/Q-026-s1-scope.md) | Other decisions System One should make | D-027 |
| [Q-027](resolved/Q-027-knn-layer.md) | Keep the correction-learning kNN layer? | D-028, D-029 |
| [Q-028](resolved/Q-028-email-retention.md) | Email content retention & privacy | D-030 |
| [Q-029](resolved/Q-029-child-login.md) | How children sign in | D-031 |
| [Q-030](resolved/Q-030-code-hosting-ci.md) | Code repository & CI | D-032 |
| [Q-031](resolved/Q-031-update-mechanism.md) | How updates are deployed | D-033 |
| [Q-032](resolved/Q-032-email-metadata-kept.md) | What email metadata survives the content purge | D-034 |
| [Q-033](resolved/Q-033-repo-visibility.md) | Repository visibility & license | D-035 |
| [Q-034](resolved/Q-034-event-visibility.md) | Who sees which calendar events (dashboard, children) | D-036 |
| [Q-035](resolved/Q-035-recurrence-anchor.md) | When the next occurrence of a recurring task is due | D-037 |
| [Q-036](resolved/Q-036-missed-occurrences.md) | What happens to recurring tasks that aren't done in time | D-038, D-048 |
| [Q-037](resolved/Q-037-rotation-rules.md) | Rotation details: who did it, skips, absences | D-039 |
| [Q-038](resolved/Q-038-pool-claiming.md) | Pool tasks: claim first, or just do it | D-040 |
| [Q-039](resolved/Q-039-routines.md) | Children's morning (and other) routines | D-041 |
| [Q-040](resolved/Q-040-now-list-priority.md) | The "now important" list: what goes on it and in what order | D-042 |
| [Q-041](resolved/Q-041-seasonal-weather-tasks.md) | Seasonal and weather-triggered tasks | D-043 |
| [Q-042](resolved/Q-042-notifications.md) | Which task events send phone notifications (ntfy) | D-044 |
| [Q-043](resolved/Q-043-task-permissions.md) | Who can create and edit tasks and lists | D-045 |
| [Q-044](resolved/Q-044-tasks-in-calendar.md) | Do tasks appear in the calendar and agenda | D-046 |
| [Q-045](resolved/Q-045-starter-templates.md) | Starter chores and routines | D-047 |
| [Q-046](resolved/Q-046-missed-schedule-default.md) | Default for missed schedule-anchored tasks | D-049 |
| [Q-047](resolved/Q-047-goals-milestone.md) | When to build goals | D-050 |
| [Q-048](resolved/Q-048-goals-model.md) | What a goal looks like | D-051 |
| [Q-049](resolved/Q-049-mail-connector.md) | Gmail API only, or also IMAP? | D-052 |
| [Q-050](resolved/Q-050-which-mail.md) | Which mail does the system read? | D-053 |
| [Q-051](resolved/Q-051-categories.md) | Email categories and what each one does | D-054 |
| [Q-052](resolved/Q-052-school-relevance.md) | Filtering school and daycare mail per child: grade, age, gender? | D-055 |
| [Q-053](resolved/Q-053-contact-linking.md) | How do teachers and coaches get linked to a child? | D-056 |
| [Q-054](resolved/Q-054-event-auto-add.md) | Which extracted events are added to calendars without asking? | D-057 |
| [Q-055](resolved/Q-055-gmail-actions.md) | What may the system do in Gmail when a source isn't read-only? | D-058 |
| [Q-056](resolved/Q-056-bill-details.md) | Bills: who pays, autopay, and detecting payment | D-059 |
| [Q-057](resolved/Q-057-email-tasks.md) | Tasks from email: 'reply', 'sign', 'RSVP' | D-060 |
| [Q-058](resolved/Q-058-approval-inbox.md) | The approval inbox: notifications and expiry | D-061 |
| [Q-059](resolved/Q-059-labelling-scope.md) | The labelling view: what you label and how messages are picked | D-062 |
| [Q-060](resolved/Q-060-mail-visibility.md) | Who can see which messages? | D-063 |
| [Q-061](resolved/Q-061-next-milestone-order.md) | What comes next: real-hardware shakedown, M4 AI, or M5 food? | D-066 |
| [Q-062](resolved/Q-062-m4-scope.md) | How big is M4: minimal AI slice or the full registry design? | D-067 |
| [Q-063](resolved/Q-063-device-inbox-and-advisories.md) | What may an unelevated dashboard device do: read the approval inbox, dismiss advisories? | D-068, D-078 |
| [Q-064](resolved/Q-064-trusted-proxy-default.md) | Which peers may set the client IP (X-Forwarded-For)? | D-069 |
| [Q-065](resolved/Q-065-members-page-placement.md) | Where does the Members page live, and what do children see there? | D-070 |
| [Q-066](resolved/Q-066-interests-consumer.md) | What should use member interests, hobbies and responsibilities? | D-071 |
| [Q-067](resolved/Q-067-lan-exposure.md) | Without Tailscale, what is reachable on the home network over plain HTTP? | D-072 |
| [Q-068](resolved/Q-068-first-run-protection.md) | Who may run the first-run wizard? | D-073 |
| [Q-069](resolved/Q-069-secrets-and-key-backup.md) | How are the master key and database password created, and how is the key backed up? | D-074 |
| [Q-070](resolved/Q-070-settings-in-ui.md) | Which server settings move from environment variables into the UI? | D-075 |
| [Q-071](resolved/Q-071-container-layout.md) | How many app containers does the default compose file run? | D-076 |
| [Q-072](resolved/Q-072-image-distribution.md) | How do households get the app: a pulled image or a local build? | D-077 |
| [Q-073](resolved/Q-073-theme-catalogue.md) | Which colour themes does the app ship, and how do light and dark pair up? | D-079 |
| [Q-074](resolved/Q-074-member-colour-hues.md) | Do members pick a named hue instead of a hex colour? | D-080 |
| [Q-075](resolved/Q-075-theme-scope.md) | Who chooses the theme: household, member, or device? | D-081 |
| [Q-076](resolved/Q-076-custom-themes.md) | Can a household add its own themes? | D-079 |
| [Q-077](resolved/Q-077-adding-another-adult.md) | How is a second adult added and set up? | D-088 |
