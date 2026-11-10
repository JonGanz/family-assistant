---
title: AI — tiers, LLM endpoint, llm stage, recipe extraction, eval
scope: The intelligence tiers (rules, System One, LLM), how the core reaches an LLM server (D-093), fa-llm and its Provider, the llm pipeline stage and the mail.llm job, the recipe-extraction contract for M5, basic eval, LLM call observability, security and failure modes, and the M4.1 roadmap
status: draft
updated: 2026-11-09
drawio: AI.drawio
drawio_src: src/AI.py
depends_on: [Overall.md, Ingest.md, Platform.md]
milestone: M4
decisions: [D-002, D-020, D-024, D-025, D-026, D-027, D-028, D-029, D-057, D-060, D-071, D-093, D-094, D-104, D-107, D-109, D-113]
research: [R-001, R-004]
---

# AI: tiers, LLM endpoint, llm stage, recipes, eval (DRAFT v1, M4)

This document is derived from PROJECT.md and every accepted AI decision: D-024 to D-029, D-071, D-093, D-094 and D-104 (confirmed M4 choices). D-067 is superseded by D-094. The plan with the exact contracts is [plan/M4.md](../plan/M4.md); when the two disagree, the plan's contract wins until this doc is updated.

**LLM first (D-107, built in M4 wave 6):** with the endpoint reachable, every message goes to the LLM and the rules are the provisional value and the fallback (§5a). `mail_mode = rules_first` keeps the original M4 behaviour (the LLM only when the rules are unsure).

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
  - `thinking`: default false. Off asks a reasoning model to answer straight away (`reasoning_effort: "none"` for `openai_chat`, `think: false` for `ollama_native`); on sends neither and adds 8192 tokens to every request's output budget, because the reasoning counts against it. Measured on Ollama 0.40.1 with `qwen3.5:9b` (thinking by default): with thinking on, a two-line mail spent the whole 1024-token budget reasoning and returned no answer (`bad_json`); off, it answered in 25 tokens;
  - `enabled`: default false in the table, but Settings → AI saves a newly created endpoint as enabled (D-104, Q-096);
  - an optional API key, stored as a `secret` row (`owner_type = 'llm_endpoint'`, `kind = 'api_key'`, AAD-bound per C-10) and never returned, only `has_api_key`.
- **Default profile:** Ollama on the GPU PC at `http://<gpu-pc-lan-or-tailnet-ip>:11434/v1`, model `qwen3:8b` (R-004; about 5.2 GB to download, fits 8 GB of VRAM at Q4). Swapping the server or model is a settings change.
- **Model list:** `GET {base_url}/models` (OpenAI shape) or `/api/tags` (`ollama_native`) fills the model picker. `ping` = list the models, then check that the configured one is present.

## 3. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-llm` (new) | `LlmClient`: one endpoint and both dialects, with `extract<T: JsonSchema>`, `models` and `ping`. Its own `reqwest::Client` has **redirects disabled**, and private and LAN hosts are allowed (an adult sets the URL). `Provider`: per-household client lookup with a 30 s cache, a per-household `Semaphore(1)` (one call in flight; an 8 GB GPU serves one request well), and an `llm_call` row after every call | fa-domain, fa-store, `reqwest`, `schemars` |
| `fa-pipeline` (extended) | `Outcome.pending_llm`, `Pipeline::record_stage` (accept an LLM result at or above the `llm` stage's threshold), `record_stage_or` (the same, with a fallback threshold for a point that has no `llm` stage; `llm_first`), `Pipeline::set_llm_stage` and `set_stages` (a new version with the stage inserted before `human`; `set_stages` can also change the `rules` stage's threshold in the same version) | (unchanged) |
| `fa-mail` (extended) | Enqueues `mail.llm`; the `mail.llm` job (one call per message, schema `MailLlmResult`); `mail.llm.catchup` (§6a); `llm.prune` | + fa-llm |
| `fa-recipe` (new) | `from_html` (schema.org `Recipe` JSON-LD, no I/O), `from_llm` (fallback), `fetch_page` (guarded fetch through `fa_connectors::http::Http` with private hosts blocked), `RecipeDraft` | fa-llm, fa-connectors, `scraper` |
| `fa-eval` (new, basic) | Replays the labelled sample (`message.in_eval_sample`) through rules, and through rules plus `llm`, and reports accuracy, coverage and latency per stage | fa-pipeline, fa-mail, fa-llm, fa-store |
| `fa-api` (extended) | `ai.rs`: the `/ai/*` routes (§8) | + fa-llm, fa-recipe |
| `fa-server` (extended) | Builds one `fa_llm::Provider` and hands it to the API state and `fa_mail::MailDeps`; registers `mail.llm`, `mail.llm.catchup` and `llm.prune` (through `fa_mail::handlers`); subcommand `eval` | |

