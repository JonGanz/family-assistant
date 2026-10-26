---
id: D-053
title: Each source reads everything except Spam and Trash, narrowed by an optional filter
status: accepted
date: 2026-10-25
source_questions: [Q-050]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-053: Each source reads everything except Spam and Trash, narrowed by an optional filter

## Context
Reading more mail gives better recall and more training examples (D-028); reading less keeps noise and private mail out.

## Decision
Every source has an optional filter. Default: read everything except Spam and Trash; the category rules then ignore promotions and social mail.
- Gmail API: a Gmail search string (for example `-category:social`, `label:kids`).
- IMAP: a folder list (default `INBOX`; Gmail IMAP uses `[Gmail]/All Mail`), plus a Gmail search string through `X-GM-RAW` when the server supports it.

## Consequences
Promotions and social mail are stored as metadata with the `ignore` behaviour (D-054), so they are hidden but still count as examples for M4.

## Alternatives considered
- Primary and Updates only.
- Only mail carrying a household-applied label.
