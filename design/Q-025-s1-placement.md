---
id: Q-025
title: Where System One models run
status: open
batch: 4
area: infra
created: 2026-10-23
resolved:
decisions: []
research: [R-001]
---

# Q-025: Where System One models run

## Question
Options:
- **Core node (CPU, always on):** only the smallest model fits, GLiNER2.5-Decide (~0.4B, ~167 ms CPU per vendor). Its independent score is low.
- **GPU node (on demand):** Strands-Decider 2B or Kev 4B. Likely more accurate, but only available when the PC is awake. That loses the 'instant' benefit for urgent mail.
- **Both:** small on the CPU for instant triage, and a larger one on the GPU as an intermediate tier before the LLM.

The eval harness (D-024) will measure this on your email. Which shape should we build for?

## Options
- A: both, small on CPU always plus larger on GPU, pick final models after eval (recommended)
- B: core node CPU only
- C: GPU node only

## Answer
<!-- answer below this line -->

