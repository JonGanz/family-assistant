---
id: D-052
title: Mail connectors: Gmail API and IMAP, chosen per source, default Gmail API
status: accepted
date: 2026-10-25
source_questions: [Q-049]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Domain.md]
---

# D-052: Mail connectors: Gmail API and IMAP, chosen per source, default Gmail API

## Context
D-007 chose the Gmail API. Its scopes are restricted, so each account re-consents every 7 days while the household's OAuth client is in Testing mode. IMAP with an app password never expires and works with any provider.

## Decision
M3 builds **both** connectors behind one sync interface. Each mail source has `provider: gmail | imap`; new sources default to `gmail`.
- **Gmail API:** as D-007 (`history.list` incremental sync).
- **IMAP:** host, port (993, TLS), username, and an app password stored as a `Secret`. Incremental sync per folder with UIDVALIDITY and UIDNEXT. When the server advertises `X-GM-EXT-1` (Gmail), it also reads `X-GM-MSGID`, `X-GM-THRID` and `X-GM-LABELS`, so Gmail categories and labels work as with the API.
- Everything after sync (pipeline, extraction, actions, retention) is provider-neutral.

## Consequences
- `message.gmail_id` becomes `remote_id` (the Gmail id, the hex `X-GM-MSGID`, or `uidvalidity:uid`).
- Gmail actions map to IMAP: archive = MOVE to the archive folder (Gmail: remove `\Inbox`), mark read = `\Seen`, label = `X-GM-LABELS` on Gmail or an IMAP keyword elsewhere.
- Non-Gmail IMAP messages have no web deep link; the app's message view stands in.
- Extends D-007; nothing is superseded.

## Alternatives considered
- Gmail API only (weekly re-consent per account).
- IMAP only (loses reliable Gmail categories on non-extension servers).
- IMAP in a later milestone.
