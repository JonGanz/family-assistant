---
id: D-062
title: Labelling covers all four email decision points with a suggested balanced queue
status: accepted
date: 2026-10-25
source_questions: [Q-059]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Frontend.md]
---

# D-062: Labelling covers all four email decision points with a suggested balanced queue

## Context
D-028 needs a ~200-message eval sample for M4.

## Decision
The labelling view shows one message at a time with `email.category`, `email.time_sensitive`, `email.member_link` and `email.is_bill` pre-filled from the current decisions. The system suggests the next message from a queue stratified by category and by rule-decided vs fell-to-default. Any message can also be labelled from the message view.

## Consequences
Most messages take one or two taps. Each confirmation is a `human` Decision.

## Alternatives considered
- Category only.
- Browse freely.
