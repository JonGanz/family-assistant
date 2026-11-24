---
id: Q-128
title: Events deleted at the provider
status: open
batch: 27
area: domain
created: 2026-11-23
resolved:
decisions: []
---

# Q-128: Events deleted at the provider

## Context
Events the app wrote and the user later deleted at the provider stay in the family calendar (`fa-store/src/repo/event.rs:137`) (B-06). Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
What happens to a synced copy when it disappears at the provider?

## Options
- A. Delete synced copies; keep mail-proposed ones and mark them "removed at provider" (recommended)
- B. Configurable, default A: delete, mark, or keep
- C. Always keep, marked

## Answer
<!-- answer below this line -->

