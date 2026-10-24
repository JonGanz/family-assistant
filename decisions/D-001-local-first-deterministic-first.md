---
id: D-001
title: Local-first, deterministic-first processing
status: accepted
date: 2026-10-23
source_questions: []
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-001: Local-first, deterministic-first processing

## Context
PROJECT.md "Tech Goals" requires everything to run locally on 32 GB RAM and an RTX 3070 with 8 GB, and asks for as much deterministic processing as possible.

## Decision
- Any AI work uses a local LLM runtime (Ollama class). No cloud LLM APIs.
- Each pipeline runs its rule and parser stages first. The LLM is called only for items those stages leave unresolved, and its output must be JSON that matches a schema.
- Model budget is about a 7–8B model at Q4 or smaller, plus a small embedding model.

## Consequences
- We need a rule configuration (YAML) that a person can maintain.
- LLM work goes into a queue and runs in batches. See Q-015.

## Alternatives considered
- Cloud LLM: ruled out by the requirements.
