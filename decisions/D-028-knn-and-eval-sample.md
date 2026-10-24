---
id: D-028
title: Keep the kNN correction-learning stage; build a ~200-email labelled eval sample
status: accepted
date: 2026-10-23
source_questions: [Q-027]
supersedes:
superseded_by:
affects: [arch/Overall.md]
research: [R-001]
---

> **Refined by [D-030](D-030-email-provenance-purge.md):** the eval sample stores labels plus Gmail IDs, and its content is re-fetched from Gmail for each eval run.

# D-028: Keep the kNN correction-learning stage; build a ~200-email labelled eval sample

## Context
The user picked option A.

## Decision
- A small embedding model (on the core node by default, placed through the registry from D-026) embeds each message. A **pgvector kNN** over `CategoryLabel` (rule hits, System 2 labels, user corrections) runs as a stage *before* the System One stage at each decision point it supports.
- A **labelling view** in the web app (it shares the 'Fix category' action) builds a sample of about 200 real emails for `fa-eval`. Eval data stays local and is excluded from git (D-004).

## Consequences
- The labelling view is needed in M3 so the sample exists before M4 picks a model.

## Alternatives considered
Dropping kNN. Not chosen.
