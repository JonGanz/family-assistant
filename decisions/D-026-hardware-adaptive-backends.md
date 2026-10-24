---
id: D-026
title: Hardware-adaptive model placement via a backend registry
status: accepted
date: 2026-10-23
source_questions: [Q-025]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/AI.md (future)]
research: [R-001]
---

# D-026: Hardware-adaptive model placement via a backend registry

## Context
The user wants placement to be flexible and based on the hardware each user has. The reference hardware is defined, but households can upgrade or differ.

## Decision
- No model is tied to a particular node. A **backend registry** lists every model endpoint:
  - `Backend{node, kind: system_one|llm|embedding, endpoint, model, dialect, capabilities, availability: always|on_demand}`.
- **Nodes report their hardware:**
  - The core node runs `fa-sysprobe`.
  - Each GPU node's `fa-gpu-agent` reports VRAM, RAM, and CPU (this extends D-020).
  - A **profile suggester** recommends backends for each node from a small catalogue of known-good model sizes, for example: CPU-only means GLiNER2.5-Decide; 8 GB VRAM means Strands 2B or Kev 4B plus a 7–8B LLM; 16 GB or more means larger.
- At runtime, each pipeline stage names a **backend preference list**. The router uses the first one that is *available now*, and otherwise falls through. The reference hardware ends up with a small System One model on the core CPU, plus a larger System One model and the LLM on the GPU node.
- Any number of GPU nodes is supported (0, 1, or more).

## Consequences
- This generalises D-020, which assumed exactly one GPU node, and D-024. Neither is superseded: their behaviour is the reference-hardware case.
- The model catalogue is data and ships with the release. Users can add their own entries.

## Alternatives considered
Fixed placement (option A from Q-025). Replaced by the registry.
