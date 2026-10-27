---
id: D-071
title: Member interests, hobbies and responsibilities feed LLM features only
status: accepted
date: 2026-10-26
source_questions: [Q-066]
supersedes: 
superseded_by:
affects: [arch/Domain.md, arch/Overall.md]
---

# D-071: Member interests, hobbies and responsibilities feed LLM features only

## Context
PROJECT.md says these details guide intelligent decisions. They are stored but nothing reads them (review R-07).

## Decision
They are consumed **only by the LLM features** (System 2, from M4), which use them to drive automations. They are not used as rules inputs: not mail relevance tags (D-055 tags remain the deterministic mechanism) and not the Now-list score.

## Consequences
- They stay unread until M4; R-07 is accepted as by design.
- M4/M4.1 designs (AI.md) must say which prompts include them, and children's details stay inside the household (local LLM only).

## Alternatives considered
- A, mail relevance now.
- C, M6 smart assignment rules.
