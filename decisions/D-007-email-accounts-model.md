---
id: D-007
title: Multiple Gmail accounts mapped to members; read-only by default with opt-in actions
status: accepted
date: 2026-10-23
source_questions: [Q-006]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/Ingest.md (future)]
---

> Extended by [D-052](D-052-mail-connectors.md) (IMAP as a per-source alternative) and [D-053](D-053-mail-scope.md) (default scope).

# D-007: Multiple Gmail accounts mapped to members; read-only by default with opt-in actions

## Context
There are several accounts, and some are shared between adults. A read-only flag should be on by default. Archive and mark-read would be useful. Testing-mode OAuth is acceptable.

## Decision
- `MailSource` has a many-to-many link to `Member` (owners). Classification uses the owner set as context. For example, mail to a shared parents' account can be routed to either adult or to a child.
- `read_only: true` is the default and requests scope `gmail.readonly`. Turning it off asks for re-consent with `gmail.modify` and enables these actions: archive, mark read, and apply `FA/*` labels. Actions are logged and can be undone.
- Sync is incremental via Gmail `history.list`, with a starting backfill window that can be set per source.
- With testing-mode OAuth, the refresh token expires after 7 days. The system detects `invalid_grant`, marks the source `needs_reauth`, and sends a notification with a re-auth link.

## Consequences
- Weekly re-auth is a cost the user accepted. Moving the OAuth app to 'internal' or 'published' later removes it.

## Alternatives considered
IMAP with app passwords. It's simpler, but it loses Gmail categories and history IDs, and Google is phasing it out.
