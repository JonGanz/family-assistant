---
id: D-086
title: Calendars have no colour; an event is coloured by its members
status: accepted
date: 2026-10-29
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Feeds.md, arch/Domain.md]
---

# D-086: No calendar colours

## Context
Direct user instruction (2026-10-29, no question). A calendar linked to two members showed only one colour on the dashboard, and the user does not want colours tied to calendars at all. This overrides the part of D-080 that kept calendar colours.

## Decision
- **Calendars no longer have a colour**: the column, the API field, the settings picker and the colour read from CalDAV are removed. An event's colour comes only from the members linked to it.
- An event for several members shows **every member's colour**, as equal bands in its left stripe, wherever events from several members are combined into one entry (Home agenda, the calendar page). An event for nobody uses the neutral fallback.
- In the **Week view**, an event sits in the column of each member it belongs to and is drawn in **that member's** colour.
- `EventView` loses `color` and `colors`; `merged` stays (duplicate events from several calendars still fold into one).

## Consequences
- D-080's statement that calendars keep their hex colours no longer holds. Member hues are the only event colours.
- Migration `20261029000003_drop_calendar_color` drops the column; the down migration adds it back empty. Colours that users had set on calendars are lost.
- A `color` key on a calendar in a household TOML is still accepted on load, ignored, and never written back, so older files keep loading.
