---
id: R-004
title: Talking to a local LLM over the OpenAI-compatible Chat Completions API (Rust)
date: 2026-11-04
method: Sonnet 5.5 sub-agent; Ollama source (shallow clone of github.com/ollama/ollama, `openai/openai.go`, `envconfig/config.go`) and docs read directly; llama.cpp server README, vLLM and LM Studio docs fetched; `cargo add --dry-run`, `cargo search` and `cargo tree` run in a scratch dir outside the workspace; ollama.com library pages fetched
verified: see the UNVERIFIED list at the end. Nothing was run against a live model server
informs: [M4 plan]
---

# R-004: OpenAI-compatible LLM API

## 1. Ollama
| Question | Finding |
|---|---|
| `/v1/chat/completions` with `response_format` `json_schema` | Honoured. `openai/openai.go` maps `json_schema` to the native `format` field (`format = ResponseFormat.JsonSchema.Schema`), so decoding is constrained by the schema. Structured outputs via `/v1/chat/completions` landed with commit "api: structured outputs - chat endpoint (#7900)", 2024-12-04, first tag containing it: **v0.5.0** |
| `{"type":"json_object"}` | Honoured: mapped to native `format: "json"` (plain JSON mode, no schema) |
| `strict` | The `strict` flag is part of the OpenAI shape. The source only forwards `.Schema`; it was not found to change behaviour (the schema is always enforced) |
| Native `/api/chat` | `format: <JSON schema object>` (or `"json"`). Docs: "Provide a JSON schema to the `format` field"; they advise also putting the schema in the prompt. The result is a JSON string in `message.content` |
| `GET /v1/models` vs `GET /api/tags` | Both exist. `/v1/models` is OpenAI-shaped (`created` = last modified, `owned_by` defaults to `"library"`); `/api/tags` is the native list with sizes, family and quantisation |
| `keep_alive` | Not in the OpenAI-compat docs, but the request struct in `openai.go` carries `KeepAlive` with the comment "Ollama extension: without it an OpenAI-API client cannot release a model". So it is accepted as an extra top-level field in the compat body; native `/api/chat` takes it too. Which release added it to the compat layer: UNVERIFIED |
| Port, binding | Default `127.0.0.1:11434`; `OLLAMA_HOST` sets address (source: `envconfig/config.go`). Use `0.0.0.0:11434` to reach it from another node |
| Auth | None built in for a local server. The compat docs: "The client requires an API key value, but Ollama ignores it". Put a reverse proxy or firewall in front when binding beyond localhost |
| Cloud | Structured outputs are not supported by Ollama cloud; not relevant to us (local only) |

Request (compat), response is the usual `choices[0].message.content` string that must be parsed:
```json
{"model":"qwen3:8b","messages":[{"role":"user","content":"..."}],
 "response_format":{"type":"json_schema","json_schema":{"name":"extract","strict":true,"schema":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]}}},
 "temperature":0,"keep_alive":"10m"}
```
Native equivalent: `POST /api/chat {"model":..,"messages":[..],"format":{<schema>},"stream":false,"options":{"temperature":0},"keep_alive":"10m"}`; the text is in `message.content`.

## 2. Other servers
| Server | `response_format` json_schema | Auth | Default port |
|---|---|---|---|
| llama.cpp `llama-server` | Yes; README: `response_format` "supports both plain JSON output (e.g. `{"type": "json_object"}`) and schema-constrained JSON" (grammar based) | `--api-key KEY` / `--api-key-file` / env `LLAMA_API_KEY`; default none | README says `--port` default **9931** (old versions used 8080; the README examples still use 8080) |
| vLLM | Yes; docs show `response_format={"type":"json_schema","json_schema":{...}}` on `chat.completions` | `--api-key` exists in vLLM but the structured-outputs page does not mention it: UNVERIFIED here | Docs examples use 8000 (the default is not stated on the page) |
| LM Studio | Yes; `json_schema` with `name`, optional `strict`, `schema`. Docs warn that models under about 7B may not support it; GGUF uses llama.cpp grammar sampling | Placeholder key in examples; a separate Authentication page was not read | 1234 |

All accept `Authorization: Bearer <key>`, so one optional "api key" setting covers them (UNVERIFIED for vLLM and LM Studio, verified for llama-server only via the flag description).

## 3. Rust client
| | (a) `reqwest` + `serde` own DTOs | (b) `async-openai` |
|---|---|---|
| Version | none to add: workspace has `reqwest = 0.13.5` (rustls, json, gzip, http2, query, form) | 0.43.0 (`cargo add --dry-run`, `cargo search`); default feature `rustls`; 59 optional features |
| Custom base URL | yes, trivially | yes: `OpenAIConfig::with_api_base` (also `OPENAI_BASE_URL`) |
| json_schema | we write the 4 structs | `ResponseFormatJsonSchema` type exists (`types/shared`, re-exported in `types/chat`) |
| Weight | zero new crates | `cargo tree` of a scratch crate with only it plus serde_json: 135 unique crates, and it pulls the same `reqwest v0.13.5` (so no version clash). Extra deps include tokio-stream, tokio-util, futures, rand, base64, thiserror, tracing |
| Risk | we own the wire format; Ollama extras (`keep_alive`, native `format`) are one field each | typed to OpenAI; Ollama extensions need the BYO-types path |

**Recommendation: (a).** We need one endpoint (`/v1/chat/completions`) plus `/v1/models`, a handful of fields and Ollama extras, and the workspace already has reqwest 0.13.5. Keep the DTOs in one small module and a trait so the client is mockable.

Prefer the **OpenAI-compat endpoint with `json_schema`** as the one code path (works on Ollama, llama.cpp, vLLM, LM Studio). Always parse and validate the returned string ourselves, since the schema is only guaranteed by the server's decoder.

## 4. Model for structured extraction (8 GB VRAM, Q4)
Verified tags on ollama.com/library:
| Tag | Size (library page) | Notes |
|---|---|---|
| `qwen3:8b` (= `latest`) | 5.2 GB | 40K context; recommended default. Weights 5.2 GB leave about 2 to 3 GB for KV cache on an 8 GB card, so budget a context of a few thousand tokens. Qwen3 has a thinking mode; disable it for extraction (the compat layer accepts `reasoning_effort`, see `openai.go`; the `/no_think` prompt switch is a Qwen3 feature, not verified here) |
| `llama3.1:8b` (= `latest`) | 4.9 GB | 128K context; quant not named on the page, ID matches q4_K_M: UNVERIFIED |
| `qwen3:4b` | 2.5 GB | fallback for smaller GPUs |

Make the model tag a setting (default `qwen3:8b`), as per the configurable-defaults rule.

## UNVERIFIED
- Behaviour against a live server (nothing was run); that `strict` is a no-op in Ollama.
- Release that added `keep_alive` to the compat layer.
- vLLM and LM Studio API key behaviour; the vLLM default port.
- Exact VRAM use at a given context (sizes above are download sizes, not VRAM).
- The Qwen3 `/no_think` mechanism.
