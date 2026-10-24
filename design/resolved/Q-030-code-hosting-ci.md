---
id: Q-030
title: Code repository & CI
status: resolved
batch: 5
area: tech
created: 2026-10-23
resolved: 2026-10-23
decisions: [D-032]
---

# Q-030: Code repository & CI

## Question
This repo currently holds the design docs. Where should the **code** live, and what runs CI?

## Options
- A: same repo (design docs in /docs or as-is alongside crates/), GitHub + GitHub Actions
- B: same repo, self-hosted Gitea/Forgejo + its CI
- C: separate code repo (say where)

## Answer
<!-- answer below this line -->
A
