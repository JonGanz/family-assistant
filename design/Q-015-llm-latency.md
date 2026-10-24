---
id: Q-015
title: Acceptable LLM latency & batching
status: open
batch: 1
area: tech
created: 2026-10-23
resolved:
decisions: []
---

# Q-015: Acceptable LLM latency & batching

## Question
Can email processing run in batches (for example every 15 to 30 minutes, with a delay of a few minutes)? Or does it need to feel near-real-time? This decides whether a small 3–4B model is always loaded, or a 7–8B model runs in batches.

## Options
- A: batched is fine
- B: near-real-time required for some categories (which?)

## Answer
<!-- answer below this line -->

