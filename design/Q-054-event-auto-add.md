---
id: Q-054
title: Which extracted events are added to calendars without asking?
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-054: Which extracted events are added to calendars without asking?

## Question
D-008: an extracted event is written when **both** the extraction confidence and the routing certainty are above a threshold. Otherwise it goes to the approval inbox. M3 has no LLM, so extraction is deterministic:
- **ICS invites** and **schema.org data** (booking confirmations) are near-certain (0.9–0.95);
- **date phrases** in plain text ("Picture day is Friday, Nov 7") are guesses (0.5–0.7).

Routing in M3 uses rules only: the org's default calendar, then the linked child's default calendar, then the household default.

**What should be added automatically in M3?**

## Options
- A. Nothing. Every extracted event waits for approval until M4 brings better extraction.
- B. ICS invites and schema.org events from known orgs only. Everything else is approved by hand.
- C. **Configurable thresholds per category, default: write at ≥ 0.9 extraction and ≥ 0.9 routing** (recommended). That means ICS and schema.org events with a certain target go in automatically, and date-phrase guesses always wait for approval. Adults can raise or lower the thresholds per category.
- D. Everything with a date goes in automatically, and you delete mistakes.
- E. Something else (describe).

## Answer
<!-- answer below this line -->

