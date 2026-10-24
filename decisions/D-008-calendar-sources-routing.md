---
id: D-008
title: Multiple calendar sources; confidence-gated auto-add; configurable + learned target routing
status: accepted
date: 2026-10-23
source_questions: [Q-007]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/Ingest.md (future)]
---

# D-008: Multiple calendar sources; confidence-gated auto-add; configurable + learned target routing

## Context
Google, CalDAV, ICS feeds, and sports apps are all valid sources. The user picked option C: auto-add above a confidence threshold, otherwise propose. The target calendar is not fixed. It comes from configuration or learned rules.

## Decision
- Connector types:
  - `google` (read and write)
  - `caldav` (read and write)
  - `ics` (read-only subscription, which covers school and sports-app feeds)
- Calendars link to members.
- An extracted event gets a **confidence** score and goes through **target routing**:
  1. Explicit routing rules, such as sender, member, or category mapping to a calendar.
  2. Learned rules, which come from past approvals and edits (for example, sender X went to calendar Y the last N times).
  3. An LLM suggestion when the GPU node is available.
- If both the confidence and the routing certainty are at or above the threshold, the event is written. Otherwise it goes to an **approval inbox**.
- Each approval or correction is a training signal for the learned rules.
- Before any write, a dedupe check compares the normalised title, time window, and location similarity with existing events across all linked calendars.

## Consequences
- The approval inbox is a first-class UI surface on the dashboard and on phones.
- Thresholds can be set per household and per category.

## Alternatives considered
A single system-owned calendar. Rejected because the user wants events routed to the right calendar.