The coupling rule holds: `fa-recipe` stores nothing (M5 adds storage in `fa-meals`). `fa-llm` knows nothing about mail.

## 4. Data
| Table | Columns | Written by |
|---|---|---|
| `llm_endpoint` | id, household_id (UNIQUE in M4), base_url, model, dialect, timeout_ms, thinking, enabled, created_at, updated_at | `PUT /ai/llm-endpoint` |
| `ai_settings` | household_id (PK), `mail_extract_mode` `off\|fallback\|always` (default `fallback`), `max_input_chars` (default 12000), `mail_mode` `llm_first\|rules_first` (default `llm_first`), `catchup_days` (0 to 365, default 30; 0 switches the catch-up off), `extract_fallback_cutoff` (0.5 to 1, default 0.8), updated_at | `PUT /ai/settings`; read with defaults when absent |
| `message.llm_state` | `none\|queued\|done\|awaiting`, default `none` (partial index on `(household_id, received_at DESC) WHERE llm_state = 'awaiting'`) | `mail.process`, the `mail.llm` job, `mail.llm.catchup` |
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

## 5a. LLM first (D-107)
`ai_settings.mail_mode` decides who goes first. It is edited in Settings → AI.

| | `llm_first` (default) | `rules_first` (the M4 behaviour) |
|---|---|---|
| Queued | **Every** processed message, ignored categories included, one `mail.llm` call for all four decision points, plus extraction when the resulting category's behaviour is `extract` and `mail_extract_mode` is not `off` | Only when a point has `pending_llm`, or the extraction fallback applies (§6) |
| Rules | Run inline first; their result applies at once as a provisional value, so nothing waits on the GPU | Run inline first and decide whatever reaches the rules threshold |
| LLM answer | **Replaces** the rules' decisions (recorded through `Pipeline::record_stage_or`, so the model's decision is the latest) and refines their pending proposals (below) | Fills what the rules left open |
| Per-point `llm` stage | A point is asked unless its `llm` stage exists and is switched off; a point with no stage is asked at the default threshold 0.7 | The stage must exist and be enabled |

**What the model never changes:**
- a point where a person decided (the latest decision is `human` and final, which is what a label writes): it is not asked for, so a label survives;
- an item a person approved, rejected or edited, or that the model already made. The store guards each refinement in SQL (`ProposalRefineRepo`): events must be `extracted` or `proposed` with no `decided_at`, bills `proposed` at version 1, tasks `suggested` at version 1, all `extracted_by = rules`. An edit bumps the version, an approval or rejection changes the status, so none of them can be refined.

**Refining pending proposals.** For items of the message that the rules proposed and nobody touched, the model's item takes over instead of adding a second one. The match uses the dedupe inputs: an event by its start slot (local day and quarter hour, the same slot as `event_dedupe_key`), and among several in a slot by title similarity of 0.6 or more; a bill by amount and due date (a missing date matches); a task by sort (a model `reply` takes a rules `reply`, anything else takes a rules `action`). The match takes the model's title (events: also location and confidence; bills: payee, due date, confidence and the new dedupe key unless another bill holds it; tasks: due date and kind) and sets `extracted_by = llm`. A model event or bill for the same slot or amount as an item that is no longer pending is dropped as a duplicate, so a rejected or approved proposal is not proposed again. Rules proposals that the model did not match stay.

**State.** `message.llm_state`: `queued` when the job is queued, `done` when it has run (also when there was nothing to ask), `awaiting` when no model could answer (below). Existing rows stay `none`; `rules_first` queues set `queued` and `done` too.

