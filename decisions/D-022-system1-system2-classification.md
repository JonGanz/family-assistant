---
id: D-022
title: Tiered email classification: rules → System 1 (local, always-on) → System 2 (GPU, batched)
status: accepted
date: 2026-10-23
source_questions: [Q-022, Q-023]
supersedes: D-016
superseded_by:
affects: [arch/Overall.md, arch/Ingest.md (future)]
---

# D-022: Tiered email classification: rules → System 1 (local, always-on) → System 2 (GPU, batched)

## Context
Q-023 suggested a locally hosted 'System 1' model to categorise the emails that rules can't place, before anything else runs. Q-022: the core node is on the same LAN as the GPU node. Instead of fixed specs, the user said either to probe the hardware or to make it a feature toggle.
This **supersedes D-016**, whose CPU model only answered 'is it time-sensitive?'.

## Decision
**Yes, it makes sense,** and it fits D-002 (the system works on its own and AI makes it better). The email pipeline becomes a cascade where each tier handles only what the tier before it couldn't settle:
1. **Rules (deterministic):** Gmail category labels, List-Unsubscribe, Org sender and domain maps, keyword rules. These settle most mail at no cost.
2. **System 1 (core node, CPU, always available, `llm.system1.enabled` toggle):**
   - **a. Embedding kNN.** A small embedding model (e.g. `nomic-embed-text`, about 0.3 GB) embeds subject, sender, and snippet, and pgvector finds the nearest *labelled* messages. Labels come from rule hits and from user corrections. This is fast and close to deterministic, and it learns each family's own mail.
   - **b. Small instruct model** (about 1.5–3B, Q4). Used only when the kNN neighbours disagree or there are too few labelled examples. It returns grammar-constrained JSON: `{category, time_sensitive, confidence}`.
   - A result at or above `system1.accept_threshold` is used. A `time_sensitive` result sends a notification straight away.
3. **System 2 (GPU node, 7–8B, batched, D-020):** handles low-confidence leftovers and all *extraction* work (events, bills, tasks, member linking hints). Its labels also feed the System 1 kNN set.

**Hardware probe:** at start-up `fa-server` reads total and available RAM and CPU cores (`sysinfo` crate). It suggests a System 1 profile and shows it in settings: `off`, `embed-only` (under 4 GB free), `embed+1.5B`, or `embed+3B` (8 GB or more free). The toggle and profile can always be overridden by hand. Wake-on-LAN is confirmed possible because both nodes are on the same LAN.

## Consequences
- **User corrections** (re-categorising a message in the UI) become the training data. A 'Fix category' action is needed in the email views.
- Embeddings are now created on the core node when System 1 is enabled, not only on the GPU node (an adjustment to D-002).
- Ollama on the core node becomes a regular service, still behind a toggle.
- **Escalation rule:** System 2 only receives an item if System 1's confidence is under the threshold or the item needs extraction.

## Alternatives considered
- Keeping D-016 as it was (triage only). That wastes the always-on capacity.
- Using only the small LLM for System 1. The kNN on embeddings is cheaper and adapts better to one family's mail.
