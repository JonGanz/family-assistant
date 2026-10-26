---
id: Q-050
title: Which mail does the system read?
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-050: Which mail does the system read?

## Question
A family Gmail account gets everything: school mail, bills, receipts, newsletters, promotions, social notifications, spam. Reading more gives better recall and more training examples (D-028). Reading less means less storage, less noise in the labelling view, and less private mail inside the system.

The backfill window on first connect is set per source (D-007, default 30 days).

**What should a source read by default?**

## Options
- A. Everything except Spam and Trash.
- B. Primary + Updates only (skip Promotions, Social, Forums, Spam).
- C. **Configurable per source with a Gmail search filter, default A** (recommended): every source gets an optional filter (for example `-category:social`, or `label:kids`), the default reads everything except Spam and Trash, and the category rules then ignore promotions.
- D. Only mail with a specific Gmail label you apply yourself (for example a filter that labels school senders).
- E. Something else (describe).

## Answer
<!-- answer below this line -->

