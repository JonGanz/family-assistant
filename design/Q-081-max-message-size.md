---
id: Q-081
title: What happens to a mail message over the size limit
status: open
batch: 21
area: ingest
created: 2026-11-02
resolved:
decisions: []
---

# Q-081: What happens to a mail message over the size limit

## Question
Code review 2 (C2-01, C2-05): a Gmail message over about 7.5 MB stopped the mailbox from syncing for good (fixed in M3.7 with a 40 MB constant and a per-message skip), and IMAP fetches have no size bound. What should happen to a message over the limit, and should the limit be a setting?

## Options
- A. Configurable limit (default 40 MB); the message is stored as headers only with a 'too large' marker so it can still be categorised, and shown in Settings (suggested default)
- B. Skip it silently and count it
- C. Fixed limit, headers-only row

## Answer
<!-- answer below this line -->

