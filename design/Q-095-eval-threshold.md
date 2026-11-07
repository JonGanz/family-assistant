---
id: Q-095
title: Which threshold the eval uses when the llm stage is off
status: open
batch: 23
area: tech
created: 2026-11-04
resolved:
decisions: []
---

# Q-095: Which threshold the eval uses when the llm stage is off

## Question
`fa-server eval --with-llm` asks what the llm stage would decide. The stage may be disabled.

## Options
- A. Use the stored llm stage threshold even if disabled, else 0.7; a failed LLM call counts as undecided (applied)
- B. Refuse to run unless the stage is enabled
- C. A --threshold flag, default as A

## Proposed answer (UNVERIFIED: applied in M4 by Claude, 2026-11-05; confirm or change below)
A (T-09).

## Answer
<!-- answer below this line -->
Option A.