**Fallback and backlog (§6a).** With no provider, no endpoint or a disabled endpoint, `mail.process` sets `awaiting` and queues nothing, and a `mail.llm` job that finds the endpoint off sets `awaiting` instead of dropping the message. After the last failed attempt (an unreachable server, a timeout, bad JSON) the message is `awaiting` too. The job runner passes no attempt number to a handler, so the handler reads `attempts` and `max_attempts` of its own running `job` row (`LlmStateRepo::running_attempt`).

**Auto actions.** `mail.auto_actions` is queued by `mail.process` with the **rules'** category, so the label, archive and mark-read actions of that category apply before the model answers. When the model changes the category, the new category's actions run too (a new job key, `mail.auto_actions:<message>:llm`). Nothing the first run did is reverted; the actions stay undoable for 30 days as before.

## 6. The `mail.llm` job
**Trigger (`rules_first`; in `llm_first` every message, §5a).** `mail.process` enqueues `mail.llm {message_id}`, with idempotency key `mail.llm:<message_id>` so a message is queued at most once, when the endpoint is enabled and either of these holds:
- a decision point has `pending_llm`;
- the category's behaviour is `extract` and either `mail_extract_mode = always`, or the mode is `fallback` and no rule extractor produced an item at `ai_settings.extract_fallback_cutoff` (default 0.8) or above.

