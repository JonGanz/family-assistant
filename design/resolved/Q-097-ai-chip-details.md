---
id: Q-097
title: How the inbox shows AI-made items
status: resolved
batch: 23
area: frontend
created: 2026-11-04
resolved: 2026-11-07
decisions: [D-104]
---

# Q-097: How the inbox shows AI-made items

## Question
Items the local LLM proposed carry an 'AI' chip. Task proposals have no confidence value. Should there also be a way to see only AI items?

## Options
- A. Chip 'AI · 82%' (just 'AI' for tasks), a tooltip on wide screens and a sheet on phones; an 'AI only' filter chip in the Inbox, not remembered (applied)
- B. Chip only, no filter
- C. As A, plus confidence on tasks (needs a backend field)

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-16). Contacts carry no provenance field yet.

## Answer
<!-- answer below this line -->
Option A.
