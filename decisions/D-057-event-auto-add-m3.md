---
id: D-057
title: M3 adds no extracted event automatically; every event waits for approval
status: accepted
date: 2026-10-25
source_questions: [Q-054]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-057: M3 adds no extracted event automatically; every event waits for approval

## Context
D-008 writes an event when both the extraction and routing confidences pass a threshold. M3 extraction is deterministic only.

## Decision
In M3 every extracted event becomes a **proposal** in the approval inbox, including ICS invites and schema.org events. The D-008 gate is still built and tested, behind a household setting `events.auto_write` that defaults to **off**; the thresholds (default 0.9 / 0.9) are kept for when it is turned on. The default is revisited when M4 brings better extraction.

## Consequences
- Calendar writes happen only on approval in M3.
- D-008 stands; this sets its M3 default.

## Alternatives considered
- ICS and schema.org from known orgs automatically.
- Thresholds per category, default 0.9.
- Everything with a date.
