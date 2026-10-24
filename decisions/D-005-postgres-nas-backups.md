---
id: D-005
title: PostgreSQL 16 + pgvector; nightly backups to NAS
status: accepted
date: 2026-10-23
source_questions: [Q-004]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-005: PostgreSQL 16 + pgvector; nightly backups to NAS

## Context
The user picked Postgres, with backups going to a NAS.

## Decision
- PostgreSQL 16 with `pgvector` and the built-in FTS runs on the core node.
- A nightly `pg_dump` (custom format) is written to a NAS mount, with retention rotation of 7 daily, 4 weekly, and 6 monthly. Attachments and blob storage go to the same target.
- Postgres also holds the job queue (`FOR UPDATE SKIP LOCKED`), so there's no separate broker.

## Consequences
- Restore must be tested, so a restore step is part of the runbook.
- Embeddings are created on the GPU node and stored here.

## Alternatives considered
SQLite. Rejected by the user.
