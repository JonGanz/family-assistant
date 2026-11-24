---
id: Q-124
title: Require the secret when an endpoint host changes
status: open
batch: 27
area: security
created: 2026-11-23
resolved:
decisions: []
---

# Q-124: Require the secret when an endpoint host changes

## Context
Testing an AI endpoint with a kept key and a new `base_url` sends the stored key to the new host (`fa-api/src/ai.rs`); a CalDAV patch does the same with the password (`fa-calendar/src/service.rs:506`) (S-04). IMAP already requires the secret again (C2-07). Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
Require the secret again when the host changes?

## Options
- A. Yes, for AI endpoints and CalDAV, as for IMAP (recommended)
- B. Only warn in the UI
- C. No; the admin is trusted

## Answer
<!-- answer below this line -->

