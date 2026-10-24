---
id: D-029
title: Decision pipelines are data, fully configurable in a settings UI
status: accepted
date: 2026-10-23
source_questions: [Q-027]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/AI.md (future), arch/Frontend.md (future)]
research: [R-001]
---

# D-029: Decision pipelines are data, fully configurable in a settings UI

## Context
In Q-027 the user suggested that "we should consider being able to fully configure the pipeline of decision making via a settings UI." Evaluation: **adopted**.
- It fits D-026: if the hardware varies, the stage order has to vary too.
- It fits D-018: config lives in the UI and is exportable.
- The cascades already designed (email, D-024; calendar routing, D-008) are just fixed pipelines. Turning them into data removes special cases.

## Decision
- Each **decision point** (D-027) has an ordered list of **stages**:
  - `rules(ruleset)`
  - `knn(k, min_agreement)`
  - `system_one(backend prefs)`
  - `llm(backend prefs)`
  - `human(approval inbox)`

  Each stage has `enabled`, an `accept_threshold`, and an `on_unavailable: skip|wait|queue` setting. The first stage whose result reaches its threshold wins. Otherwise the item falls through, and the last stage sets the default (usually `human` or `queue`).
- The engine lives in `fa-pipeline`. Its config is stored in the DB, edited in a **Pipeline settings** screen (adults only, D-006), and included in config import and export (D-018).
- **Guardrails:**
  - Every pipeline must end in a deterministic or human stage, so it works with no AI (D-002).
  - Changes are **dry-run** against the eval sample and recent history, and the UI shows the before and after accuracy and the share of items that reach each stage before you save.
  - Pipelines are versioned, so a change can be rolled back.
- **Presets:** 'Rules only', 'Reference hardware' (the default), and 'Max accuracy'. The profile suggester (D-026) proposes one.
- The UI shows live stats for each stage: hit rate, how often it escalates, and latency.

## Consequences
- D-008's routing order and D-024's cascade become the *default pipeline definitions*. Their behaviour is unchanged, so they are not superseded.
- M3 ships the engine with the 'Rules only' preset and the labelling view. M4 adds the AI stages, eval, and the Pipeline settings UI.
- More flexibility means more ways to misconfigure, which is why the dry-run and presets exist.

## Alternatives considered
A fixed cascade with only threshold settings. Not chosen: it can't adapt to different hardware.
