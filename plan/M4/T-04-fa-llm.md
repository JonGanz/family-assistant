---
id: T-04
title: "fa-llm: the OpenAI-compatible client"
milestone: M4
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-llm/** (except src/provider.rs)"]
decisions: [D-093, D-094]
---

# T-04: `fa-llm`: the OpenAI-compatible client

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M4's track B adds an LLM (System 2) reached over a configurable OpenAI-compatible endpoint (D-093), by default Ollama on a GPU PC. This task builds the new crate `fa-llm`'s client. It has no database access; the `Provider`, which reads the endpoint from the store, is T-06's job in wave 2.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md` (disk, no heavy deps, no content in logs).
- `plan/M4.md`: *`fa-llm`*, the `LlmClient` part (the code block and the bullets under it).
- `arch/AI.md` §2 (endpoint), §3 (crates), §7 (failure modes) and §11 (security).
- `research/R-004-openai-compatible-llm-api.md`: request and response shapes for `/v1/chat/completions` with `response_format` `json_schema`, `/v1/models`, Ollama native `/api/chat` with `format` and `/api/tags`.
- The root `Cargo.toml`: the main session (T-01) has added `crates/fa-llm` as a member and `schemars = "1.2.3"` as a workspace dependency. If either is missing, stop and report.
- Style references: `crates/fa-connectors/src/http.rs` (reqwest usage), and `crates/fa-connectors/Cargo.toml` (`workspace = true` dependencies).

## You own
`crates/fa-llm/**` **except** `src/provider.rs`, which T-06 adds in wave 2 together with a `mod provider; pub use provider::Provider;` line in `lib.rs`. Leave a comment in `lib.rs` where that line goes.

## Work
1. `crates/fa-llm/Cargo.toml` with only `workspace = true` dependencies: `reqwest`, `serde`, `serde_json`, `schemars`, `thiserror`, `tracing`, `url`, `tokio`, and dev-deps `axum` and `tokio` (macros, rt). No `async-openai`.
2. The public API exactly as in the plan: `EndpointConfig`, `Dialect`, `LlmClient` (`new`, `extract<T>`, `models`, `ping`), `ExtractRequest`, `Extracted<T>`, `Ping`, `LlmError` (thiserror) and `LlmError::kind()`, which returns `unavailable|timeout|http|bad_json|schema|config`.
3. Building the HTTP client:
   - `LlmClient::new` takes a `reqwest::Client`.
   - Also provide `pub fn default_http() -> reqwest::Result<reqwest::Client>`, which builds one with `redirect::Policy::none()`, rustls and no global timeout. The per-request timeout comes from `cfg.timeout`.
   - Private and LAN hosts are allowed.
4. **`openai_chat`:** `POST {base_url}/chat/completions` (join the paths without losing `/v1`; test with and without a trailing slash).
   - Body: `{"model", "messages":[{"role":"system",...},{"role":"user",...}], "response_format":{"type":"json_schema","json_schema":{"name":schema_name,"schema":<schemars::schema_for!(T) as JSON>,"strict":true}}, "max_tokens", "temperature":0, "stream":false}`.
   - Send `Authorization: Bearer <key>` only when a key is set.
   - Parse `choices[0].message.content`, a string, into `T`. Invalid JSON is `BadJson`; valid JSON that does not deserialise into `T` is `Schema`.
   - Read `usage.prompt_tokens` and `usage.completion_tokens` when they are present.
5. **`ollama_native`:** `POST {base_url}/api/chat` with `{"model","messages","format":<schema>,"stream":false,"options":{"temperature":0,"num_predict":max_tokens}}`. Parse `message.content`; the token counts come from `prompt_eval_count` and `eval_count`.
6. **`models()`:** `GET {base_url}/models` returns `data[].id` (openai_chat), and `GET {base_url}/api/tags` returns `models[].name` (ollama_native). **`ping()`:** call `models()`, then set `model_found` to whether the configured model is in the list. Ollama lists `qwen3:8b`, and may list `qwen3:8b` vs `qwen3:latest`, so compare exactly and also accept a match after appending `:latest` to a configured name that has no tag.
7. **Error mapping:** connection refused, a DNS failure or a TLS connect failure is `Unavailable` (with a short reason, no URL credentials); the request timeout is `Timeout`; a non-2xx status is `Http(status)`; a bad config (an empty model, a base URL that isn't http or https) is `Config`.
8. **Logging:** `tracing` logs dialect, model, latency and error kind. **Never** log prompts, responses, the key or the full URL with userinfo.
9. **Schema helper:** `pub fn json_schema_for<T: JsonSchema>() -> serde_json::Value`. It strips `$schema`. If OpenAI's strict mode needs `additionalProperties: false` on objects, add it, and note in the report what R-004 says about strictness on Ollama.

## Tests (`crates/fa-llm/tests/`, an in-process axum stub on `127.0.0.1:0`)
- openai_chat: the stub asserts the request body (model, `response_format.type == "json_schema"`, temperature 0, the `Authorization` header present or absent) and returns content JSON → `extract::<Fixture>()` gives the value and the token counts.
- ollama_native: the same, with `format`.
- Content that isn't JSON gives `BadJson`; JSON with the wrong shape gives `Schema`; a 500 gives `Http(500)`; a slow stub with a 200 ms timeout gives `Timeout`; a closed port gives `Unavailable`.
- A 302 from the stub is **not** followed.
- `models()` and `ping()` for both dialects, including `model_found: false`.
- Base URL joining with and without a trailing slash.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-llm`, `cargo clippy -p fa-llm --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes, or the report says which other task's in-progress crate broke it.
- [ ] The report lists:
  - the public API as built;
  - the strict-mode handling;
  - any deviation from the plan's signatures;
  - "list what outside your paths needs changing" (for example, workspace deps);
  - README changes needed: none.

## Outcome (T-04 report)
- **Built:** `fa-llm` (`client.rs`, `schema.rs`, `tests/client.rs` with 6 axum-stub tests). It has both dialects, `models`, `ping` and `default_http()` (redirects off).
- **Schema:** `json_schema_for` strips `$schema` and sets `additionalProperties: false`.
- **Follow-up for T-06:** OpenAI strict mode also wants every property listed in `required`, so `Option` fields need handling if a server enforces it. Ollama does not enforce it.