The rules threshold per decision point (the `rules` stage's `accept_threshold`, default 0.8, 0.5 to 0.99) is editable in Settings → AI; saving it is a new pipeline version, as the `llm` toggle is.

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
- the inbox shows an "AI · NN%" chip with the confidence ("AI" for tasks), a tooltip on wide screens and a sheet on phones, and an "AI only" filter chip that is not remembered (D-104, Q-097).

**Retries.** The job is enqueued with `max_attempts = 12`. The worker's backoff is `30 s × 2^attempts`, so the queue keeps retrying for about a day while the GPU PC is off. `Unavailable` and `Timeout` fail the job for a retry. `BadJson` and `Schema` fail it the same way, because the model may answer differently next time. After the last attempt the job is `dead`, the message keeps its rule or default values and is `awaiting` for the catch-up (§6a).

## 6a. Catch-up (`mail.llm.catchup`, D-107)
Queues `mail.llm` for the messages with `llm_state = awaiting` that were received within `catchup_days` (a household calendar day count, from local midnight; 0 switches the catch-up off), newest first, 200 per batch. Each batch moves its messages to `queued` in the same transaction as the jobs. The jobs run behind live mail (priority -1) and in newest-first order (staggered `run_at`); idempotency key `mail.llm:<message>:cu:<unix seconds>`, because the original key is spent. The provider's concurrency limit (one call at a time) paces the burst.

It does nothing unless messages wait, a client exists and a cheap reachability probe succeeds: the same `ping` as the Settings test, logged as `purpose = endpoint.test`. Triggers:
- `PUT /ai/llm-endpoint` with `enabled = true` (a save or a switch-on): payload `{household}`, key `mail.llm.catchup:<household>:<unix ms>`;
- the cron `*/15 * * * *` (registered in `fa_mail::default_schedules`): empty payload, so it visits every household with waiting messages.

Messages older than the window stay rules-only (`awaiting`, and they show in the backlog count).

## 7. Failure modes
| Situation | Behaviour |
|---|---|
| No endpoint or `enabled = false` | The rules' result stands. `llm_first`: the message is `awaiting` and is caught up when an endpoint is saved or enabled (§6a). `rules_first`: `mail.llm` is not enqueued. `/ai/recipes/extract` returns JSON-LD results or 503 `llm_unavailable` |
| GPU PC asleep or Ollama down | `LlmError::Unavailable` → retry with backoff (§6); the rules' result or the default applies meanwhile; after the last attempt the message is `awaiting` and the 15-minute catch-up picks it up once the server answers (§6a) |
| Model missing on the server | `ping` reports `model_found: false` in Settings; calls fail as `Http(404)` |
| Slow model | `timeout_ms` per call; one call in flight per household |
| Malformed or schema-violating JSON | `BadJson` / `Schema`; nothing is applied; retried |
| Low confidence | below the stage threshold → human stage, as before |

## 8. API (adult-only; plan/M4.md fixes the shapes)
| Endpoint | Purpose |
|---|---|
| `GET/PUT /ai/llm-endpoint` | Read or replace the endpoint. `api_key`: absent keeps it, `null` clears it, a string sets it. An enabled save also enqueues `mail.llm.catchup` (§6a) |
| `POST /ai/llm-endpoint/test` | Ping stored or unsaved values: `{ok, latency_ms, model_found, error}` |
| `GET /ai/llm-endpoint/models` | `{models: string[]}` from the server |
| `GET/PUT /ai/settings` | `mail_extract_mode`, `max_input_chars`, `mail_mode`, `catchup_days`, `extract_fallback_cutoff`. On `PUT` the last three are optional: absent keeps the stored value |
| `GET /ai/backlog` | `{awaiting, queued}`: messages waiting for the LLM, and queued for it |
| `GET /ai/pipeline`, `PUT /ai/pipeline/{decision_point}` | The per-point `llm` toggle and threshold, and `rules_threshold` (0.5 to 0.99, default 0.8; optional on `PUT`; a change is a new version) |
| `GET /ai/stats?days=7` | Calls per purpose (total, ok, p50 and p95 in ms) and decisions per stage |
| `POST /ai/recipes/extract` | `{url}` or `{html}` → `RecipeDraft` (nothing stored) |

Settings → AI (the web app, Frontend.md, under "Sources and rules" after Mail, D-104 Q-096) shows the endpoint card, the test result, the model picker, the mail mode, the catch-up window and backlog, the extract mode, the per-point toggles (with the rules threshold) and the stats card.

## 9. Recipe extraction contract (for M5)
- **Deterministic first:** `from_html` reads schema.org `Recipe` JSON-LD, including `@graph`, arrays and `HowToSection` steps.
  - ISO 8601 durations become integer minutes (`PT1H30M` → 90; unparseable → `null`).
  - Ingredient `text` is always the source line. `quantity`, `unit` and `item` come from a small rule parser that handles fractions, unicode fractions and ranges (taking the low end) with a fixed unit list. When it is unsure they are `null`.
- **LLM fallback:** `from_llm` gets the page's visible text (truncated to `max_input_chars`) and the `RecipeDraft` schema.
- **`RecipeDraft`:** `{name, source_url, origin: jsonld|llm, yield_text, servings, prep_minutes, cook_minutes, total_minutes, ingredients[{text, quantity, unit, item}], steps[], tags[], image_url}`.
- **Fetching:** `fa_connectors::http::Http` with private hosts blocked (on redirects too), its body cap and its timeout. M5 moves `RecipeDraft` into `fa-domain` and adds storage (Domain.md `Recipe.import_method: jsonld|microdata|llm|manual`).
- **M5 (D-109, D-113; [Meals.md](Meals.md) §4.1, §4.2):** `RecipeDraft` moves to `fa-domain` (re-exported by `fa-recipe`); `fa-recipe` adds schema.org microdata (`origin: microdata`, tried after JSON-LD and before the LLM) and pasted-text input (straight to the LLM); `POST /recipes/import` replaces `POST /ai/recipes/extract`, which stays one milestone as an alias. A second LLM purpose, `ingredient`, proposes a catalogue match for ingredient lines the rules cannot match (`ingredient.llm` job in `fa-meals`); an adult confirms every proposal.

## 10. Eval (basic, D-028)
- `fa-server eval --household <id> --decision-point <dp> [--with-llm] [--limit N] [--json]` runs over `message.in_eval_sample` messages.
- The **truth** is the human decision for that point; messages without one are skipped.
- It reports, per stage (`rules`, then `llm`, then the remainder to `human`): items decided, correct, p50 and p95 latency. Overall it reports accuracy and coverage (the share decided before `human`).
- With `--with-llm` the `llm` stage uses its stored threshold even when the stage is off, else 0.7; a failed LLM call counts as undecided (D-104, Q-095).
- `--with-llm` honours `mail_mode`: in `rules_first` the model sees only what the rules leave open; in `llm_first` it answers every item, and the rules decide only what it leaves open (a low-confidence or failed answer falls back to the rules).
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
