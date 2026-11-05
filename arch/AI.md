---
title: AI — tiers, LLM endpoint, llm stage, recipe extraction, eval
scope: The intelligence tiers (rules, System One, LLM), how the core reaches an LLM server (D-093), fa-llm and its Provider, the llm pipeline stage and the mail.llm job, the recipe-extraction contract for M5, basic eval, LLM call observability, security and failure modes, and the M4.1 roadmap
status: draft
updated: 2026-11-04
drawio: AI.drawio
drawio_src: src/AI.py
depends_on: [Overall.md, Ingest.md, Platform.md]
milestone: M4
decisions: [D-002, D-020, D-024, D-025, D-026, D-027, D-028, D-029, D-057, D-060, D-071, D-093, D-094]
research: [R-001, R-004]
---

# AI: tiers, LLM endpoint, llm stage, recipes, eval (DRAFT v1, M4)

This document is derived from PROJECT.md and every accepted AI decision: D-024 to D-029, D-071, D-093 and D-094. D-067 is superseded by D-094. The plan with the exact contracts is [plan/M4.md](../plan/M4.md); when the two disagree, the plan's contract wins until this doc is updated.

**M4 builds only the LLM extraction slice (D-094).** System One, kNN, the backend registry, `fa-gpu-agent` and Wake-on-LAN come in M4.1 (§10).

## 1. Tiers (D-002, D-024, D-029)
| Tier | What | Where | Milestone |
|---|---|---|---|
| Rules | Headers, org and contact maps, keywords, biller templates, ICS and schema.org extractors (`fa-rules`) | core, in-process | M3 |
| kNN | pgvector over labelled decisions (D-028) | core | M4.1 |
| System 1 | System One decision models (`Choice`, `Score`, `Noul`) behind `fa-decide`, as Python sidecars (D-024, D-025) | core CPU or GPU node | M4.1 |
| System 2 | A generative LLM returning **schema-constrained JSON** (D-093) | any OpenAI-compatible server; default Ollama on the GPU node | **M4** |
| Human | The approval inbox and the labelling view | web app | M3 |

The app works with no AI at all. Every pipeline ends in `rules` or `human`, and an unreachable LLM only delays items (§7).

## 2. The LLM endpoint (D-093)
- **Protocol:** the OpenAI-compatible Chat Completions API (`POST {base_url}/chat/completions`), with structured output through `response_format: {type: "json_schema", json_schema: {name, schema, strict: true}}`.
  - Ollama has honoured this since v0.5.0 (R-004). So do llama.cpp `llama-server`, vLLM and LM Studio.
  - Every System 2 call is "prompt plus JSON Schema in, validated Rust struct out".
- **Fallback dialect `ollama_native`:** `POST {base_url}/api/chat` with `format: <schema>` and `stream: false`. This is only for a server that ignores `json_schema`. `base_url` then has no `/v1`.
- **Configuration is data** (table `llm_endpoint`, one row per household in M4), edited in Settings → AI by adults:
  - `base_url`: `http` or `https`, no query string;
  - `model`: a free text field, also offered from the server's model list;
  - `dialect`: `openai_chat` (the default) or `ollama_native`;
  - `timeout_ms`: default 120000;
  - `enabled`: default false;
  - an optional API key, stored as a `secret` row (`owner_type = 'llm_endpoint'`, `kind = 'api_key'`, AAD-bound per C-10) and never returned, only `has_api_key`.
- **Default profile:** Ollama on the GPU PC at `http://<gpu-pc-lan-or-tailnet-ip>:11434/v1`, model `qwen3:8b` (R-004; about 5.2 GB to download, fits 8 GB of VRAM at Q4). Swapping the server or model is a settings change.
- **Model list:** `GET {base_url}/models` (OpenAI shape) or `/api/tags` (`ollama_native`) fills the model picker. `ping` = list the models, then check that the configured one is present.

