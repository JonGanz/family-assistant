---
id: D-093
title: System 2 is reached over a configurable OpenAI-compatible endpoint; no custom agent in M4
status: accepted
date: 2026-11-04
source_questions: [Q-083]
supersedes: 
superseded_by:
affects: [arch/AI.md, arch/Overall.md, arch/Platform.md]
---

# D-093: System 2 is reached over a configurable OpenAI-compatible endpoint; no custom agent in M4

## Context
D-020 and D-026 planned a custom Rust `fa-gpu-agent` beside Ollama, and `fa-llm` as an Ollama client. On 2026-11-04 the user asked whether hosting Ollama on the GPU PC and configuring the URL and model on the core side would be enough, and whether that is the standard way to reach a remote LLM (Q-083). A custom agent with embedded inference (llama.cpp or mistral.rs with CUDA) would also add a very large build on a disk that is already tight.

## Decision
- `fa-llm` speaks the **OpenAI-compatible Chat Completions API** (`POST {base_url}/chat/completions`) with **JSON-schema structured output** (`response_format` of type `json_schema`). Every System 2 call is "prompt plus JSON Schema in, validated struct out".
- An **LLM endpoint** is data, edited in Settings (adults): `base_url`, `model`, optional API key (sealed as a secret), timeout, enabled. Any compatible server works (Ollama, llama.cpp `llama-server`, vLLM, LM Studio). The default profile is Ollama on the GPU node.
- Ollama's native API is used only for optional extras when the endpoint is Ollama: listing installed models and unloading a model (`keep_alive: 0`). If R-004 shows that a server ignores `json_schema`, a per-endpoint dialect falls back to Ollama's native `format` behind the same trait.
- **No custom GPU agent in M4.** When the endpoint is unreachable the pipeline falls back to rules and the human stage (D-002, D-029).
- Ollama has no authentication: the hoster binds it to the LAN or tailnet address only (README and deploy notes say so).

## Consequences
- No large CUDA build in the workspace; `fa-llm` is a small HTTP client.
- `fa-gpu-agent` (D-020) stays in M4.1 and becomes an optional add-on next to the LLM server (hardware report, gaming yield, wake and suspend), not an inference runtime. D-026's `Backend.dialect` gains `openai_chat` as the default value.
- Swapping or upgrading the LLM server is a config change.

## Alternatives considered
- A. A custom Rust agent with embedded inference: large builds, CUDA toolchain on the build host, and re-implementing what Ollama already does.
- B. A thin Rust agent that proxies Ollama: adds a hop and a binary without adding anything M4 needs.
- Ollama's native `/api/chat` as the main dialect: ties the household to one server.
