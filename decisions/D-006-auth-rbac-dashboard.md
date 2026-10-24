---
id: D-006
title: Per-user logins with role-based access; shared dashboard with per-action member selection
status: accepted
date: 2026-10-23
source_questions: [Q-005]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/Frontend.md (future)]
---

# D-006: Per-user logins with role-based access; shared dashboard with per-action member selection

## Context
The dashboard is a shared device. The web and phone apps have a login for each user. Kids see only their own items. Adults see everything but can't change another adult's settings.

## Decision
- **Accounts:** a local username and password (argon2id) with session cookies (`tower-sessions`, stored in Postgres). Passkeys come later.
- **RBAC:**
  - `child` can read and act on their own calendar and tasks.
  - `adult` can read everything in the household and edit their own settings.
  - Changes to household-level settings and to children's settings are allowed for any adult.
  - An adult can't edit another adult's personal settings, sources, or credentials.
- **Dashboard device:** a long-lived `device` token, not a user. To act (complete a task, approve an event), the person picks who they are from member avatars. Whether this also needs a PIN is Q-018.
- The audit log records actor, device, and action.

## Consequences
- The API checks permissions in a central policy layer, not separately in each handler.

## Alternatives considered
One shared household login. Rejected because kids would get adult access.
