---
id: Q-032
title: What email metadata survives the content purge
status: open
batch: 6
area: security
created: 2026-10-23
resolved:
decisions: []
---

# Q-032: What email metadata survives the content purge

## Question
D-030 deletes email content once it has been processed. To keep a usable email list, 'Fix category', provenance titles, and kNN, I propose keeping **sender address/name, subject, received date, and the embedding vector** (a 768-number vector; it is derived from the text, but the text can't easily be rebuilt from it). The body, snippet, and attachment text are deleted.
Also: how long can an **unprocessed** email keep its content while waiting for AI (e.g. the GPU node is off)?

## Options
- A: keep sender + subject + date + embedding; max pending 14 days (recommended)
- B: keep sender + date only (no subject, no embedding: kNN would re-fetch from Gmail)
- C: other (describe)

## Answer
<!-- answer below this line -->

