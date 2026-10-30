---
id: D-085
title: The dashboard Home view shows compact weather and events, with details in a modal
status: accepted
date: 2026-10-29
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-085: Compact Home view, details in a modal

## Context
Direct user instruction (2026-10-29, no question), refining D-082 and D-084.

## Decision
- The connection status, stale badge, last heartbeat and the adult unlock button move to the **end of the navigation bar**, on every view. D-082's "hidden on the Week view" no longer applies.
- **Weather panel (Home):** row one is the icon, the current temperature, "feels like / wind" and, before noon (household time), today's high, low and rain chance. Row two holds the advisories and the thunderstorm notice. The hourly and multi-day strips leave the panel. Tapping the panel opens a **forecast modal**: today by the hour (scrolls sideways) and the upcoming days, as many as the snapshot holds.
- **Agenda events (Home):** each shows the time and the member chips under it in one fixed-width column, and the title in the next column, so titles line up whatever the time range. The location is not shown. Tapping an event opens a **modal** with the date and time, location, members, notes and the link to the original.
- Events in the **Week view** open the same modal.
- The calendar page's event card is unchanged.

## Consequences
- The panel option `hourly` no longer has an effect.
- Panels ask the canvas for a modal (`detail` event); the canvas draws it inside its own root so the theme variables apply.
