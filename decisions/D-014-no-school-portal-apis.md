---
id: D-014
title: No school/daycare portal integrations; ICS feeds + email only
status: accepted
date: 2026-10-23
source_questions: [Q-013]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-014: No school/daycare portal integrations; ICS feeds + email only

## Context
Portals differ too much from family to family. The user wants ICS feeds supported.

## Decision
- School and daycare data comes from email (the pipeline) and from ICS subscriptions (D-008 `ics` connector).
- There are no platform-specific API connectors such as ParentSquare or Brightwheel.

## Consequences
- The `Org` config should allow an ICS URL alongside the sender and domain lists.

## Alternatives considered
Per-platform connectors. Declined.