## 3. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-llm` (new) | `LlmClient`: one endpoint and both dialects, with `extract<T: JsonSchema>`, `models` and `ping`. Its own `reqwest::Client` has **redirects disabled**, and private and LAN hosts are allowed (an adult sets the URL). `Provider`: per-household client lookup with a 30 s cache, a per-household `Semaphore(1)` (one call in flight; an 8 GB GPU serves one request well), and an `llm_call` row after every call | fa-domain, fa-store, `reqwest`, `schemars` |
| `fa-pipeline` (extended) | `Outcome.pending_llm`, `Pipeline::record_stage` (accept an LLM result at or above the `llm` stage's threshold), `Pipeline::set_llm_stage` (a new version with the stage inserted before `human`) | (unchanged) |
| `fa-mail` (extended) | Enqueues `mail.llm`; the `mail.llm` job (one call per message, schema `MailLlmResult`); `llm.prune` | + fa-llm |
| `fa-recipe` (new) | `from_html` (schema.org `Recipe` JSON-LD, no I/O), `from_llm` (fallback), `fetch_page` (guarded fetch through `fa_connectors::http::Http` with private hosts blocked), `RecipeDraft` | fa-llm, fa-connectors, `scraper` |
| `fa-eval` (new, basic) | Replays the labelled sample (`message.in_eval_sample`) through rules, and through rules plus `llm`, and reports accuracy, coverage and latency per stage | fa-pipeline, fa-mail, fa-llm, fa-store |
| `fa-api` (extended) | `ai.rs`: the `/ai/*` routes (§8) | + fa-llm, fa-recipe |
| `fa-server` (extended) | Builds one `fa_llm::Provider` and hands it to the API state and `fa_mail::MailDeps`; registers `mail.llm` and `llm.prune`; subcommand `eval` | |

The coupling rule holds: `fa-recipe` stores nothing (M5 adds storage in `fa-meals`). `fa-llm` knows nothing about mail.

## 4. Data
| Table | Columns | Written by |
|---|---|---|
| `llm_endpoint` | id, household_id (UNIQUE in M4), base_url, model, dialect, timeout_ms, enabled, created_at, updated_at | `PUT /ai/llm-endpoint` |
| `ai_settings` | household_id (PK), `mail_extract_mode` `off\|fallback\|always` (default `fallback`), `max_input_chars` (default 12000), updated_at | `PUT /ai/settings`; read with defaults when absent |
| `llm_call` | id, household_id, purpose (`mail.llm`, `recipe.extract`, `endpoint.test`, `eval`), model, started_at, latency_ms, ok, error_kind?, prompt_tokens?, completion_tokens? | `Provider::call`; pruned after 90 days by `llm.prune` |
| `extracted_by` on `event`, `bill`, `task` | `rules\|llm`, default `rules` | the `mail.llm` job writes `llm` |

`event.origin` (`synced|extracted|manual`) and `task.origin` (`manual|system|weather|email`) already exist with other meanings, so the provenance of an extraction is a separate column, `extracted_by`. The inbox views carry it as `extracted_by: ExtractOrigin`.

**No prompt or response text is stored or logged** (§9). Decisions keep their value and confidence as before (`decision.stage_kind = 'llm'`).

## 5. The `llm` stage (D-029, page "Sequence")
- The existing stage kind `llm` uses the existing columns: `enabled`, `accept_threshold` (default 0.7), `on_unavailable = queue`, `config = {}`.
- In M4 the stage is offered only for the four email decision points: `email.category`, `email.time_sensitive`, `email.member_link` and `email.is_bill`. `event.calendar_route` keeps going to the inbox (D-057).
- **Inline run:** `Pipeline::run` stays synchronous and never calls the LLM. When `rules` falls below its threshold and the next enabled stage is `llm`, the `Outcome` carries `pending_llm = true`, the default value applies meanwhile (as for `human`, Ingest.md §5), and `mail.process` enqueues the job.
- **Deferred result:** `Pipeline::record_stage(.., StageKind::Llm, candidates, version)` accepts the best candidate at or above the `llm` threshold. Below the threshold, the decision falls through to `human`: the default stays and the message is eligible for the labelling queue.
- **Enabling:** Settings → AI toggles the stage per decision point. `Pipeline::set_llm_stage` saves a **new pipeline version** (rollback by reactivating the old one), so existing validation applies: the last enabled stage is still `rules` or `human`.

## 6. The `mail.llm` job
**Trigger.** `mail.process` enqueues `mail.llm {message_id}`, with idempotency key `mail.llm:<message_id>` so a message is queued at most once, when the endpoint is enabled and either of these holds:
- a decision point has `pending_llm`;
- the category's behaviour is `extract` and either `mail_extract_mode = always`, or the mode is `fallback` and no rule extractor produced an item at 0.8 or above.

**One call per message** (`purpose = "mail.llm"`), schema `MailLlmResult`. Only the parts that are asked for are in the schema and the prompt.
- **Decision parts:**
  - `category {value, confidence}`: value is one of the household's category keys, sorted;
  - `time_sensitive {value, confidence}` and `is_bill {value, confidence}`;
  - `member_link [{member, confidence}]`: member keys ordered by id.
- **Extraction parts:**
  - `events [{title, start, end?, location?, confidence}]`: `start` and `end` are RFC 3339, or `YYYY-MM-DD` for all-day;
  - `bills [{payee, amount_cents, currency, due, confidence}]`: `currency` is ISO 4217 and `due` is `YYYY-MM-DD`;
  - `tasks [{title, due?, kind: reply|sign|rsvp|pay|other, confidence}]`: `due` is `YYYY-MM-DD`.
- Options are listed in a fixed, stable order.
- **Input:** subject, sender, received date (household time zone) and the plain-text body, truncated to `ai_settings.max_input_chars`.
- **Member context:** names and aliases only in M4. Interests, hobbies and responsibilities (D-071) are not used yet.

**Applying the result:**
- decisions are recorded through `record_stage`;
- if `email.category` changed, the job re-runs the message's category behaviour step, which is idempotent;
- extracted items go through the **existing** dedupe and proposal paths with `extracted_by = llm`, and still wait for approval (D-057, D-060);
- the inbox shows an "AI" chip with the confidence.

**Retries.** The job is enqueued with `max_attempts = 12`. The worker's backoff is `30 s × 2^attempts`, so the queue keeps retrying for about a day while the GPU PC is off. `Unavailable` and `Timeout` fail the job for a retry. `BadJson` and `Schema` fail it the same way, because the model may answer differently next time. After the last attempt the job is `dead` and the message keeps its rule or default values.

## 7. Failure modes
| Situation | Behaviour |
|---|---|
| No endpoint or `enabled = false` | M3 behaviour; `mail.llm` is never enqueued; `/ai/recipes/extract` returns JSON-LD results or 503 `llm_unavailable` |
| GPU PC asleep or Ollama down | `LlmError::Unavailable` → retry with backoff (§6); defaults apply meanwhile |
| Model missing on the server | `ping` reports `model_found: false` in Settings; calls fail as `Http(404)` |
| Slow model | `timeout_ms` per call; one call in flight per household |
| Malformed or schema-violating JSON | `BadJson` / `Schema`; nothing is applied; retried |
| Low confidence | below the stage threshold → human stage, as before |

## 8. API (adult-only; plan/M4.md fixes the shapes)
| Endpoint | Purpose |
|---|---|
| `GET/PUT /ai/llm-endpoint` | Read or replace the endpoint. `api_key`: absent keeps it, `null` clears it, a string sets it |
| `POST /ai/llm-endpoint/test` | Ping stored or unsaved values: `{ok, latency_ms, model_found, error}` |
| `GET /ai/llm-endpoint/models` | `{models: string[]}` from the server |
| `GET/PUT /ai/settings` | `mail_extract_mode`, `max_input_chars` |
| `GET /ai/pipeline`, `PUT /ai/pipeline/{decision_point}` | The per-point `llm` toggle and threshold |
| `GET /ai/stats?days=7` | Calls per purpose (total, ok, p50 and p95 in ms) and decisions per stage |
| `POST /ai/recipes/extract` | `{url}` or `{html}` → `RecipeDraft` (nothing stored) |

Settings → AI (the web app, Frontend.md) shows the endpoint card, the test result, the model picker, the extract mode, the per-point toggles and the stats card.

## 9. Recipe extraction contract (for M5)
- **Deterministic first:** `from_html` reads schema.org `Recipe` JSON-LD, including `@graph`, arrays and `HowToSection` steps.
  - ISO 8601 durations become integer minutes (`PT1H30M` → 90; unparseable → `null`).
  - Ingredient `text` is always the source line. `quantity`, `unit` and `item` come from a small rule parser that handles fractions, unicode fractions and ranges (taking the low end) with a fixed unit list. When it is unsure they are `null`.
- **LLM fallback:** `from_llm` gets the page's visible text (truncated to `max_input_chars`) and the `RecipeDraft` schema.
- **`RecipeDraft`:** `{name, source_url, origin: jsonld|llm, yield_text, servings, prep_minutes, cook_minutes, total_minutes, ingredients[{text, quantity, unit, item}], steps[], tags[], image_url}`.
- **Fetching:** `fa_connectors::http::Http` with private hosts blocked (on redirects too), its body cap and its timeout. M5 moves `RecipeDraft` into `fa-domain` and adds storage (Domain.md `Recipe.import_method: jsonld|microdata|llm|manual`).

## 10. Eval (basic, D-028)
- `fa-server eval --household <id> --decision-point <dp> [--with-llm] [--limit N] [--json]` runs over `message.in_eval_sample` messages.
- The **truth** is the human decision for that point; messages without one are skipped.
- It reports, per stage (`rules`, then `llm`, then the remainder to `human`): items decided, correct, p50 and p95 latency. Overall it reports accuracy and coverage (the share decided before `human`).
- It writes no decisions. LLM calls are logged with `purpose = "eval"`.
- It is run by a person on the real install (a human step); tests use fixtures.
- M4.1 adds calibration (ECE), dry-run in a Pipeline screen and backend comparison.

## 11. Security and privacy
- **Ollama has no authentication.** The hoster binds it to the GPU PC's LAN or tailnet address with `OLLAMA_HOST=<ip>:11434`, never a public one, and the README says so. An API key field exists for servers or proxies that use one.
- **Mail content goes only to the configured endpoint.** It is never logged or stored in `llm_call`. Log lines carry ids, purpose, latency and error kind only (D-035, the no-real-data rule).
- **Redirects are disabled** on the LLM client, so a compromised or misconfigured server cannot bounce requests elsewhere. Recipe fetches use the guarded client: no private targets, checked again on each redirect.
- Only adults can read or change the endpoint, test it or run recipe extraction (RBAC: Settings scope).

## 12. Deployment (page "Deployment")
- **Core node:** unchanged in M4 (no new container).
- **GPU node:** Ollama installed by the hoster (the native installer or the `ollama/ollama` image), listening on the LAN or tailnet address, with the model pulled (`ollama pull qwen3:8b`).
- **Single machine:** pointing `base_url` at `http://127.0.0.1:11434/v1` from the host, or at a compose service, works the same way.
- The compose `gpu` profile's `ollama` service (Platform.md §8) is one way to run it; `s1-large` there is M4.1.

## 13. M4.1 roadmap (D-094, D-020, D-026)
- **System One sidecar** on the core CPU (`s1-small`, Python, D-025) and `fa-decide`, for `email.category` and `email.time_sensitive` first.
- **kNN** over pgvector with an `embed` sidecar (D-028).
- **Backend registry** (`fa-backends`): `llm_endpoint` grows into `Backend{kind, dialect: openai_chat|ollama_native|systemone_v1, availability}`. The unique constraint per household is dropped and preference lists are added (D-026).
- **`fa-gpu-agent`:** an *optional* add-on next to the LLM server, not an inference runtime. It reports hardware, yields to gaming (unloading through `keep_alive: 0`) and suspends. It also adds Wake-on-LAN, plus waking on queue depth (D-020).
- **Full eval** with calibration and Pipeline-screen dry-runs, and **S2 weather advisories** (Feeds.md).
- **First member-interest feature** (D-071).
