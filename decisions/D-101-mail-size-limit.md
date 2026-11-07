---
id: D-101
title: Configurable message size limit; oversized mail kept as headers only
status: accepted
date: 2026-11-07
source_questions: [Q-081]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-101: Configurable message size limit; oversized mail kept as headers only

## Context
Code review 2 (C2-01, C2-05): a Gmail message over about 7.5 MB once stopped a mailbox from syncing; M3.7 added a 40 MB constant and a per-message skip, and IMAP fetches have no size bound.

## Decision
The message size limit is a household mail setting, default 40 MB, applied to Gmail and IMAP fetches alike (IMAP fetches RFC822.SIZE first). A message over the limit is stored as headers only with a 'too large' marker, so the rules and the classifier can still categorise it, and Settings → Mail lists such messages per source.

## Consequences
Needs a household setting, a marker column on the message and a Settings list. LARGER item for the next milestone.

## Alternatives considered
B (skip silently and count), C (fixed limit, headers-only row).
