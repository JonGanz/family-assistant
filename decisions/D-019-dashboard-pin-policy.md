---
id: D-019
title: Dashboard PIN policy
status: accepted
date: 2026-10-23
source_questions: [Q-018]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-019: Dashboard PIN policy

## Context
The user picked option A.

## Decision
- Task check-offs and grocery additions on the dashboard need no PIN.
- Approvals (events) and Gmail actions need an adult's PIN, which is a 4–6 digit code stored hashed and rate-limited.
- Settings can't be changed on the dashboard.

## Consequences
- The API takes a `pin` step-up on the device session for privileged actions.

## Alternatives considered
No PINs, or a PIN for everyone. Not chosen.
