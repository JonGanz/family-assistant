---
id: D-024
title: System 1 tier uses System One decision models behind a pluggable DecisionBackend
status: accepted
date: 2026-10-23
source_questions: [Q-023]
supersedes: D-022
superseded_by:
affects: [arch/Overall.md, arch/AI.md (future), arch/Ingest.md (future)]
research: [R-001]
---

# D-024: System 1 tier = System One decision models (pluggable)

## Context
The user clarified that "System 1" in Q-023 meant the **System One models** listed at https://systemonemodels.org/. D-022 had read it as a small generative LLM plus kNN. Research R-001 found the following:
- These are decision models (`Choice`, `Score`, `Noul`) that take labels per request and return probabilities.
- Several are Apache-2.0 and self-hostable, but only through **Python** runtimes. None run on Ollama, GGUF, or ONNX.
- None can extract data locally.
- The whole field is under 6 weeks old.

## Decision
**They fit the plan.** The tier structure from D-022 stays, but System 1 changes:
1. **Rules**: unchanged.
2. **System 1 = System One decision model**, reached through a `DecisionBackend` trait (crate `fa-decide`) that speaks the `/v1/systemone` dialect over HTTP. It answers:
   - email category (`Choice` over the household's configured categories);
   - time-sensitivity (`Noul`);
   - more uses depending on **[Q-026]**.

   The probabilities feed the existing confidence thresholds. Each request asks one question, in a fixed and stable option order (R-001 warns about order sensitivity).
3. **System 2 = generative LLM (Ollama, GPU node)**: still does all **extraction** (events, bills, tasks), plus whatever System 1 left at low confidence.

**These parts of D-022 still apply:**
- the escalation rule;
- corrections are collected through 'Fix category';
- the hardware probe and toggle (`ai.system1.enabled`, profile suggested by `fa-sysprobe`).

**Model choice is made by evaluation, not up front.** `fa-eval` replays a labelled sample of real household email against each candidate backend and reports accuracy, calibration, and latency. Candidates are GLiNER2.5-Decide, Strands-Decider 2B, Kev 0.8B/4B, and a quantized pplx-decider-flash. The first eval is part of M4 (D-023).

**Pending Batch 4:** the runtime/language exception (Q-024), where it runs (Q-025), extra uses (Q-026), and whether the kNN layer stays (Q-027).

## Consequences
- The pluggable trait means a better model can be swapped in by config as the field changes.
- One new container, `s1-decider`, if Q-024 allows a Python sidecar.
- Without the toggle on, behaviour is the same as before: rules, then the S2 queue.

## Alternatives considered
- D-022 as written (small generative LLM on CPU). Superseded: it was based on a misreading. A decision model is also faster and has no JSON-format failures.
- Hosted Jev or Fastino APIs. Rejected because of the local-only rule (D-001).
