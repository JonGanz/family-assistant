---
id: D-034
title: Email retention is a configurable policy; content kept while it helps local learning; provenance always
status: accepted
date: 2026-10-23
source_questions: [Q-032]
supersedes: D-030
superseded_by:
affects: [arch/Domain.md, arch/Platform.md, arch/Overall.md]
---

# D-034: Email retention is a configurable policy; content kept while it helps local learning; provenance always

## Context
On Q-032 the user clarified that D-030's 'we don't need to keep it' was a permission, not a requirement. Content that helps train or evaluate a local model may stay as long as it helps. Retention should be **configurable**, especially for when the kNN layer is disabled.
This **supersedes D-030**. Its provenance rules carry over unchanged.

## Decision
- **Provenance (from D-030, unchanged):** every derived item has a `SourceRef` (Gmail deep link), shown in the UI as a 'from email ↗' link.
- **Metadata is always kept:** Gmail IDs, sender, subject, received_at, labels, and decisions. The email list, 'Fix category', and provenance titles depend on them.
- **Content policy** (`retention.mail.content_policy`, adults edit it in settings, it is included in config export):
  - `purge_after_processing`: D-030 behaviour. Bodies and attachment text are deleted once processing finishes.
  - `keep_for_learning` (**the default when kNN or eval is enabled**): content is kept for messages that are *learning examples*, and purged for the rest. Learning examples are:
    - every user-corrected message;
    - the eval sample (D-028);
    - the newest `retention.mail.examples_per_category` (default 200) messages per category that have rule or S2 labels.

    Kept content allows re-embedding when the embedding model changes (D-026), and offline eval or dry-run without re-fetching from Gmail.
  - `keep_all`: nothing is purged.
- **Derived policy:** when kNN is disabled, embeddings are dropped. When both kNN and eval are disabled, the suggested policy changes to `purge_after_processing`. The UI suggests it and the adult confirms.
- **Unprocessed content** is kept for at most `retention.mail.max_pending_days` (default 14). After that, pending stages are skipped, and content is kept or purged according to the policy.
- **Fallback:** if content was purged, it is re-fetched from Gmail when needed (D-030).

## Consequences
- A purge job evaluates the policy daily (`mail.retention`). The number of learning examples is bounded, so storage stays predictable.
- DB backups will contain the bodies of learning examples. They are protected by the NAS backup location and the tailnet boundary. Email bodies are not secrets, so they are not encrypted at rest. This can be revisited.

## Alternatives considered
- D-030 as written (always purge). Superseded: it was based on reading the user's permission as a requirement.
- Keep everything. That remains available as `keep_all`.
