---
id: D-036
title: Show all events; adults' private events appear as Busy
status: accepted
date: 2026-10-24
source_questions: [Q-034]
supersedes: 
superseded_by:
affects: [['arch/Feeds.md', 'arch/Frontend.md', 'arch/Domain.md']]
---

# D-036: Show all events; adults' private events appear as Busy

## Context
Q-034 asked how synced calendar events show on the shared dashboard and to children. The user answered: "For now let's show everything but show private for adults as busy." This is option A, plus masking for adults' private events. It drops the per-calendar `full | busy | hidden` setting that options B and C proposed.

## Decision
- **Scope (option A):**
  - Adults see every event (D-006).
  - The dashboard shows every enabled calendar.
  - A child sees events on calendars linked to them, plus household calendars (calendars linked to no member).
- **Masking:** an event marked private at its source (Google `visibility=private`/`confidential`, iCal `CLASS:PRIVATE`/`CONFIDENTIAL`) that **belongs to an adult** is shown as **Busy** to the dashboard and to children. Busy keeps the time block and member colour but replaces the title, location, and description with "Busy". Adult viewers always see the full event.
- **When an event belongs to an adult:** at least one of its linked members is an adult. A household calendar has no members, so the source owner counts instead; if there's no owner either, the event is treated as an adult's. A child's own private events are shown in full.
- **Configurable:** this is a household toggle, `private_events_as_busy` (default **on**), editable in Settings → Household. Turning it off shows everything in full ("for now" leaves room to change it later).
- There is no per-calendar visibility setting. An adult who wants a calendar off the dashboard can disable it, or not link it.

## Consequences
- `fa_domain::calendar::view()` returns `Full | Busy`. There is no `Hidden`, and the `calendar.dashboard_visibility` column is dropped from the M1 contracts.
- The M1 launch gate is lifted.
- A per-calendar or per-device visibility setting can be added later as a new question if needed.

## Alternatives considered
- B or C (a per-calendar full, busy, or hidden setting): deferred, because the user chose a simpler rule for now.
- Masking private events for everyone, adults included: not chosen, because adults read everything (D-006).
