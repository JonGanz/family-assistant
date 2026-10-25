---
id: D-044
title: Task notifications: per-member triggers, quiet hours, children's copies, batching
status: accepted
date: 2026-10-25
source_questions: [Q-042]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Domain.md]
---

# D-044: Task notifications: per-member triggers, quiet hours, children's copies, batching

## Context
The user picked option C.

## Decision
- Per-member triggers. On by default: **assigned**, **overdue**, **morning digest**. Off by default: due-soon reminder, routine nudge.
- Quiet hours, default 21:00–07:00. Notifications in quiet hours are held until they end.
- Children have their own topic, and their notifications can optionally be copied to a chosen adult.
- A burst of notifications for one member is batched into one message.
- Everything is editable in Settings → Notifications.

## Consequences
- `notify_pref` gains quiet hours and `copy_to_member`. An outbox table holds pending notifications for batching and retries.

## Alternatives considered
Adults only with fewer triggers (A), or everything on (B). Not chosen.
