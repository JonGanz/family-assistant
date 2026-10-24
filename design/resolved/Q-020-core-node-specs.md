---
id: Q-020
title: Core node hardware & CPU fallback model
status: resolved
batch: 2
area: infra
created: 2026-10-23
resolved: 2026-10-23
decisions: [D-016]
---

# Q-020: Core node hardware & CPU fallback model

## Question
What are the specs of the Ubuntu server (CPU, RAM, disk)? If it has around 8 GB or more of spare RAM, a tiny CPU model (1–3B, for example Qwen2.5-1.5B) could handle urgent classification while the GPU node is off. This would only be for 'is this time-sensitive?' checks. Do you want that, or should the system stay rules-only until the GPU node is up?

## Options
- A: yes, add a small CPU model for urgent triage if specs allow
- B: no, rules-only until GPU node is available

## Answer
<!-- answer below this line -->
Option A. And allow that feature to be turned on or off.
