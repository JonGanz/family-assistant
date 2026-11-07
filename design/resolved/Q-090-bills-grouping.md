---
id: Q-090
title: "Bills: grouped by state, and a state filter"
status: resolved
batch: 23
area: frontend
created: 2026-11-04
resolved: 2026-11-07
decisions: [D-105]
---

# Q-090: Bills: grouped by state, and a state filter

## Question
The new Bills page groups bills by state (open, proposed, paid, dismissed). The old page also had a state filter.

## Options
- A. Groups plus filter chips (applied)
- B. Groups only
- C. Filter only, a flat list

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-12). Time-sensitive mail uses the `warning` icon.

## Answer
<!-- answer below this line -->
Option A. There is however a noticable inconsistency I've seen. These options here, as well as for the calendar and task views where you can select which members to filter by, should be additive. Item 1 would be "All", which unchecks all others, then Item 2...n would be the groups, members, etc. This section should be multiselect, to turn on/off each option in order to view, as an example, bills that are either Proposed or Open.
