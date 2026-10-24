---
id: Q-001
title: Host machine & uptime
status: open
batch: 1
area: infra
created: 2026-10-23
resolved:
decisions: []
---

# Q-001: Host machine & uptime

## Question
Is the 32GB/RTX 3070 machine also the always-on server, or is it a gaming/workstation PC that sleeps? Would you add a small always-on box (e.g. a mini-PC or NAS) for polling and the API, and wake the GPU box only for LLM batches?

## Options
- A: the GPU box is always on and runs everything
- B: always-on low-power box + GPU box used on demand (LLM work queued)
- C: other (describe)

## Answer
<!-- answer below this line -->
I think I'd be fine with B. The RTX 3070 is in fact a gaming machine that is not currently running 24/7. So hosting the majority of the application on a separate Ubuntu Server host while waking or queueing LLM work is totally fine by me. It's probably best to think of the system as functional on it's own, enhanced by LLMs.
