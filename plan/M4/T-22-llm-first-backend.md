---
id: T-22
title: "LLM-first mail processing, rules fallback and catch-up queue (backend)"
milestone: M4
wave: 6
depends_on: [T-20, T-21]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-store/migrations/20261107000001_llm_first.up.sql", "crates/fa-store/migrations/20261107000001_llm_first.down.sql", "crates/fa-store/src/**", "crates/fa-store/tests/**", "crates/fa-store/.sqlx/**", ".sqlx/**", "crates/fa-domain/src/ai.rs", "crates/fa-domain/src/lib.rs", "crates/fa-pipeline/**", "crates/fa-mail/**", "crates/fa-tasks/src/ingest.rs", "crates/fa-api/src/ai.rs", "crates/fa-api/tests/**", "bins/fa-server/src/**", "crates/fa-eval/**", "arch/AI.md", "arch/Ingest.md", "arch/src/AI.py", "arch/src/Ingest.py", "arch/AI.drawio", "arch/Ingest.drawio"]
decisions: [D-107, D-094, D-057, D-060]
---

# T-22: LLM-first mail processing (backend)

You are a sub-agent on milestone M4 of the Family Assistant project, a self-hosted family organiser: a Rust workspace (axum, sqlx with runtime queries, utoipa, ts-rs) plus a Vue app. M4 added an `llm` pipeline stage and a `mail.llm` job that ask a local OpenAI-compatible LLM only when the rules are unsure. The user now wants the reverse (D-107): every message goes to the LLM when one is available, the rules are the fallback, and messages processed while the LLM was unavailable are caught up once it is back.

## Read first
- `CLAUDE.md` ("Implementation via sub-agents"), `plan/_rules-common.md`, `plan/M4/_rules.md`, `plan/M4.md` (Contracts: the `llm` stage and `mail.llm` job sections)
- `decisions/D-107-llm-first-mail.md` (the spec), `D-094`, `D-057`, `D-060`
- `arch/AI.md` §4–§6, `arch/Ingest.md` §4–§5
- `crates/fa-mail/src/process.rs` (where `mail.llm` is enqueued, around `wants_extract`), `crates/fa-mail/src/process/llm.rs` (`run_llm`, the job handler, `rules_strong`), `crates/fa-pipeline`, `crates/fa-domain/src/ai.rs` (`AiSettings`), `crates/fa-store` (AI repos, message repo), `crates/fa-api/src/ai.rs`, `crates/fa-mail/tests/llm_flow.rs`

## You own
The paths in `owns`. Never edit an existing migration; add the new one. If you need a path outside `owns`, stop and report.

## Contracts (fixed; the web task T-23 builds against them)
- **Migration `20261107000001_llm_first`:**
  - `ai_settings.mail_mode text NOT NULL DEFAULT 'llm_first' CHECK in ('llm_first','rules_first')`
  - `ai_settings.catchup_days int NOT NULL DEFAULT 30 CHECK (catchup_days BETWEEN 0 AND 365)`
  - `message.llm_state text NOT NULL DEFAULT 'none' CHECK in ('none','queued','done','awaiting')`, with a partial index on `(household_id, received_at DESC) WHERE llm_state = 'awaiting'`
  - Existing rows keep `none`.
  - Use the actual column names of `message` (check `received_at` and the household column).
  - Include a `down` file.
- **Domain (`fa-domain/src/ai.rs`, ts-rs plus `ToSchema`):**
  - `MailMode { LlmFirst => "llm_first", RulesFirst => "rules_first" }`;
  - `AiSettings` gains `mail_mode: MailMode` and `catchup_days: u32`, and the `get_or_default` defaults match the migration;
  - a new `LlmBacklog { awaiting: u32, queued: u32 }`.
- **Rules thresholds (rules_first):**
  - `GET /ai/pipeline` items gain `rules_threshold: f32`, and `PUT /ai/pipeline/{decision_point}` accepts an optional `rules_threshold` (0.5–0.99). Saving it is a new pipeline version, as the llm toggle is.
  - `ai_settings.extract_fallback_cutoff real NOT NULL DEFAULT 0.8` (in the same migration) replaces the hard-coded 0.8 of `Fallback`.
- **API (`fa-api/src/ai.rs`, adult-only, OpenAPI):**
  - `GET`/`PUT /ai/settings` carry the new fields;
  - `GET /ai/backlog` returns `LlmBacklog`.

## Work
1. **Enqueue (`mail.process`):**
   - **llm_first:** every processed message queues `mail.llm` (same idempotency key) when a client is configured and enabled. Set `llm_state = queued`. With no client, or with the endpoint disabled, set `llm_state = awaiting` instead.
   - **rules_first:** the current behaviour (`pending_llm` or `wants_extract`), using the configurable cutoff.
   - In both modes the rules' result applies at once, as today.
2. **The `mail.llm` job:**
   - In llm_first, ask for all four decision points, plus extraction when the category (after the LLM's own category) has the extract behaviour.
   - On success, set `llm_state = done`.
   - When the endpoint is disabled, set `awaiting` instead of silently dropping.
   - On the final failed attempt, set `awaiting`. Read how the job runner exposes the attempt number; if it can't, mark `awaiting` on every transient failure and `queued` again when retried.
3. **Override with respect for people (llm_first):**
   - LLM decisions replace rules decisions through `record_stage`. This applies only when no `human` decision or label exists for that point.
   - For extracted items, a still-pending proposal from the rules for the same item (use the existing dedupe key) takes the LLM title, fields and `extracted_by = llm`. Never touch an item a person has approved, rejected or edited (check how D-057 and D-060 record that).
   - A changed category re-runs the category behaviour (existing path).
   - Note in the report what happens to auto mail actions the rules already applied.
4. **Catch-up:**
   - A new job `mail.llm.catchup {household}` (idempotency key `mail.llm.catchup:<household>`) queues `mail.llm` for `awaiting` messages received within `catchup_days`, newest first, in batches. It sets them to `queued`.
   - It is enqueued:
     - when `PUT /ai/llm-endpoint` saves an enabled endpoint;
     - when the endpoint is switched on;
     - by a cron `*/15 * * * *` that runs only when `awaiting > 0` and a cheap reachability probe succeeds. Reuse the endpoint-test call; it logs as `purpose = endpoint.test`.
   - Register it in `fa-server`, next to `mail.llm` and `llm.prune`.
5. **Eval (`fa-eval`):** `--with-llm` honours `mail_mode` (llm_first means the LLM answers every point). Keep the D-104 threshold rule.
6. **Tests:**
   - Extend `crates/fa-mail/tests/llm_flow.rs` (fake LLM server, as it does today) to cover:
     - the llm_first enqueue;
     - the override of rules decisions and of pending proposals, with the title replaced;
     - no override after a human decision, approval or edit;
     - disabled to `awaiting`, then enabling triggers catch-up;
     - the `catchup_days` window;
     - unchanged rules_first behaviour with the configurable cutoff.
   - Add store tests for the new repo functions and API tests for the new routes and fields.
7. **Arch:** update `arch/AI.md` and `arch/Ingest.md`, plus their `arch/src/*.py`, regenerated, to describe the built behaviour with the final names. Bump `updated:`.

## Acceptance
- [ ] `scripts/ci-local.sh --no-web` passes (copy the final lines and the test totals from its output)
- [ ] `cargo fmt --all`, clippy clean
- [ ] Report: files changed, deviations from the contracts (names), the auto-action behaviour on a category change, README changes needed, and "list what outside your paths needs changing"
- [ ] Never run `scripts/dev-run.sh`, never bind 8080/5173/18091, never touch the dev DB at 127.0.0.1:55432, and run no state-changing git commands
- [ ] Disk: stop below 6 GB free

## Outcome (T-22 report)
- **Migration `20261107000001_llm_first`:** adds `ai_settings.mail_mode` (default `llm_first`), `catchup_days` (default 30; 0 means no catch-up) and `extract_fallback_cutoff` (default 0.8, check 0.5–1, API validates 0.5–0.99), plus `message.llm_state` with a partial index on `awaiting`.
- **Domain types:** `MailMode`, `LlmState`, `LlmBacklog` and `AiSettingsInput`. The request fields are optional, so old clients keep working.
- **Store:** `LlmStateRepo` and `ProposalRefineRepo`. The SQL guards change only untouched rule proposals.
- **Pipeline:** `record_stage_or` and `set_stages` (the rules threshold and the llm stage saved in one version).
- **fa-mail:**
  - In `llm_first`, every message queues `mail.llm`; with no client it is marked `awaiting`. `rules_first` uses the configurable cutoff.
  - The job asks every point a person has not decided and whose stage is not switched off, refines pending proposals, and sets `done`. A disabled endpoint, or the last failed attempt (read from its own `job` row), sets `awaiting`.
  - `mail.llm.catchup`: keys `mail.llm.catchup:<household>:<unix ms>`, a `*/15` cron, per-message jobs staggered newest first. It is registered through `fa_mail::handlers` and `default_schedules`.
- **API:** the settings, pipeline and endpoint routes carry the new fields; an enabled endpoint save queues the catch-up. The main session registered `GET /ai/backlog` in `lib.rs` and `openapi.rs` and un-ignored its test.
- **Eval:** honours `mail_mode`.
- **Tests:** `fa-store/tests/llm_first.rs` (5 tests), 9 new tests in `llm_flow.rs`, plus API and eval tests. The job handlers take a clock (`handlers_with_clock`), which fixed a test that relied on today's date.
- **Category change:** auto actions of the rules' category stay applied (undoable for 30 days), and the new category's actions also run (`mail.auto_actions:<msg>:llm`). Pending proposals from the old category stay.
- **deny:** `RUSTSEC-2026-0441` (`pin-utils` via `async-imap`) is ignored with a reason, and the stale zip ignores are dropped (main session).
- **Follow-ups:**
  - A task from the model becomes a second suggestion if `tasks.from_mail` had not yet run.
  - A model flipping `time_sensitive` sends no notification (the M4 gap).
  - In `llm_first`, `llm_enabled` on a point means "not switched off"; T-23 handles the UI.
