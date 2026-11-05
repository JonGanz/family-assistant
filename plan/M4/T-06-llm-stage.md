---
id: T-06
title: "llm stage, mail.llm job, Provider, llm.prune"
milestone: M4
wave: 2
depends_on: [T-03, T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-pipeline/**", "crates/fa-mail/**", "crates/fa-llm/src/provider.rs", "crates/fa-llm/src/lib.rs (the provider mod/pub use lines only)", "crates/fa-llm/Cargo.toml", "bins/fa-server/src (the MailDeps construction line only)"]
decisions: [D-093, D-094, D-029, D-027, D-057, D-060, D-002]
---

# T-06: `llm` stage, `mail.llm` job, `Provider`, `llm.prune`

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M4's track B is the LLM extraction slice (D-094). T-03 built the tables, types and repos, and T-04 built the `fa-llm` client. You connect them to the email pipeline. The LLM is called from a deferred job, never inline, and nothing it extracts is applied without approval.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`: no content in logs or tables, no real data, runtime queries.
- `plan/M4.md`: *`fa-llm`* (the `Provider` block) and *`llm` stage and the `mail.llm` job*. **Use `extracted_by`** where the plan says `origin`; see T-03's report and `arch/AI.md` §4.
- `arch/AI.md` §3, §5, §6 and §7; `arch/Ingest.md` §5, §7 and §13.
- Decisions D-029 (stages, `on_unavailable`), D-027 (decision points), D-057, D-060 (approval only), D-093 and D-094.
- T-03's and T-04's reports (in `log/` or the main session's notes in this brief) for the exact repo and API names.
- Code:
  - `crates/fa-pipeline/src/lib.rs` (`Pipeline::run`, `validate_version`, `Outcome`) and `crates/fa-pipeline/tests/pipeline.rs`;
  - `crates/fa-mail/src/process.rs` (`decide`, `process`, `handlers`, `default_schedules`, `enqueue`), `process/extract.rs` and `process/facts.rs`;
  - `crates/fa-mail/src/lib.rs` (`MailDeps`, `handlers`);
  - `crates/fa-store/src/repo/jobs.rs` (`NewJob { max_attempts, idempotency_key }`, and `fail`: backoff is `30 s × 2^attempts`, `dead` after `max_attempts`);
  - `crates/fa-store/src/store.rs` (`Store`) and `crates/fa-store/src/secret.rs` (`MasterKey`).

## You own
- `crates/fa-pipeline/**` and `crates/fa-mail/**`.
- `crates/fa-llm/src/provider.rs`, the two `mod provider; pub use provider::Provider;` lines in `crates/fa-llm/src/lib.rs`, and `crates/fa-llm/Cargo.toml` (add `fa-domain` and `fa-store`).
- **Not yours:** the rest of `fa-llm`, `fa-recipe` (T-07, running in parallel), `fa-api`, `bins/**`, `fa-store`. If a repo is missing, write the query in `fa-mail` as a runtime query and report it.

## Work
1. **`Provider`** (`crates/fa-llm/src/provider.rs`), exactly as in the plan (`new`, `client_for`, `call`, `invalidate`):
   - **Endpoint lookup:** read `LlmEndpointRepo::get` plus the API key secret (opened with the C-10 AAD through the existing secret repo). `None` when there is no row or `enabled = false`.
   - **Cache:** 30 s per household, a `Mutex<HashMap>`; `invalidate` clears it.
   - **Concurrency:** a `tokio::sync::Semaphore` with 1 permit per household, so one call is in flight.
   - **`call`:** take the permit, run `LlmClient::extract`, then write `LlmCallRepo::insert` (purpose, model, `started_at`, latency, ok, error kind, tokens; no content) in its own short transaction. It returns the client's result. No client means `LlmError::Config("disabled")`.
2. **`fa-pipeline`:**
   - `Outcome` gains `pub pending_llm: bool`.
   - In `run`, when `rules` is below its threshold and the next enabled stage is `Llm`, set `pending_llm = true`, record the default as `human` does today, and stop.
   - Add `Pipeline::record_stage(conn, subject, dp, StageKind::Llm, candidates, version) -> Result<Outcome>`. A candidate at or above the `llm` stage's `accept_threshold` becomes a `Decision` with `stage_kind = llm`; otherwise the outcome is `human` with the default kept.
   - Add `Pipeline::set_llm_stage(conn, household, dp, enabled, threshold) -> Result<PipelineVersionId>`. It copies the active version, inserts or updates the `llm` stage right before `human` (`on_unavailable = queue`, `config = {}`, default threshold 0.7), validates it and activates it. The old version stays for rollback.
   - Tests cover each path, including that the last enabled stage stays `rules` or `human`.
3. **Enqueue in `mail.process`.** After the pipelines and the rule extractors run, enqueue `mail.llm {"message_id": uuid}` when the endpoint is enabled (`Provider::client_for(..).is_some()`, or a cheap repo check) **and** either:
   - any of the four email decision points has `pending_llm`;
   - the category's behaviour is `extract` and either `ai_settings.mail_extract_mode = always`, or `fallback` and no rule extractor produced an item at 0.8 or above.

   Use `NewJob { kind: "mail.llm", idempotency_key: Some("mail.llm:<message_id>"), max_attempts: 12, .. }`. With 12 attempts the backoff spans about a day.
4. **`mail.llm` job** (a `JobHandler` in `crates/fa-mail/src/process/llm.rs` or similar):
   - **Request:** `MailLlmResult` (derives `Deserialize` and `JsonSchema`), as in the plan's shape. Build the request schema with only the parts asked for (pending decision points; extraction parts when extraction was triggered). Use a separate struct per combination, or optional fields described in the prompt; choose and report.
   - **Prompt:** the system prompt states the household time zone and the allowed values in stable order (category keys sorted, member keys by id). The user prompt has the subject, sender, received date (household time zone) and the plain-text body truncated to `ai_settings.max_input_chars`.
   - **Decisions:** record them with `record_stage`. If `email.category` changed, re-run the message's category behaviour step, which must be idempotent. Reuse the existing function; refactor it to be callable if needed.
   - **Extracted items:** events, bills and tasks go through the **existing** insert, dedupe and enqueue paths (`calendar.route`, `tasks.from_mail`, and the bill path). Then call `ExtractedByRepo::set(.., ExtractOrigin::Llm)`. Items below the `llm` stage threshold, and any LLM item that duplicates a rule item (same dedupe key), are dropped. Nothing is auto-applied (D-057, D-060).
   - **Errors:** `Unavailable` and `Timeout` return a `JobError` (retry). So do `BadJson` and `Schema`, logged as their kind. `Config("disabled")` completes the job without doing anything.
   - **Logs:** ids, counts, latency and kinds only. Never the subject, body, sender or LLM output.
5. **`llm.prune`:** a daily job (`0 4 * * *`, added to `default_schedules()`) that deletes `llm_call` rows older than 90 days, per household. Both handlers go into `fa_mail::handlers()`.
6. **`MailDeps`** gains `pub llm: Option<Arc<fa_llm::Provider>>` (`None` means the LLM is off: no `mail.llm` jobs are enqueued). Update the fa-mail tests' deps builders. You also own the single `MailDeps { .. }` construction in `bins/fa-server` to add `llm: None`, so the workspace stays green between waves. T-08 replaces it with `Some(provider)`. Report the exact constructor call T-08 needs.

## Tests (fixtures fictional, `example.com`; fresh container on port 55406; the axum stub from T-04's tests as the LLM)
- A message whose rules give `email.category` 0.5 with `llm` enabled: `pending_llm` is set and one `mail.llm` job is queued. A second process of the same message queues no second job.
- The job against the stub, which returns category `school` at 0.9: a `Decision(stage_kind = llm)` is recorded, the behaviour re-runs, and an event comes out as a proposal with `extracted_by = llm` that is not written to a calendar.
- A stub confidence of 0.4 leaves the human default.
- With the stub down, the job fails and is re-queued with `run_at` in the future; `llm_call` gets `ok = false`, `error_kind = unavailable`.
- `mail_extract_mode`: `off` never extracts; `fallback` is skipped when a rule extractor scored 0.9; `always` runs.
- `llm_call` rows hold no subject or body text: assert that the fixture subject string appears in no column.
- `set_llm_stage` versions, and prune.

## Notes from wave 1 (main session)
- T-03: mark LLM-made items with `ExtractedByRepo::set(conn, ExtractTarget::{Event,Bill,Task}, id, ExtractOrigin::Llm)` after the existing insert. The API key is a secret: `SecretRepo::put(.., "llm_endpoint", endpoint.id, SecretKind::ApiKey, ..)`, which you read through the existing secret repo (C-10 AAD).
- T-04: `fa_llm::json_schema_for` adds `additionalProperties: false` but does not list `Option` fields in `required`. Ollama does not enforce strict mode. Keep `MailLlmResult` parts that are optional as `Option` and tolerate a missing key. Do not change `schema.rs` unless a test shows it is needed, and if you do, report it.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] On a fresh container (port 55406): `cargo test -p fa-pipeline -p fa-mail -p fa-llm` passes.
- [ ] `cargo clippy -p fa-pipeline -p fa-mail -p fa-llm --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes.
- [ ] The report lists:
  - the `Provider` constructor and the `MailDeps` change, for T-08;
  - the schema strategy for `MailLlmResult`;
  - the job and schedule names;
  - deviations;
  - "list what outside your paths needs changing";
  - README changes needed: none (T-17 writes the AI docs).

## Outcome (T-06 report)
- **`fa-llm`:** `Provider` (`new(store, key, http)`, `client_for`, `call` writes `llm_call`, `invalidate`).
- **`fa-pipeline`:** `Outcome.pending_llm`, `record_stage` (LLM only) and `set_llm_stage`, with a test.
- **`fa-mail`:**
  - `process/llm.rs` holds `MailLlmResult`, the `mail.llm` job (key `mail.llm:<id>`, `max_attempts` 12) and `llm.prune` at `0 4 * * *`. Items are approval-only (bills `Proposed`, tasks `suggest`), and duplicates of rule items are dropped.
  - `MailDeps.llm: Option<Arc<Provider>>`.
  - `tests/llm_flow.rs` (7 tests).
- **Deviations:**
  - The job payload is `{message_id, household, rules_strong}`.
  - The `llm` stage sets `pending_llm`, and a later stage that accepts clears it.
  - When the category is still pending, the one call also asks for extraction.
  - A category change re-queues `mail.auto_actions` and re-runs `extract`.
  - The item threshold is the `email.category` `llm` threshold, or 0.7.
- **Main-session fixes at integration:**
  - `fa-tasks` `MailTaskRequest.extracted_by` now marks LLM tasks through `ExtractedByRepo::set`.
  - `llm: None` was added to six fa-api test files.
  - `extracted_by: None` was added to `MailTaskRequest` test literals.
- **Known gaps** (follow-ups):
  - An LLM `time_sensitive = true` sends no notification.
  - A later `mail.process` re-run records a default decision that hides the earlier LLM one.
  - LLM task due dates are set to 23:59 local.
