---
id: D-016
title: LLM work is batched; optional small CPU triage model on the core node
status: superseded
date: 2026-10-23
source_questions: [Q-015, Q-020]
supersedes:
superseded_by: D-022
affects: [arch/Overall.md]
---

> **Superseded by [D-022](D-022-system1-system2-classification.md).** Batching still applies, but the CPU tier is now System 1.

# D-016: LLM work is batched; optional small CPU triage model on the core node

## Context
Batched processing is fine (Q-015 A). The user wants a small CPU model for urgent triage, with a switch to turn it on or off (Q-020 A).

## Decision
- All heavy LLM work goes through the queue and the GPU node (D-002, D-020).
- **Optional CPU triage** (`llm.cpu_triage.enabled`, default off until we know the specs, Q-022): Ollama runs on the core node with a model of about 1.5B parameters (for example `qwen2.5:1.5b-instruct-q4`). It answers only these questions, as JSON:
  - Is the message time-sensitive?
  - What is the coarse category?
- It runs only on messages the rules couldn't decide. A 'time-sensitive' result raises a notification straight away. The full enrichment still waits for the GPU node.
- The `fa-llm` gateway sends each job to a backend by its type: `triage` goes to the CPU when enabled, and everything else goes to the GPU queue.

## Consequences
- The core node needs about 2–3 GB of spare RAM. The model size depends on Q-022.

## Alternatives considered
Rules only until the GPU node comes up. Kept as the behaviour when the switch is off.
