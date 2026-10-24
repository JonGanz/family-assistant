---
id: Q-024
title: Python runtime exception for System One models
status: open
batch: 4
area: tech
created: 2026-10-23
resolved:
decisions: []
research: [R-001]
---

# Q-024: Python runtime exception for System One models

## Question
Every self-hostable System One model found so far runs only on a **Python** runtime (`gliner2`, `strands-decider serve`, Kev's server). None support Ollama, GGUF, ONNX, or Rust (R-001). D-003 ruled out Python for *our* code. Is a **third-party Python model server in its own container** acceptable? Our Rust code would only call it over HTTP.

## Options
- A: yes, a containerized Python sidecar is fine; our code stays Rust (recommended)
- B: no Python at all. Only adopt System One models once a Rust/ONNX-runnable build exists (S1 stays off until then)
- C: other

## Answer
<!-- answer below this line -->

