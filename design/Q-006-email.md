---
id: Q-006
title: Email accounts & processing scope
status: open
batch: 1
area: ingest
created: 2026-10-23
resolved:
decisions: []
---

# Q-006: Email accounts & processing scope

## Question
How many Gmail accounts are there, and whose are they? Should the system ever **write** to Gmail (labels, archive, mark read), or stay read-only? Should it process the full history or only mail from now on (or the last N days)? Are you OK creating a Google Cloud OAuth app in 'testing' mode? That needs a re-auth every 7 days unless you publish it as internal or verify it.

## Answer
<!-- answer below this line -->
There would be multiple accounts, which should be configurable as associating specifically to one or more users (like a shared account between adults). A config flag should lock to read-only mode (default with that on). Being able to archive, mark read, etc, would be useful features.

I am okay with a testing mode OAuth setup for now despite the 7 days limit.
