---
id: Q-083
title: How the core talks to the LLM on a GPU node
status: resolved
batch: 22
area: tech
created: 2026-11-04
resolved: 2026-11-04
decisions: [D-093]
---

# Q-083: How the core talks to the LLM on a GPU node

## Question
How does the core node talk to the generative LLM (System 2) on a GPU node? A custom Rust agent that embeds inference (llama.cpp/mistral.rs with CUDA, large builds), a thin agent over Ollama, or no agent: a standard HTTP LLM API configured by URL and model?

## Options
- A. Custom Rust agent with embedded inference
- B. Thin Rust agent over Ollama
- C. Configurable OpenAI-compatible endpoint (URL, model, optional key), default Ollama on the GPU node; no agent in M4

## Answer
<!-- answer below this line -->
C (2026-11-04, in session). The user asked whether hosting Ollama on the GPU PC and configuring URL and model on our side is enough, and whether it is standard. Yes: speak the OpenAI-compatible Chat Completions API with JSON-schema structured output, so any compatible server (Ollama, llama-server, vLLM, LM Studio) works by config. No custom agent in M4.

