---
id: Q-004
title: Datastore & backups
status: open
batch: 1
area: tech
created: 2026-10-23
resolved:
decisions: []
---

# Q-004: Datastore & backups

## Question
PostgreSQL+pgvector (more robust, one extra container) or SQLite+sqlite-vec (one file, simpler backups)? Where should backups go (local disk, NAS, encrypted cloud)?

## Options
- A: Postgres
- B: SQLite
- C: no preference — you pick

## Answer
<!-- answer below this line -->
A. Backups would end up on a NAS.
