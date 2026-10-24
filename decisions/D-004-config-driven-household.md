---
id: D-004
title: Household data is configuration, never in the repo; multi-family capable
status: accepted
date: 2026-10-23
source_questions: [Q-003]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-004: Household data is configuration, never in the repo; multi-family capable

## Context
The user wants no private family information in the app's source, and wants the system to work for other families.

## Decision
- Members, schools, teachers, sources, and rules are all **runtime configuration** for each household. None of it is hard-coded.
- No PII and no secrets in git. Secrets (OAuth tokens, passwords) are stored encrypted in the DB. The key comes from the environment or a file outside the repo. Example and seed config use made-up families only.
- The schema has a `household_id` on every row, so one install could serve several households, although we deploy one per family.

## Consequences
- We need a setup or onboarding flow (Q-017 decides file or UI).
- Test fixtures use synthetic people and synthetic email.

## Alternatives considered
Hard-coding the family. Rejected.
