---
id: D-061
title: Approval notifications batched plus digest, configurable per adult; proposals expire by kind
status: accepted
date: 2026-10-25
source_questions: [Q-058]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Frontend.md]
---

# D-061: Approval notifications batched plus digest, configurable per adult; proposals expire by kind

## Context
Proposals wait in the approval inbox. Open were notifications and expiry.

## Decision
- `approval_needed` is a per-adult notification trigger, default **batched** (D-044) and included in the daily digest; it can be set to immediate, digest only or off.
- Expiry (household settings): proposed events when their start passes; suggested tasks and contacts after 14 days; bills never.

## Consequences
`inbox.expire` runs hourly and records expired proposals as `expired` (not rejected), so they don't count as human negatives for training.

## Alternatives considered
- Immediate per proposal, nothing expires.
- Digest only.
