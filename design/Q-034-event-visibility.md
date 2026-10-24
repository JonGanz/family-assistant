---
id: Q-034
title: Who sees which calendar events (dashboard, children)
status: open
batch: 13
area: frontend
created: 2026-10-24
resolved:
decisions: []
---

# Q-034: Who sees which calendar events (dashboard, children)

## Question
M1 brings synced calendars onto the shared dashboard and into children's views. Some events are personal: an adult's medical appointment, a gift-shopping trip, a private calendar. Google and CalDAV/ICS mark some events as private (`visibility=private`, `CLASS:PRIVATE`/`CONFIDENTIAL`).

Today's rules (D-006) say a child sees only their own items and the dashboard shows all members' calendars. They don't say how an event's **detail** is shown, or which calendars count as a child's.

**How should event visibility work on the dashboard and for children?**

## Options
- A. Show everything: every synced event appears in full on the dashboard. A child sees events on calendars linked to them, plus calendars linked to no one (household calendars).
- B. Per-calendar setting `dashboard: full | busy | hidden` (busy shows only the time block, labelled "Busy" with the member's colour). Events marked private at the source are always shown as busy, whatever the calendar setting. Children follow A's rule, with the same masking.
- C. **Configurable, default B with `full`** (recommended): a per-calendar setting with the default `full`, plus a household toggle "treat source-private events as busy" (default on). Adults set both in Settings → Calendars. Children follow A's rule, with the same masking.
- D. Something else (describe).

## Answer
<!-- answer below this line -->

