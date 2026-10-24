---
id: D-030
title: Email content is transient: purge after full processing, keep provenance links
status: superseded
date: 2026-10-23
source_questions: [Q-028]
supersedes:
superseded_by: D-034
affects: [arch/Domain.md, arch/Platform.md, arch/Overall.md]
---

> **Superseded by [D-034](D-034-configurable-mail-retention.md).** The provenance rules carry over. Content retention is now a configurable policy that defaults to keeping learning examples.

# D-030: Email content is transient: purge after full processing, keep provenance links

## Context
The user's rules: anything created from an email must record its source. Email *content* doesn't need to be stored after the email is fully processed, as long as we can point back to the source email.

## Decision
- **Provenance:** every derived entity (`Event`, `Task`, `Bill`, `Advisory`, `CategoryLabel`) has a `SourceRef{kind: email|ics|weather|manual|system, message_id?, external_url?}`. For email, the `external_url` is a Gmail deep link to the message in the right account. The UI shows a "from email ↗" link wherever an item came from mail.
- **Content lifecycle:** a `Message` holds `body_text`, the attachment text extracts, and the snippet only while `processing_state` is `pending`. Once every decision point is final and all S2 extraction is done (`processed`), a purge job deletes them and sets `content_purged_at`.
- **What we keep** after the purge: Gmail IDs (message, thread), source, received_at, decisions and labels, and a provenance link. Whether sender, subject, and the embedding are also kept is **[Q-032]**. The working assumption is that they are kept, because the email list, 'Fix category', and kNN need them.
- **Re-fetch on demand:** when content is needed again (eval sample replay D-028, pipeline dry-run on history D-029, re-processing), it is fetched from Gmail through the read-only scope and held only for that run. Gmail is the system of record. If a message was deleted from Gmail, it is skipped.
- **Stuck items:** if a message stays `pending` (for example, the GPU node never wakes), its content is kept for at most `retention.max_pending_days` **[Q-032]**. After that, the pending stages are marked `skipped` and the content is purged.

## Consequences
- This reduces what we hold that is sensitive. A DB backup no longer contains email bodies.
- Eval and dry-run become slower (Gmail API calls) and depend on the quota, so they batch and cache in memory per run.
- This refines D-028: the eval sample is stored as labels plus Gmail IDs, not content.

## Alternatives considered
Keeping bodies with time-based retention (the old option A). Not chosen, because the user wants the content gone once it has been processed.
