---
id: D-107
title: LLM-first mail processing with rules fallback and catch-up queue
status: accepted
date: 2026-11-07
source_questions: []
supersedes: 
superseded_by:
affects: [arch/AI.md, arch/Ingest.md, arch/Frontend.md]
---

# D-107: LLM-first mail processing with rules fallback and catch-up queue

## Context
Direct user instruction (2026-11-07), following the R4 discussion: 'my ultimate preference here would actually be for all items to be processed with the LLM if it's available, but we fall back to the deterministic scanning when we cannot reach or have an LLM enabled. Once that option is restored/added though, we should have those queued up for processing.' The aim is better task titles and categories than the rules give.

Today (D-094, AI.md §5 and §6) the LLM is asked only when the rules fall below their threshold or extract nothing. When no endpoint is configured, no job is queued. When it is disabled, a queued job is dropped. When it is unreachable, the job retries 12 times and then dies. Nothing catches up later.

## Decision
1. **Mode.** `ai_settings.mail_mode` is `llm_first` or `rules_first`, default `llm_first`, edited in Settings → AI. `rules_first` is the M4 behaviour.
2. **llm_first.**
   - Every processed message, ignored categories included, gets one `mail.llm` call covering all four decision points, plus extraction when the resulting category's behaviour is extract.
   - The rules still run inline first, and their result applies at once as a provisional value, so nothing waits on the GPU.
   - The LLM result replaces the rules' decisions and their still-pending proposals (inbox items, suggested tasks). For a duplicate proposal it keeps the LLM title.
   - It never changes anything a person has approved, rejected, edited or labelled.
   - A changed category re-runs the category behaviour, as today.
3. **Fallback and backlog.**
   - When the endpoint is missing, disabled or unreachable (retries exhausted), the message keeps the rules' result and is marked as waiting for the LLM (`message.llm_state = awaiting`).
   - When the endpoint is saved or enabled, or the next health check reaches it again, a `mail.llm.catchup` job queues the waiting messages, newest first.
   - Only messages received within a window are queued: `ai_settings.catchup_days`, default 30. Older ones stay rules-only.
   - The provider's concurrency limit paces the work.
4. **Rules threshold.** In `rules_first` the rules threshold per decision point (default 0.8) and the extraction fallback cutoff (default 0.8) become settings in Settings → AI.

## Consequences
- Needs a migration: `ai_settings.mail_mode`, `ai_settings.catchup_days`, and `message.llm_state` (`none|queued|done|awaiting`).
- Changes `mail.process` and the `mail.llm` job, adds the catch-up job, and adds an override path that respects human edits.
- Adds API fields and Settings → AI controls.
- GPU load grows to one call per message. A first enable with a 30-day window queues a burst of messages, paced by the provider.
- Follows the configurable-default rule: `rules_first` stays available.
- Planned as M4 wave 6 (T-22 backend, T-23 UI).

## Alternatives considered
- Keep rules-first and only expose the thresholds (offered first). This was rejected: it does not give the LLM's titles on mail the rules already handle.
- Run the LLM before the rules and hold results until it answers. This was rejected: mail would wait on the GPU PC, and nothing would happen while it is off.
- Catch up on every message ever received. This was rejected as the default because it could mean thousands of calls; the window is configurable instead.
