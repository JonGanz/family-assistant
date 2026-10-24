---
id: D-012
title: Deals of interest removed from scope
status: accepted
date: 2026-10-23
source_questions: [Q-011]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-012: Deals of interest removed from scope

## Context
The user decided deal detection felt fragile and removed it.

## Decision
- Spam and promotions are classified and **ignored**. Nothing is extracted from them.
- `extracted.deals` and any deal UI are removed from the design.

## Consequences
- The pipeline is simpler, and spam needs no LLM work at all.

## Alternatives considered
Keeping deals as a low-priority digest. Declined.
