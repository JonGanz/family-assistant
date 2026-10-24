---
id: Q-027
title: Keep the correction-learning kNN layer?
status: open
batch: 4
area: tech
created: 2026-10-23
resolved:
decisions: []
research: [R-001]
---

# Q-027: Keep the correction-learning kNN layer?

## Question
System One models don't learn from your corrections unless they are fine-tuned, and fine-tuning is barely documented (R-001). D-022 had an embedding kNN over labelled email (rule hits, corrections, and System 2 labels), which adapts to your family's mail. Keep it as a stage before the System One model? It needs a small embedding model on the core node: about 0.3 GB, run through Ollama or a Rust embedding crate. Also: will you label a sample of about 200 real emails for the eval harness? Labelling the sample is what lets us choose a model with evidence rather than benchmarks.

## Options
- A: keep kNN before System One; yes to a ~200 email labelled sample (recommended)
- B: drop kNN; yes to the sample
- C: other

## Answer
<!-- answer below this line -->
option A, and we should consider being able to fully configure the pipeline of decision making via a settings UI.
