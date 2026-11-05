---
id: D-094
title: M4's AI track is the LLM extraction slice; System One and the agent move to M4.1
status: accepted
date: 2026-11-04
source_questions: [Q-084]
supersedes: D-067
superseded_by:
affects: [arch/AI.md, arch/Overall.md, arch/Ingest.md, README.md]
---

# D-094: M4's AI track is the LLM extraction slice; System One and the agent move to M4.1

## Context
D-067 set M4 as one System One backend on the core CPU plus one Ollama model, aimed at the decision points with the most inbox rejections. With D-093 the LLM is reached over a standard endpoint. The user wants AI support that improves email processing and clears the way for M5 (meal planning and recipe parsing), with the AI UI built at the end together with the UI revamp (D-097). Q-084.

## Decision
M4's AI track is the **LLM extraction slice**:
- `fa-llm`: the endpoint client from D-093, with a health check, a model list (when the server offers one), a bounded request queue, and per-call timeouts. Schemas are Rust types that derive a JSON Schema.
- An **`llm` stage in `fa-pipeline`** (D-029 stage kind) for email: extraction of events, bills and tasks, plus a fallback answer for decision points the rules left below their threshold. Its results land in the approval inbox as today (D-057, D-060); nothing new is applied without approval.
- A **recipe-extraction contract** ready for M5: a `Recipe` extraction schema, schema.org `Recipe` JSON-LD parsed deterministically first, the LLM only for pages without it.
- **Basic eval:** replay the labelled sample (D-028) through the pipeline with and without the `llm` stage and report accuracy and latency per decision point (CLI first).
- **Settings UI** for the endpoint and the pipeline's `llm` stage (enabled, threshold), built in the UI track's final wave.
The System One sidecar, kNN, the backend registry, `fa-gpu-agent`, Wake-on-LAN and the full eval move to M4.1.

## Consequences
- M4 needs no new container: the LLM server is the hoster's.
- The first consumer of member interests (D-071) is not in M4.
- The decision points with the most rejections (D-067) are measured by the basic eval once the shakedown data exists; the slice does not wait for it.

## Alternatives considered
- Add the System One CPU sidecar now: more moving parts before the LLM path is proven.
- Add a first member-interest feature: deferred so the slice stays small.
