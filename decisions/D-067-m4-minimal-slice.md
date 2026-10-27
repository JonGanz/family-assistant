---
id: D-067
title: M4 ships a minimal AI slice first; the registry and platform pieces move to M4.1
status: accepted
date: 2026-10-26
source_questions: [Q-062]
supersedes: 
superseded_by:
affects: [arch/Overall.md, README.md]
---

# D-067: M4 ships a minimal AI slice first; the registry and platform pieces move to M4.1

## Context
Overall.md planned M4 as a backend registry, profile suggester, sysprobe, GPU agent with gaming detection, eval suite, kNN and two System One sizes. The review judged it platform-scale for one household.

## Decision
Option A. **M4:** one System One backend (CPU, core node) and one Ollama model on the GPU node, behind the existing `fa-pipeline` stages, aimed at the decision points with the most inbox rejections measured in the M3.5 shakedown (D-066). **M4.1:** backend registry and profile suggester (`fa-backends`), `fa-sysprobe`, `fa-gpu-agent` wake/yield and gaming detection, the full eval suite, once a second hardware profile actually exists.

## Consequences
- D-026 and D-029 (hardware-adaptive, configurable pipelines) still hold; this is sequencing only.
- Until M4.1 the GPU PC must be awake for System 2 work; the pipeline falls back to rules when it is not.
- Interests and hobbies (D-071) are LLM inputs, so their first consumer lands no earlier than M4.

## Alternatives considered
- B, the full design first.
- C, the slice plus the GPU agent: deferred until gaming conflicts are observed.
