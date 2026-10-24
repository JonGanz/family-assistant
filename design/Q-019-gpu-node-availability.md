---
id: Q-019
title: GPU node wake / schedule policy
status: open
batch: 2
area: infra
created: 2026-10-23
resolved:
decisions: []
---

# Q-019: GPU node wake / schedule policy

## Question
What OS is the gaming PC (Windows, Linux, dual boot)? Can the core node wake it with Wake-on-LAN? Should the system wake it automatically, for example when the queue is over N items or at set times like 3am? Should it avoid running the LLM while you're gaming (Ollama would compete for VRAM)?

## Options
- A: auto-wake on schedule/queue depth, yield when a game is running
- B: never auto-wake; only process when I happen to have it on
- C: other

## Answer
<!-- answer below this line -->

