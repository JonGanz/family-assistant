---
id: Q-033
title: Repository visibility & license
status: open
batch: 6
area: tech
created: 2026-10-23
resolved:
decisions: []
---

# Q-033: Repository visibility & license

## Question
Will the GitHub repo and GHCR images be **public** (other families can self-host. This needs a license) or **private**? Public is fine privacy-wise, since no household data is ever in the repo (D-004). But note the design docs (decisions, logs) would be public too.

## Options
- A: public, MIT or Apache-2.0
- B: public, AGPL-3.0 (keeps hosted forks open)
- C: private for now

## Answer
<!-- answer below this line -->
Option B. I have been specific to avoid any personal information. Closest thing is my hardware.
