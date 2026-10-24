---
id: D-002
title: Split hosting: always-on server + on-demand GPU node; LLM is an enhancement
status: accepted
date: 2026-10-23
source_questions: [Q-001]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-002: Split hosting: always-on server + on-demand GPU node; LLM is an enhancement

## Context
The RTX 3070 is in a gaming PC that is not on 24/7. The user picked option B and said the system should work on its own, with LLMs making it better.

## Decision
- **Core node:** an always-on Ubuntu Server host. It runs the API, workers, DB, notifications, and the web assets.
- **GPU node:** the gaming PC running Ollama. It is used when it's available. The details of waking it or keeping a schedule are in Q-019.
- All LLM work goes through a durable **LLM job queue** in Postgres. Items that need the LLM get `enrichment: pending|done|skipped`. They show up straight away with their rule-based results, and the LLM results are added when the GPU node next runs.
- Every feature must keep a deterministic path that works with no LLM.

## Consequences
- The rules engine is now the main classifier, not a pre-filter. It needs good sender, domain, and keyword configuration, and should learn from user corrections.
- An LLM gateway component watches GPU node health and drains the queue in batches.
- Q-015 (latency) is mostly settled by this: batched, best effort.

## Alternatives considered
A: run everything on the GPU box. Rejected because it isn't always on.
