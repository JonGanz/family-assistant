---
id: D-068
title: The dashboard is at-a-glance only; advisories clear themselves and can't be dismissed
status: accepted
date: 2026-10-26
source_questions: [Q-063]
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Feeds.md, arch/Ingest.md, arch/Domain.md]
---

> **Refined by [D-078](D-078-private-approvals.md) (2026-10-26):** approvals are private to the adults who may see the source mail; the dashboard may show a count badge per adult.

# D-068: The dashboard is at-a-glance only; advisories clear themselves and can't be dismissed

## Context
Review findings C-08 (any device could read the approval inbox) and C-09 (anyone could dismiss weather advisories). The user's answer went further than the options.

## Decision
- **Weather advisories cannot be dismissed** by anyone. An advisory is removed when it is no longer relevant: when its `valid_until` passes, or when `weather.evaluate` finds the rule no longer matches the forecast. The `DismissAdvisory` action, the dismiss endpoint and the `dismissed_*` fields go.
- **The dashboard shows at-a-glance material only:** date, time, weather (with advisories), agenda, tasks and routines. **Nothing derived from email is on it**: no inbox panel, no counts, no approvals, elevated or not. Devices (elevated or not) are refused `ReadInbox`.
- Approvals happen in the app (`/inbox`) on phones and desktops.

## Consequences
- Amends D-008 (the inbox was 'a first-class surface on the dashboard and on phones') and the M3 DoD item 'approve/reject from an elevated dashboard'.
- PIN elevation (D-019) remains for the other adult actions on the dashboard (task overrides).
- Fixes C-08 and C-09 by removal rather than permission checks.

## Alternatives considered
- A, configurable inbox and dismiss permissions: rejected; the user wants the dashboard free of mail, and advisories self-managing.
