---
id: Q-126
title: Long AI calls inside a 30 s request timeout
status: open
batch: 27
area: tech
created: 2026-11-23
resolved:
decisions: []
---

# Q-126: Long AI calls inside a 30 s request timeout

## Context
One 30 s request timeout covers inline LLM calls of 120 to 600 s (`fa-api/src/lib.rs:507`) (B-02). Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
How should long AI calls be handled?

## Options
- A. Move extraction to a job the UI polls, and exempt the AI routes from the timeout meanwhile (recommended)
- B. Exempt the AI routes only
- C. Configurable, default 30 s: a per-route timeout setting
- D. Leave as is; failures surface as timeouts

## Answer
<!-- answer below this line -->

